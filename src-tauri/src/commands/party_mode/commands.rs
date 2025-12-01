// Tauri commands for party mode

use super::lcu::{get_friend_display_name, get_friends_with_connection, get_lcu_connection};
use super::types::PARTY_MODE_VERBOSE;
use crate::commands::types::{FriendInfo, PairedFriend, PartyModeConfig, SavedConfig};
use crate::commands::ConfigLock;
use serde_json;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

// Tauri command to get friends list from LCU
#[tauri::command]
pub async fn get_lcu_friends(app: AppHandle) -> Result<Vec<FriendInfo>, String> {
  let lcu_connection = get_lcu_connection(&app).await?;
  return get_friends_with_connection(&lcu_connection.port, &lcu_connection.token).await;
}

// Tauri command to add a friend directly to party mode
#[tauri::command]
pub async fn add_party_friend(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
  friend_summoner_id: String,
) -> Result<String, String> {
  println!(
    "[DEBUG] Adding friend to party mode: {}",
    friend_summoner_id
  );

  // Get friend display name from LCU
  let friend_display_name = get_friend_display_name(&app, &friend_summoner_id)
    .await
    .unwrap_or_else(|_| format!("User {}", friend_summoner_id));

  // Add to paired friends with sharing enabled by default - NO CHAT MESSAGE SENT
  add_paired_friend(
    &app,
    &config_lock,
    &friend_summoner_id,
    &friend_display_name,
    true,
  )
  .await?;

  println!("[DEBUG] Successfully added friend to party mode silently!");
  Ok(friend_summoner_id)
}

// Tauri command to remove paired friend
#[tauri::command]
pub async fn remove_paired_friend(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
  friend_summoner_id: String,
) -> Result<(), String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if !config_file.exists() {
    return Ok(());
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;

  let mut config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  config
    .party_mode
    .paired_friends
    .retain(|f| f.summoner_id != friend_summoner_id);

  let updated_config = serde_json::to_string_pretty(&config)
    .map_err(|e| format!("Failed to serialize config: {}", e))?;

  std::fs::write(&config_file, updated_config)
    .map_err(|e| format!("Failed to save config: {}", e))?;

  // Emit event to update UI components
  let _ = app.emit("party-mode-paired-friends-updated", ());

  Ok(())
}

#[tauri::command]
pub async fn set_party_mode_verbose_logging(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
  enabled: bool,
) -> Result<bool, String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  let mut config: SavedConfig = if config_file.exists() {
    let raw =
      std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse config: {}", e))?
  } else {
    SavedConfig {
      league_path: None,
      skins: Vec::new(),
      custom_skins: Vec::new(),
      favorites: Vec::new(),
      theme: None,
      party_mode: PartyModeConfig::default(),
      selected_misc_items: std::collections::HashMap::new(),
      auto_update_data: true,
      last_data_commit: None,
    }
  };

  config.party_mode.verbose_logging = enabled;
  PARTY_MODE_VERBOSE.store(enabled, Ordering::Relaxed);

  std::fs::create_dir_all(&config_dir)
    .map_err(|e| format!("Failed to create config dir: {}", e))?;
  let serialized = serde_json::to_string_pretty(&config)
    .map_err(|e| format!("Failed to serialize config: {}", e))?;
  std::fs::write(&config_file, serialized)
    .map_err(|e| format!("Failed to persist config: {}", e))?;

  let _ = app.emit(
    "party-mode-config-updated",
    serde_json::json!({ "verbose_logging": enabled }),
  );
  Ok(enabled)
}

#[tauri::command]
pub async fn get_party_mode_verbose_logging(_app: AppHandle) -> Result<bool, String> {
  Ok(PARTY_MODE_VERBOSE.load(Ordering::Relaxed))
}

#[tauri::command]
pub async fn set_party_mode_max_share_age(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
  seconds: u64,
) -> Result<u64, String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  let mut config: SavedConfig = if config_file.exists() {
    let raw =
      std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse config: {}", e))?
  } else {
    SavedConfig {
      league_path: None,
      skins: Vec::new(),
      custom_skins: Vec::new(),
      favorites: Vec::new(),
      theme: None,
      party_mode: PartyModeConfig::default(),
      selected_misc_items: std::collections::HashMap::new(),
      auto_update_data: true,
      last_data_commit: None,
    }
  };

  config.party_mode.max_share_age_secs = seconds;

  std::fs::create_dir_all(&config_dir)
    .map_err(|e| format!("Failed to create config dir: {}", e))?;
  let serialized = serde_json::to_string_pretty(&config)
    .map_err(|e| format!("Failed to serialize config: {}", e))?;
  std::fs::write(&config_file, serialized)
    .map_err(|e| format!("Failed to persist config: {}", e))?;

  let _ = app.emit(
    "party-mode-config-updated",
    serde_json::json!({ "max_share_age_secs": seconds }),
  );
  Ok(seconds)
}

// Tauri command to get paired friends
#[tauri::command]
pub async fn get_paired_friends(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
) -> Result<Vec<PairedFriend>, String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  println!(
    "[Party Mode] Loading paired friends from: {:?}",
    config_file
  );

  if !config_file.exists() {
    println!("[Party Mode] Config file does not exist, returning empty list");
    return Ok(Vec::new());
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;

  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  println!(
    "[Party Mode] Loaded {} paired friends from config",
    config.party_mode.paired_friends.len()
  );
  for friend in &config.party_mode.paired_friends {
    println!(
      "[Party Mode] - Friend: {} ({}) - Sharing: {}",
      friend.display_name, friend.summoner_id, friend.share_enabled
    );
  }

  Ok(config.party_mode.paired_friends)
}

#[tauri::command]
pub async fn get_party_mode_settings(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
) -> Result<bool, String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if !config_file.exists() {
    return Ok(true); // Default notifications enabled
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;

  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  Ok(config.party_mode.notifications)
}

#[tauri::command]
pub async fn update_party_mode_settings(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
  notifications: bool,
) -> Result<(), String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  std::fs::create_dir_all(&config_dir)
    .map_err(|e| format!("Failed to create config directory: {}", e))?;

  let mut config = if config_file.exists() {
    let config_data =
      std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
    serde_json::from_str::<SavedConfig>(&config_data)
      .map_err(|e| format!("Failed to parse config: {}", e))?
  } else {
    SavedConfig {
      league_path: None,
      skins: Vec::new(),
      custom_skins: Vec::new(),
      favorites: Vec::new(),
      theme: None,
      party_mode: PartyModeConfig::default(),
      selected_misc_items: std::collections::HashMap::new(),
      auto_update_data: true,
      last_data_commit: None,
    }
  };

  config.party_mode.notifications = notifications;

  let updated_config = serde_json::to_string_pretty(&config)
    .map_err(|e| format!("Failed to serialize config: {}", e))?;

  std::fs::write(&config_file, updated_config)
    .map_err(|e| format!("Failed to save config: {}", e))?;

  Ok(())
}

// Internal function to add paired friend
async fn add_paired_friend(
  app: &AppHandle,
  config_lock: &ConfigLock,
  friend_summoner_id: &str,
  friend_name: &str,
  share_enabled: bool,
) -> Result<(), String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  std::fs::create_dir_all(&config_dir)
    .map_err(|e| format!("Failed to create config directory: {}", e))?;

  let mut config = if config_file.exists() {
    let config_data =
      std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
    serde_json::from_str::<SavedConfig>(&config_data)
      .map_err(|e| format!("Failed to parse config: {}", e))?
  } else {
    SavedConfig {
      league_path: None,
      skins: Vec::new(),
      custom_skins: Vec::new(),
      favorites: Vec::new(),
      theme: None,
      party_mode: PartyModeConfig::default(),
      selected_misc_items: std::collections::HashMap::new(),
      auto_update_data: true,
      last_data_commit: None,
    }
  };

  // Check if friend is already paired
  if !config
    .party_mode
    .paired_friends
    .iter()
    .any(|f| f.summoner_id == friend_summoner_id)
  {
    config.party_mode.paired_friends.push(PairedFriend {
      summoner_id: friend_summoner_id.to_string(),
      summoner_name: friend_name.to_string(),
      display_name: friend_name.to_string(),
      paired_at: chrono::Utc::now().timestamp_millis() as u64,
      share_enabled,
    });

    let updated_config = serde_json::to_string_pretty(&config)
      .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
      .map_err(|e| format!("Failed to save config: {}", e))?;

    // Emit event to update UI components
    let _ = app.emit("party-mode-paired-friends-updated", ());
  }

  Ok(())
}
