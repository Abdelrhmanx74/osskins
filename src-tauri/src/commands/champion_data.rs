use crate::commands::types::{DataUpdateResult, SavedConfig};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

// LeagueSkins repo configuration
const LEAGUE_SKINS_REPO: &str = "Alban1911/LeagueSkins";
const LEAGUE_SKINS_RAW_BASE: &str =
  "https://raw.githubusercontent.com/Alban1911/LeagueSkins/main";
const GITHUB_API_BASE: &str = "https://api.github.com";

// Data update progress event
const DATA_UPDATE_EVENT: &str = "data-update-progress";

/// Lightweight manifest structure for tracking local state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalManifest {
  pub version: u32,
  pub last_commit: Option<String>,
  pub last_updated: Option<String>,
  pub champions: HashMap<u32, ChampionManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionManifestEntry {
  pub id: u32,
  pub name: String,
  pub skins: HashMap<u32, SkinManifestEntry>,
  pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinManifestEntry {
  pub id: u32,
  pub name: String,
  pub has_chromas: bool,
  pub chroma_ids: Vec<u32>,
  pub has_forms: bool,
  pub form_ids: Vec<u32>,
  pub file_size: Option<u64>,
  pub downloaded: bool,
}

/// Progress payload for data updates
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataUpdateProgressPayload {
  pub phase: String, // "checking" | "fetching" | "comparing" | "downloading" | "complete" | "error"
  pub message: String,
  pub current_champion: Option<String>,
  pub total_champions: Option<usize>,
  pub processed_champions: Option<usize>,
  pub total_skins: Option<usize>,
  pub processed_skins: Option<usize>,
  pub bytes_downloaded: Option<u64>,
  pub total_bytes: Option<u64>,
  pub speed: Option<f64>,
  pub changed_champions: Option<Vec<String>>,
  pub new_skins_count: Option<usize>,
  pub updated_skins_count: Option<usize>,
}

fn emit_update_progress(app: &AppHandle, payload: DataUpdateProgressPayload) {
  let _ = app.emit(DATA_UPDATE_EVENT, payload);
}

/// GitHub commit info
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubCommit {
  sha: String,
  commit: GitHubCommitDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubCommitDetails {
  message: String,
  committer: GitHubCommitter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubCommitter {
  date: String,
}

/// GitHub compare result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubCompareResult {
  ahead_by: u32,
  behind_by: u32,
  total_commits: u32,
  files: Option<Vec<GitHubChangedFile>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubChangedFile {
  filename: String,
  status: String, // "added", "modified", "removed"
  additions: u32,
  deletions: u32,
}

/// Build HTTP client for API requests
fn build_http_client() -> Result<reqwest::Client, String> {
  reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(10))
    .timeout(Duration::from_secs(30))
    .pool_idle_timeout(Duration::from_secs(60))
    .pool_max_idle_per_host(10)
    .user_agent("osskins-tauri/3.0")
    .build()
    .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

fn manifest_storage_dir(app: &AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
  Ok(app_data_dir.join("manifest"))
}

/// Load local manifest from disk
fn load_local_manifest(app: &AppHandle) -> Result<LocalManifest, String> {
  let dir = manifest_storage_dir(app)?;
  let file_path = dir.join("local_manifest.json");
  if !file_path.exists() {
    return Ok(LocalManifest::default());
  }

  let content =
    fs::read_to_string(&file_path).map_err(|e| format!("Failed to read local manifest: {}", e))?;
  serde_json::from_str::<LocalManifest>(&content)
    .map_err(|e| format!("Failed to parse local manifest: {}", e))
}

/// Save local manifest to disk
fn save_local_manifest(app: &AppHandle, manifest: &LocalManifest) -> Result<(), String> {
  let dir = manifest_storage_dir(app)?;
  fs::create_dir_all(&dir).map_err(|e| format!("Failed to create manifest directory: {}", e))?;
  let file_path = dir.join("local_manifest.json");
  let data = serde_json::to_string_pretty(manifest)
    .map_err(|e| format!("Failed to serialize local manifest: {}", e))?;
  fs::write(&file_path, data).map_err(|e| format!("Failed to persist local manifest: {}", e))?;
  Ok(())
}

/// Fetch the latest commit SHA from LeagueSkins repo
async fn fetch_latest_commit_sha() -> Result<String, String> {
  let client = build_http_client()?;
  let url = format!(
    "{}/repos/{}/commits/main",
    GITHUB_API_BASE, LEAGUE_SKINS_REPO
  );

  let mut req = client.get(&url).header("Accept", "application/vnd.github.v3+json");

  // Add GitHub token if available for higher rate limits
  if let Ok(token) = std::env::var("GITHUB_TOKEN") {
    if !token.trim().is_empty() {
      req = req.header("Authorization", format!("token {}", token));
    }
  }

  let resp = req.send().await.map_err(|e| format!("Failed to fetch commit: {}", e))?;

  if !resp.status().is_success() {
    return Err(format!("GitHub API returned status {}", resp.status()));
  }

  let commit: GitHubCommit = resp
    .json()
    .await
    .map_err(|e| format!("Failed to parse commit response: {}", e))?;

  Ok(commit.sha)
}

/// Extract champion ID from a file path in the repo
/// e.g., "skins/1/1000/1000.zip" -> Some(1)
fn extract_champion_id_from_path(path: &str) -> Option<u32> {
  let segments: Vec<&str> = path.split('/').collect();
  if segments.len() >= 2 && segments[0] == "skins" {
    segments[1].parse().ok()
  } else {
    None
  }
}

/// Extract skin ID from a file path
/// e.g., "skins/1/1000/1000.zip" -> Some(1000)
fn extract_skin_id_from_path(path: &str) -> Option<u32> {
  let segments: Vec<&str> = path.split('/').collect();
  if segments.len() >= 3 && segments[0] == "skins" {
    segments[2].parse().ok()
  } else {
    None
  }
}

/// Compare commits and get changed files
async fn get_changed_files_between_commits(
  base_sha: &str,
  head_sha: &str,
) -> Result<Vec<GitHubChangedFile>, String> {
  let client = build_http_client()?;
  let url = format!(
    "{}/repos/{}/compare/{}...{}",
    GITHUB_API_BASE, LEAGUE_SKINS_REPO, base_sha, head_sha
  );

  println!("[DataUpdate] Comparing commits: {} -> {}", base_sha, head_sha);

  let mut req = client.get(&url).header("Accept", "application/vnd.github.v3+json");

  if let Ok(token) = std::env::var("GITHUB_TOKEN") {
    if !token.trim().is_empty() {
      req = req.header("Authorization", format!("token {}", token));
    }
  }

  let resp = req.send().await.map_err(|e| format!("Failed to compare commits: {}", e))?;

  if !resp.status().is_success() {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    return Err(format!("GitHub compare API returned {} - {}", status, body));
  }

  let compare: GitHubCompareResult = resp
    .json()
    .await
    .map_err(|e| format!("Failed to parse compare response: {}", e))?;

  Ok(compare.files.unwrap_or_default())
}

/// Get list of champion IDs that have changed between commits
async fn get_changed_champion_ids(base_sha: &str, head_sha: &str) -> Result<HashSet<u32>, String> {
  let files = get_changed_files_between_commits(base_sha, head_sha).await?;
  let mut champion_ids = HashSet::new();

  for file in files {
    if file.filename.starts_with("skins/") && file.filename.ends_with(".zip") {
      if let Some(champ_id) = extract_champion_id_from_path(&file.filename) {
        champion_ids.insert(champ_id);
      }
    }
  }

  println!(
    "[DataUpdate] Found {} changed champions",
    champion_ids.len()
  );
  Ok(champion_ids)
}

// Champion data management commands

#[tauri::command]
pub async fn check_data_updates(app: tauri::AppHandle) -> Result<DataUpdateResult, String> {
  emit_update_progress(
    &app,
    DataUpdateProgressPayload {
      phase: "checking".into(),
      message: "Checking for updates...".into(),
      current_champion: None,
      total_champions: None,
      processed_champions: None,
      total_skins: None,
      processed_skins: None,
      bytes_downloaded: None,
      total_bytes: None,
      speed: None,
      changed_champions: None,
      new_skins_count: None,
      updated_skins_count: None,
    },
  );

  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  let champions_dir = app_data_dir.join("champions");
  if !champions_dir.exists() {
    println!("[DataUpdate] No champions directory found -> initial download needed");
    return Ok(DataUpdateResult {
      success: true,
      error: None,
      updated_champions: vec!["all".to_string()],
    });
  }

  // Load local manifest to check last commit
  let local_manifest = load_local_manifest(&app).unwrap_or_default();
  let stored_commit = local_manifest.last_commit.clone();

  // Fetch latest commit from repo
  let latest_commit = match fetch_latest_commit_sha().await {
    Ok(sha) => sha,
    Err(err) => {
      println!(
        "[DataUpdate] Failed to fetch latest commit: {}. Assuming update needed.",
        err
      );
      return Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: vec!["repo".into()],
      });
    }
  };

  println!(
    "[DataUpdate] Last saved commit: {:?} | Latest commit: {}",
    stored_commit, latest_commit
  );

  // Check if update is needed
  let needs_update = match &stored_commit {
    Some(stored) if stored == &latest_commit => false,
    _ => true,
  };

  if !needs_update {
    println!("[DataUpdate] Data up-to-date. No update required.");
    emit_update_progress(
      &app,
      DataUpdateProgressPayload {
        phase: "complete".into(),
        message: "Already up to date".into(),
        current_champion: None,
        total_champions: None,
        processed_champions: None,
        total_skins: None,
        processed_skins: None,
        bytes_downloaded: None,
        total_bytes: None,
        speed: None,
        changed_champions: Some(vec![]),
        new_skins_count: Some(0),
        updated_skins_count: Some(0),
      },
    );
    return Ok(DataUpdateResult {
      success: true,
      error: None,
      updated_champions: Vec::new(),
    });
  }

  // Get changed champion IDs if we have a previous commit
  let mut updated_champions: Vec<String> = Vec::new();

  if let Some(ref base_commit) = stored_commit {
    emit_update_progress(
      &app,
      DataUpdateProgressPayload {
        phase: "comparing".into(),
        message: "Comparing changes...".into(),
        current_champion: None,
        total_champions: None,
        processed_champions: None,
        total_skins: None,
        processed_skins: None,
        bytes_downloaded: None,
        total_bytes: None,
        speed: None,
        changed_champions: None,
        new_skins_count: None,
        updated_skins_count: None,
      },
    );

    match get_changed_champion_ids(base_commit, &latest_commit).await {
      Ok(changed_ids) => {
        if changed_ids.is_empty() {
          println!("[DataUpdate] No champion changes detected");
        } else {
          // Convert IDs to strings for the result
          updated_champions = changed_ids.iter().map(|id| id.to_string()).collect();
          println!(
            "[DataUpdate] Found {} changed champions: {:?}",
            updated_champions.len(),
            updated_champions.iter().take(10).collect::<Vec<_>>()
          );
        }
      }
      Err(err) => {
        println!("[DataUpdate] Failed to get changed champions: {}. Defaulting to full update.", err);
        updated_champions = vec!["repo".into()];
      }
    }
  } else {
    // No previous commit, need full update
    updated_champions = vec!["all".into()];
  }

  if updated_champions.is_empty() {
    updated_champions.push("repo".into());
  }

  emit_update_progress(
    &app,
    DataUpdateProgressPayload {
      phase: "complete".into(),
      message: format!("Found {} champions to update", updated_champions.len()),
      current_champion: None,
      total_champions: Some(updated_champions.len()),
      processed_champions: None,
      total_skins: None,
      processed_skins: None,
      bytes_downloaded: None,
      total_bytes: None,
      speed: None,
      changed_champions: Some(updated_champions.clone()),
      new_skins_count: None,
      updated_skins_count: None,
    },
  );

  Ok(DataUpdateResult {
    success: true,
    error: None,
    updated_champions,
  })
}

#[tauri::command]
pub async fn set_last_data_commit(
  app: tauri::AppHandle,
  sha: String,
  _manifest_json: Option<String>,
) -> Result<(), String> {
  // Update local manifest with new commit
  let mut local_manifest = load_local_manifest(&app).unwrap_or_default();
  local_manifest.last_commit = Some(sha.clone());
  local_manifest.last_updated = Some(chrono::Utc::now().to_rfc3339());
  save_local_manifest(&app, &local_manifest)?;

  // Also update config.json for backward compatibility
  let config_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data dir: {}", e))?
    .join("config");
  std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
  let file = config_dir.join("config.json");

  let mut cfg: serde_json::Value = if file.exists() {
    let content = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())?
  } else {
    serde_json::json!({"auto_update_data": true})
  };

  cfg["last_data_commit"] = serde_json::json!(sha.clone());

  let data = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
  std::fs::write(&file, data).map_err(|e| e.to_string())?;
  println!("[DataUpdate] Saved last_data_commit: {}", sha);

  Ok(())
}

#[tauri::command]
pub async fn get_latest_data_commit() -> Result<String, String> {
  fetch_latest_commit_sha().await
}

#[tauri::command]
pub async fn get_changed_champions_since(last_sha: String) -> Result<Vec<String>, String> {
  let latest_sha = fetch_latest_commit_sha().await?;
  let changed_ids = get_changed_champion_ids(&last_sha, &latest_sha).await?;
  Ok(changed_ids.iter().map(|id| id.to_string()).collect())
}

#[tauri::command]
pub async fn get_changed_champions_from_config(
  app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
  let local_manifest = load_local_manifest(&app).unwrap_or_default();
  let Some(last_commit) = local_manifest.last_commit else {
    return Ok(Vec::new());
  };

  let latest_commit = match fetch_latest_commit_sha().await {
    Ok(sha) => sha,
    Err(err) => {
      println!(
        "[DataUpdate] Failed to fetch latest commit: {}",
        err
      );
      return Ok(Vec::new());
    }
  };

  if last_commit == latest_commit {
    return Ok(Vec::new());
  }

  let changed_ids = get_changed_champion_ids(&last_commit, &latest_commit).await?;
  Ok(changed_ids.iter().map(|id| id.to_string()).collect())
}

#[tauri::command]
pub async fn update_champion_data(
  app: tauri::AppHandle,
  champion_name: String,
  data: String,
) -> Result<(), String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  let champion_dir = app_data_dir.join("champions").join(&champion_name);
  fs::create_dir_all(&champion_dir)
    .map_err(|e| format!("Failed to create champion directory: {}", e))?;

  let champion_file = champion_dir.join(format!("{}.json", champion_name));
  fs::write(champion_file, data).map_err(|e| format!("Failed to write champion data: {}", e))?;

  Ok(())
}

#[tauri::command]
pub async fn save_skin_file(
  app: tauri::AppHandle,
  champion_name: String,
  skin_name: String,
  is_chroma: bool,
  chroma_id: Option<u32>,
  content: Vec<u8>,
) -> Result<(), String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  // Create champions directory if it doesn't exist
  let champions_dir = app_data_dir.join("champions");
  fs::create_dir_all(&champions_dir)
    .map_err(|e| format!("Failed to create champions directory: {}", e))?;

  // Create champion directory if it doesn't exist
  let champion_dir = champions_dir.join(&champion_name);
  fs::create_dir_all(&champion_dir)
    .map_err(|e| format!("Failed to create champion directory: {}", e))?;

  let skin_file = if is_chroma {
    champion_dir.join(format!(
      "{}_chroma_{}.skin_file",
      skin_name,
      chroma_id.unwrap_or(0)
    ))
  } else {
    champion_dir.join(format!("{}.skin_file", skin_name))
  };

  // Ensure parent directory exists
  if let Some(parent) = skin_file.parent() {
    fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {}", e))?;
  }

  fs::write(&skin_file, content).map_err(|e| format!("Failed to write skin_file file: {}", e))?;

  Ok(())
}

#[tauri::command]
pub async fn get_champion_data(app: tauri::AppHandle, champion_id: u32) -> Result<String, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  let champions_dir = app_data_dir.join("champions");
  if !champions_dir.exists() {
    return Ok("[]".to_string()); // Return empty array if no champions directory exists
  }

  // If champion_id is 0, return all champions
  if champion_id == 0 {
    let mut all_champions = Vec::new();
    for entry in fs::read_dir(champions_dir)
      .map_err(|e| format!("Failed to read champions directory: {}", e))?
    {
      let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
      let path = entry.path();
      if path.is_dir() {
        // Get the champion directory name
        let champion_name = path
          .file_name()
          .and_then(|n| n.to_str())
          .ok_or_else(|| format!("Invalid champion directory name"))?;

        // Read only the specific champion JSON file: {champion_name}/{champion_name}.json
        let champion_file = path.join(format!("{}.json", champion_name));
        if champion_file.exists() {
          let data = fs::read_to_string(&champion_file)
            .map_err(|e| format!("Failed to read champion file: {}", e))?;
          all_champions.push(data);
        }
      }
    }
    return Ok(format!("[{}]", all_champions.join(",")));
  }

  // Otherwise, return data for specific champion
  // We need to search through all champion directories to find the one with matching ID
  for entry in
    fs::read_dir(champions_dir).map_err(|e| format!("Failed to read champions directory: {}", e))?
  {
    let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
    let path = entry.path();
    if path.is_dir() {
      let champion_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("Invalid champion directory name"))?;
      let champion_file = path.join(format!("{}.json", champion_name));
      if champion_file.exists() {
        return fs::read_to_string(champion_file)
          .map_err(|e| format!("Failed to read champion data: {}", e));
      }
    }
  }

  Err(format!("Champion data not found for ID: {}", champion_id))
}

#[tauri::command]
pub async fn check_champions_data(app: tauri::AppHandle) -> Result<bool, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  let champions_dir = app_data_dir.join("champions");
  if !champions_dir.exists() {
    return Ok(false);
  }

  // Check if there are any champion directories with JSON files
  let has_data = fs::read_dir(champions_dir)
    .map_err(|e| format!("Failed to read champions directory: {}", e))?
    .filter_map(|entry| entry.ok())
    .filter(|entry| entry.path().is_dir())
    .any(|champion_dir| {
      fs::read_dir(champion_dir.path())
        .ok()
        .map_or(false, |mut entries| {
          entries.any(|entry| {
            entry.map_or(false, |e| {
              e.path().extension().and_then(|s| s.to_str()) == Some("json")
            })
          })
        })
    });

  Ok(has_data)
}

#[tauri::command]
pub async fn delete_champions_cache(app: tauri::AppHandle) -> Result<(), String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  let champions_dir = app_data_dir.join("champions");

  // If the directory exists, remove it and all its contents
  if champions_dir.exists() {
    fs::remove_dir_all(&champions_dir)
      .map_err(|e| format!("Failed to delete champions cache: {}", e))?;
  }

  Ok(())
}

// Helper functions
pub async fn get_champion_name(app: &tauri::AppHandle, champion_id: u32) -> Result<String, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data dir: {}", e))?;

  let champions_dir = app_data_dir.join("champions");

  // Look through champion directories to find the one with matching ID
  if champions_dir.exists() {
    for entry in std::fs::read_dir(champions_dir).map_err(|e| e.to_string())? {
      if let Ok(entry) = entry {
        let path = entry.path();
        if path.is_dir() {
          let json_file = path.join(format!("{}.json", entry.file_name().to_string_lossy()));

          if json_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&json_file) {
              if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(id) = data.get("id").and_then(|v| v.as_u64()) {
                  if id as u32 == champion_id {
                    if let Some(_name) = data.get("name").and_then(|v| v.as_str()) {
                      // Use champion directory name instead of display name for consistency
                      return Ok(entry.file_name().to_string_lossy().to_string());
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }

  // Fallback
  Ok(format!("champion_{}", champion_id))
}

// Helper function to get champion ID from name
#[allow(dead_code)]
pub fn get_champion_id_by_name(app: &AppHandle, champion_name: &str) -> Option<u32> {
  let app_data_dir = match app.path().app_data_dir() {
    Ok(dir) => dir,
    Err(_) => return None,
  };

  let champions_dir = app_data_dir.join("champions");
  if !champions_dir.exists() {
    return None;
  }

  // Normalize the champion name for comparison
  let normalized_name = champion_name
    .to_lowercase()
    .replace(" ", "")
    .replace("'", "");

  // Search through champion JSON files
  if let Ok(entries) = fs::read_dir(champions_dir) {
    for entry in entries.filter_map(Result::ok) {
      if entry.path().is_dir() {
        let champ_dir_name = entry.file_name().to_string_lossy().to_lowercase();

        // Check if directory name matches
        if champ_dir_name == normalized_name {
          // Found a potential match, check the JSON file
          let json_path = entry.path().join(format!("{}.json", champ_dir_name));

          if let Ok(content) = fs::read_to_string(json_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
              // Extract champion ID from JSON
              return data.get("id").and_then(|v| v.as_u64()).map(|id| id as u32);
            }
          }
        }
      }
    }
  }

  None
}
