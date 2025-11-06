use crate::commands::types::{DataUpdateResult, SavedConfig};
use std::fs;
use tauri::{AppHandle, Manager};

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

  let last_saved = config.last_data_commit.clone();

  // Fetch latest commit sha from GitHub for darkseal-org/lol-skins main branch
  let latest_sha = match fetch_latest_commit_sha().await {
    Ok(sha) => Some(sha),
    Err(e) => {
      println!("[DataUpdate] Failed to fetch latest commit: {}", e);
      None
    }
  };

  if let Some(latest) = latest_sha {
    println!(
      "[DataUpdate] Last saved commit: {:?} | Latest upstream commit: {}",
      last_saved, latest
    );
    if config.last_data_commit.as_deref() == Some(&latest) {
      println!("[DataUpdate] Data up-to-date. No update required.");
      return Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: Vec::new(),
      });
    } else {
      println!("[DataUpdate] Update required (commit changed). Proceeding.");
      return Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: vec!["repo".into()],
      });
    }
  }

  // Fallback to no updates when API fails
  println!("[DataUpdate] GitHub API unavailable. Skipping update check.");
  Ok(DataUpdateResult {
    success: true,
    error: None,
    updated_champions: Vec::new(),
  })
}

async fn fetch_latest_commit_sha() -> Result<String, String> {
  let url = "https://api.github.com/repos/darkseal-org/lol-skins/commits?per_page=1";
  let client = reqwest::Client::new();
  let mut req = client.get(url).header("User-Agent", "osskins-tauri");

  // Prefer token from environment to avoid being rate-limited by unauthenticated requests
  if let Ok(token) = std::env::var("GITHUB_TOKEN") {
    if !token.trim().is_empty() {
      req = req.header("Authorization", format!("token {}", token));
    }
  }

  let resp = req
    .send()
    .await
    .map_err(|e| format!("Failed to fetch commits: {}", e))?;

  if !resp.status().is_success() {
    // try to capture some diagnostic info from the response body and rate-limit headers
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
      "GitHub API returned status {} (x-ratelimit-remaining={}, x-ratelimit-reset={}) - body: {}",
      status, rl_remaining, rl_reset, body
    ));
  }

  let json: serde_json::Value = resp
    .json()
    .await
    .map_err(|e| format!("Invalid JSON: {}", e))?;
  if let Some(first) = json.as_array().and_then(|arr| arr.first()) {
    if let Some(sha) = first.get("sha").and_then(|v| v.as_str()) {
      println!("[DataUpdate] Fetched latest commit SHA: {}", sha);
      return Ok(sha.to_string());
    }
  }
  Err("No commit found".into())
}

#[tauri::command]
pub async fn set_last_data_commit(app: tauri::AppHandle, sha: String) -> Result<(), String> {
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
  Ok(())
}

#[tauri::command]
pub async fn get_latest_data_commit() -> Result<String, String> {
  fetch_latest_commit_sha().await
}

#[tauri::command]
pub async fn get_changed_champions_since(last_sha: String) -> Result<Vec<String>, String> {
  let url = format!(
    "https://api.github.com/repos/darkseal-org/lol-skins/compare/{}...main",
    last_sha
  );
  println!("[DataUpdate] Comparing commits via: {}", url);
  let client = reqwest::Client::new();
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
  let mut champs: std::collections::HashSet<String> = std::collections::HashSet::new();

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

#[tauri::command]
pub async fn get_changed_champions_from_config(
  app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
  let cfg = super::config::load_config(app.clone()).await?;
  if let Some(last) = cfg.last_data_commit {
    get_changed_champions_since(last).await
  } else {
    Ok(Vec::new())
  }
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
