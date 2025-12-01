use crate::commands::types::{DataUpdateResult, SavedConfig};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const LOL_SKINS_MANIFEST_URL: &str = "https://abdelrhmanx74.github.io/osskins/manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LolSkinsManifestItem {
  path: String,
  url: String,
  size: u64,
  sha256: String,
  commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestAssetV2 {
  #[serde(default)]
  key: Option<String>,
  #[serde(default)]
  name: Option<String>,
  url: String,
  size: u64,
  sha256: String,
  commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestChampionV2 {
  key: String,
  #[serde(default)]
  name: Option<String>,
  #[serde(default)]
  id: Option<u32>,
  #[serde(default)]
  alias: Option<String>,
  #[serde(default)]
  fingerprint: Option<String>,
  #[serde(default)]
  size: Option<u64>,
  assets: ManifestChampionAssetsV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestChampionAssetsV2 {
  #[serde(default)]
  skins: Vec<ManifestAssetV2>,
  #[serde(default)]
  chromas: Vec<ManifestAssetV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LolSkinsManifest {
  schema: u32,
  generated_at: String,
  source_repo: String,
  branch: String,
  license: Option<String>,
  source: Option<String>,
  attribution: Option<String>,
  #[serde(default)]
  items: Vec<LolSkinsManifestItem>, // v1
  #[serde(default)]
  champions: Vec<ManifestChampionV2>, // v2
}

fn extract_manifest_commit(manifest: &LolSkinsManifest) -> Option<String> {
  let parts: Vec<&str> = manifest.source_repo.split('@').collect();
  if parts.len() == 2 && !parts[1].is_empty() {
    return Some(parts[1].to_string());
  }

  if let Some(item) = manifest.items.first() {
    if !item.commit.is_empty() {
      return Some(item.commit.clone());
    }
  }

  if let Some(champ) = manifest.champions.first() {
    if let Some(asset) = champ
      .assets
      .skins
      .first()
      .or_else(|| champ.assets.chromas.first())
    {
      if !asset.commit.is_empty() {
        return Some(asset.commit.clone());
      }
    }
  }

  None
}

async fn fetch_latest_manifest() -> Result<LolSkinsManifest, String> {
  // Use a tuned HTTP client so we fail fast on poor networks and let the UI fallback
  let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(5))
    .timeout(Duration::from_secs(10))
    .pool_idle_timeout(Duration::from_secs(30))
    .build()
    .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

  let resp = client
    .get(LOL_SKINS_MANIFEST_URL)
    .header("User-Agent", "osskins-tauri")
    .header("Accept", "application/json")
    .header("Cache-Control", "no-cache")
    .send()
    .await
    .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

  if !resp.status().is_success() {
    let status = resp.status();
    let body = resp
      .text()
      .await
      .unwrap_or_else(|_| "<unable to read body>".to_string());
    return Err(format!(
      "Manifest request returned status {} - body: {}",
      status, body
    ));
  }

  resp
    .json::<LolSkinsManifest>()
    .await
    .map_err(|e| format!("Invalid manifest JSON: {}", e))
}

fn manifest_storage_dir(app: &AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
  Ok(app_data_dir.join("manifest"))
}

fn load_cached_manifest(app: &AppHandle) -> Result<Option<LolSkinsManifest>, String> {
  let dir = manifest_storage_dir(app)?;
  let file_path = dir.join("latest.json");
  if !file_path.exists() {
    return Ok(None);
  }

  let content =
    fs::read_to_string(&file_path).map_err(|e| format!("Failed to read cached manifest: {}", e))?;
  let manifest = serde_json::from_str::<LolSkinsManifest>(&content)
    .map_err(|e| format!("Failed to parse cached manifest: {}", e))?;
  Ok(Some(manifest))
}

fn save_manifest_snapshot(app: &AppHandle, manifest: &LolSkinsManifest) -> Result<(), String> {
  let dir = manifest_storage_dir(app)?;
  fs::create_dir_all(&dir).map_err(|e| format!("Failed to create manifest directory: {}", e))?;
  let file_path = dir.join("latest.json");
  let data = serde_json::to_string_pretty(manifest)
    .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
  fs::write(&file_path, data).map_err(|e| format!("Failed to persist manifest snapshot: {}", e))?;
  Ok(())
}

fn champion_from_path(path: &str) -> Option<String> {
  let mut segments = path.split('/');
  let prefix = segments.next()?;
  if !prefix.eq_ignore_ascii_case("skins") {
    return None;
  }
  let champion_segment = segments.next()?;
  let decoded = percent_decode_str(champion_segment).decode_utf8().ok()?;
  if decoded.is_empty() {
    None
  } else {
    Some(decoded.to_string())
  }
}

fn diff_manifests(previous: &LolSkinsManifest, current: &LolSkinsManifest) -> Vec<String> {
  let mut changes: HashSet<String> = HashSet::new();
  // Prefer v2 champion-based diff if both provide champions
  if !previous.champions.is_empty() && !current.champions.is_empty() {
    let prev_map: HashMap<&str, &ManifestChampionV2> = previous
      .champions
      .iter()
      .map(|c| (c.key.as_str(), c))
      .collect();
    let curr_map: HashMap<&str, &ManifestChampionV2> = current
      .champions
      .iter()
      .map(|c| (c.key.as_str(), c))
      .collect();

    for (key, curr) in &curr_map {
      let changed = match prev_map.get(key) {
        Some(prev) => prev.fingerprint != curr.fingerprint || prev.size != curr.size,
        None => true,
      };
      if changed {
        let name = curr.name.clone().unwrap_or_else(|| (*key).to_string());
        changes.insert(name);
      }
    }
    for key in prev_map.keys() {
      if !curr_map.contains_key(key) {
        changes.insert((*key).to_string());
      }
    }
  } else {
    // v1 path: compare items by path and sha
    let prev_map: HashMap<&str, &LolSkinsManifestItem> = previous
      .items
      .iter()
      .map(|item| (item.path.as_str(), item))
      .collect();
    let curr_map: HashMap<&str, &LolSkinsManifestItem> = current
      .items
      .iter()
      .map(|item| (item.path.as_str(), item))
      .collect();

    for (path, current_item) in &curr_map {
      let needs_update = match prev_map.get(path) {
        Some(previous_item) => previous_item.sha256 != current_item.sha256,
        None => true,
      };
      if needs_update {
        if let Some(champion) = champion_from_path(path) {
          changes.insert(champion);
        }
      }
    }
    for path in prev_map.keys() {
      if !curr_map.contains_key(path) {
        if let Some(champion) = champion_from_path(path) {
          changes.insert(champion);
        }
      }
    }
  }

  let mut result: Vec<String> = changes.into_iter().collect();
  result.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
  result
}

async fn get_changed_champions_via_github(
  base_sha: &str,
  head_sha: &str,
) -> Result<Vec<String>, String> {
  let url = format!(
    "https://api.github.com/repos/darkseal-org/lol-skins/compare/{}...{}",
    base_sha, head_sha
  );
  println!("[DataUpdate] Comparing commits via: {}", url);
  // Apply reasonable timeouts to avoid hanging the UI
  let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(5))
    .timeout(Duration::from_secs(10))
    .pool_idle_timeout(Duration::from_secs(30))
    .build()
    .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
  let mut req = client.get(&url).header("User-Agent", "osskins-tauri");
  if let Ok(token) = std::env::var("GITHUB_TOKEN") {
    if !token.trim().is_empty() {
      req = req.header("Authorization", format!("token {}", token));
    }
  }

  let resp = req
    .send()
    .await
    .map_err(|e| format!("Failed to compare commits: {}", e))?;

  if !resp.status().is_success() {
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp
      .text()
      .await
      .unwrap_or_else(|_| "<unable to read body>".to_string());
    let rl_remaining = headers
      .get("x-ratelimit-remaining")
      .and_then(|v| v.to_str().ok())
      .unwrap_or("-");
    let rl_reset = headers
      .get("x-ratelimit-reset")
      .and_then(|v| v.to_str().ok())
      .unwrap_or("-");
    return Err(format!(
            "GitHub compare API returned status {} (x-ratelimit-remaining={}, x-ratelimit-reset={}) - body: {}",
            status, rl_remaining, rl_reset, body
        ));
  }

  let json: serde_json::Value = resp
    .json()
    .await
    .map_err(|e| format!("Invalid JSON: {}", e))?;
  let mut champs: HashSet<String> = HashSet::new();

  if let Some(files) = json.get("files").and_then(|v| v.as_array()) {
    println!(
      "[DataUpdate] Compare returned {} changed files",
      files.len()
    );
    for f in files {
      if let Some(filename) = f.get("filename").and_then(|v| v.as_str()) {
        // We care about paths like skins/<Champion>/...
        if let Some(rest) = filename.strip_prefix("skins/") {
          if let Some((champ, _)) = rest.split_once('/') {
            // Decode percent-encodings just in case
            let decoded = percent_encoding::percent_decode_str(champ)
              .decode_utf8_lossy()
              .to_string();
            champs.insert(decoded);
          }
        }
      }
    }
  }

  let mut list: Vec<String> = champs.into_iter().collect();
  list.sort();
  let preview = list.iter().take(20).cloned().collect::<Vec<_>>().join(", ");
  println!(
    "[DataUpdate] Changed champions ({}): {}{}",
    list.len(),
    preview,
    if list.len() > 20 { " ..." } else { "" }
  );

  Ok(list)
}

// Champion data management commands

#[tauri::command]
pub async fn check_data_updates(app: tauri::AppHandle) -> Result<DataUpdateResult, String> {
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

  // Try to load last saved commit from config
  let config = super::config::load_config(app.clone())
    .await
    .unwrap_or(SavedConfig {
      league_path: None,
      skins: vec![],
      custom_skins: vec![],
      favorites: vec![],
      theme: None,
      party_mode: super::types::PartyModeConfig::default(),
      selected_misc_items: std::collections::HashMap::new(),
      auto_update_data: true,
      last_data_commit: None,
    });

  let manifest = match fetch_latest_manifest().await {
    Ok(manifest) => manifest,
    Err(err) => {
      println!(
        "[DataUpdate] Failed to fetch manifest: {}. Falling back to default update behavior.",
        err
      );
      return Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: vec!["repo".into()],
      });
    }
  };

  let latest_commit = extract_manifest_commit(&manifest);
  let stored_commit = config.last_data_commit.clone();

  println!(
    "[DataUpdate] Last saved commit: {:?} | Latest manifest commit: {:?}",
    stored_commit, latest_commit
  );

  let needs_update = match (&stored_commit, &latest_commit) {
    (Some(stored), Some(latest)) if stored == latest => false,
    _ => true,
  };

  if !needs_update {
    println!("[DataUpdate] Data up-to-date. No update required.");
    return Ok(DataUpdateResult {
      success: true,
      error: None,
      updated_champions: Vec::new(),
    });
  }

  let mut updated_champions: Vec<String> = Vec::new();
  match load_cached_manifest(&app) {
    Ok(Some(previous_manifest)) => {
      let diff = diff_manifests(&previous_manifest, &manifest);
      if diff.is_empty() {
        println!(
          "[DataUpdate] Manifest diff empty despite commit change; defaulting to repo update"
        );
      } else {
        println!(
          "[DataUpdate] Manifest diff detected for {} champions",
          diff.len()
        );
        updated_champions = diff;
      }
    }
    Ok(None) => {
      println!("[DataUpdate] No cached manifest found; requiring full update");
      updated_champions = vec!["all".to_string()];
    }
    Err(err) => {
      println!(
        "[DataUpdate] Failed to load cached manifest: {}. Falling back to repo update",
        err
      );
    }
  }

  if updated_champions.is_empty() {
    updated_champions.push("repo".into());
  }

  Ok(DataUpdateResult {
    success: true,
    error: None,
    updated_champions,
  })
}

async fn fetch_latest_commit_sha() -> Result<String, String> {
  let manifest = fetch_latest_manifest().await?;
  if let Some(commit) = extract_manifest_commit(&manifest) {
    println!("[DataUpdate] Fetched manifest commit: {}", commit);
    Ok(commit)
  } else {
    Err("Manifest did not include commit information".into())
  }
}

#[tauri::command]
pub async fn set_last_data_commit(
  app: tauri::AppHandle,
  sha: String,
  manifest_json: Option<String>,
) -> Result<(), String> {
  // Load and update config.json
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
  println!("[DataUpdate] Saved last_data_commit to config: {}", sha);

  if let Some(manifest_str) = manifest_json {
    match serde_json::from_str::<LolSkinsManifest>(&manifest_str) {
      Ok(manifest) => {
        if let Some(manifest_commit) = extract_manifest_commit(&manifest) {
          if manifest_commit != sha {
            println!(
              "[DataUpdate] Warning: provided manifest commit ({}) does not match recorded commit ({})",
              manifest_commit, sha
            );
          }
        }
        if let Err(err) = save_manifest_snapshot(&app, &manifest) {
          println!(
            "[DataUpdate] Failed to persist manifest snapshot from frontend: {}",
            err
          );
        }
      }
      Err(err) => {
        println!(
          "[DataUpdate] Invalid manifest JSON supplied while saving commit {}: {}",
          sha, err
        );
      }
    }
  } else if !sha.is_empty() {
    match fetch_latest_manifest().await {
      Ok(manifest) => {
        if extract_manifest_commit(&manifest).as_deref() == Some(sha.as_str()) {
          if let Err(err) = save_manifest_snapshot(&app, &manifest) {
            println!(
              "[DataUpdate] Failed to persist manifest snapshot fetched server-side: {}",
              err
            );
          }
        } else {
          println!(
            "[DataUpdate] Skipped manifest snapshot fetch because fetched commit differed ({} != {:?})",
            sha,
            extract_manifest_commit(&manifest)
          );
        }
      }
      Err(err) => {
        println!(
          "[DataUpdate] Failed to fetch manifest for snapshot storage: {}",
          err
        );
      }
    }
  }

  Ok(())
}

#[tauri::command]
pub async fn get_latest_data_commit() -> Result<String, String> {
  fetch_latest_commit_sha().await
}

#[tauri::command]
pub async fn get_changed_champions_since(last_sha: String) -> Result<Vec<String>, String> {
  match fetch_latest_commit_sha().await {
    Ok(head_sha) => get_changed_champions_via_github(last_sha.as_str(), &head_sha).await,
    Err(err) => {
      println!(
        "[DataUpdate] Failed to resolve latest commit via manifest ({}); falling back to main",
        err
      );
      get_changed_champions_via_github(last_sha.as_str(), "main").await
    }
  }
}

#[tauri::command]
pub async fn get_changed_champions_from_config(
  app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
  let cfg = super::config::load_config(app.clone()).await?;
  let Some(last_commit) = cfg.last_data_commit else {
    return Ok(Vec::new());
  };

  let latest_manifest = match fetch_latest_manifest().await {
    Ok(manifest) => manifest,
    Err(err) => {
      println!(
        "[DataUpdate] Failed to fetch manifest while resolving changed champions: {}",
        err
      );
      return Ok(Vec::new());
    }
  };

  let Some(latest_commit) = extract_manifest_commit(&latest_manifest) else {
    println!("[DataUpdate] Manifest missing commit; skipping changed champion detection");
    return Ok(Vec::new());
  };

  if last_commit == latest_commit {
    return Ok(Vec::new());
  }

  if let Ok(Some(previous_manifest)) = load_cached_manifest(&app) {
    let diff = diff_manifests(&previous_manifest, &latest_manifest);
    if !diff.is_empty() {
      return Ok(diff);
    }
  }

  // Fall back to GitHub compare when local diffing is unavailable
  get_changed_champions_via_github(last_commit.as_str(), &latest_commit).await
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
