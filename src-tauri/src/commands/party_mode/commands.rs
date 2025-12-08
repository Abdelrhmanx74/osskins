// Tauri commands for party mode

use super::lcu::{get_friend_display_name, get_friends_with_connection, get_lcu_connection};
use super::types::{PARTY_MODE_VERBOSE, RECEIVED_SKINS, SENT_SKIN_SHARES};
use crate::commands::lcu_watcher::types::{
  current_time_ms, CHAMP_SELECT_SESSION_COUNTER, CHAMP_SELECT_START_TIME_MS,
  LAST_SHARED_CHAMPION_ID, LCU_WATCHER_ACTIVE, LCU_WATCHER_INSTANCE_ID,
  PARTY_INJECTION_DONE_THIS_PHASE,
};
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
      start_hidden: false,
      last_data_commit: None,
      cslol_tools_version: None,
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
      start_hidden: false,
      last_data_commit: None,
      cslol_tools_version: None,
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
      start_hidden: false,
      last_data_commit: None,
      cslol_tools_version: None,
    }
  };

  config.party_mode.notifications = notifications;

  let updated_config = serde_json::to_string_pretty(&config)
    .map_err(|e| format!("Failed to serialize config: {}", e))?;

  std::fs::write(&config_file, updated_config)
    .map_err(|e| format!("Failed to save config: {}", e))?;

  Ok(())
}

/// Diagnostic command to dump the current party mode state for debugging.
/// This is useful for understanding what's happening when party mode isn't working as expected.
#[tauri::command]
pub async fn get_party_mode_diagnostic_state(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
) -> Result<serde_json::Value, String> {
  let _lock = config_lock
    .0
    .lock()
    .map_err(|_| "Failed to lock config".to_string())?;

  // Gather session state
  let now_ms = current_time_ms();
  let session_start_ms = CHAMP_SELECT_START_TIME_MS.load(Ordering::SeqCst);
  let session_counter = CHAMP_SELECT_SESSION_COUNTER.load(Ordering::SeqCst);
  let last_shared_champ = LAST_SHARED_CHAMPION_ID.load(Ordering::SeqCst);
  let injection_done = PARTY_INJECTION_DONE_THIS_PHASE.load(Ordering::Relaxed);
  let watcher_active = LCU_WATCHER_ACTIVE.load(Ordering::SeqCst);
  let watcher_id = LCU_WATCHER_INSTANCE_ID.load(Ordering::SeqCst);
  let verbose_enabled = PARTY_MODE_VERBOSE.load(Ordering::Relaxed);

  // Calculate session age
  let session_age_secs = if session_start_ms > 0 {
    (now_ms.saturating_sub(session_start_ms)) / 1000
  } else {
    0
  };

  // Gather received skins
  let received_skins: Vec<serde_json::Value> = {
    let map = RECEIVED_SKINS
      .lock()
      .map_err(|e| format!("Failed to lock RECEIVED_SKINS: {}", e))?;
    map
      .iter()
      .map(|(key, skin)| {
        let age_ms = now_ms.saturating_sub(skin.received_at);
        serde_json::json!({
          "key": key,
          "from_summoner_id": skin.from_summoner_id,
          "from_summoner_name": skin.from_summoner_name,
          "champion_id": skin.champion_id,
          "skin_id": skin.skin_id,
          "chroma_id": skin.chroma_id,
          "skin_file_path": skin.skin_file_path,
          "received_at": skin.received_at,
          "age_seconds": age_ms / 1000,
        })
      })
      .collect()
  };

  // Gather sent shares
  let sent_shares: Vec<String> = {
    let set = SENT_SKIN_SHARES
      .lock()
      .map_err(|e| format!("Failed to lock SENT_SKIN_SHARES: {}", e))?;
    set.iter().cloned().collect()
  };

  // Get paired friends from config
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  let paired_friends: Vec<serde_json::Value> = if config_file.exists() {
    let config_data =
      std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
    let config: SavedConfig =
      serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;
    config
      .party_mode
      .paired_friends
      .iter()
      .map(|f| {
        serde_json::json!({
          "summoner_id": f.summoner_id,
          "display_name": f.display_name,
          "share_enabled": f.share_enabled,
        })
      })
      .collect()
  } else {
    Vec::new()
  };

  // Build diagnostic output
  let diagnostic = serde_json::json!({
    "timestamp": now_ms,
    "session": {
      "counter": session_counter,
      "start_time_ms": session_start_ms,
      "age_seconds": session_age_secs,
      "last_shared_champion_id": last_shared_champ,
      "injection_done_this_phase": injection_done,
    },
    "watcher": {
      "active": watcher_active,
      "instance_id": watcher_id,
    },
    "config": {
      "verbose_logging": verbose_enabled,
      "paired_friends_count": paired_friends.len(),
      "paired_friends": paired_friends,
    },
    "state": {
      "received_skins_count": received_skins.len(),
      "received_skins": received_skins,
      "sent_shares_count": sent_shares.len(),
      "sent_shares": sent_shares,
    },
  });

  // Also print to console for easy access in logs
  println!("=== PARTY MODE DIAGNOSTIC STATE ===");
  println!(
    "{}",
    serde_json::to_string_pretty(&diagnostic).unwrap_or_default()
  );
  println!("=== END DIAGNOSTIC STATE ===");

  Ok(diagnostic)
}

/// Manually trigger a resend of the current skin to all paired friends.
/// This is useful when the automatic sharing didn't work or when you want to
/// update friends after changing your skin selection.
#[tauri::command]
pub async fn resend_skin_to_friends(
  app: AppHandle,
  config_lock: State<'_, ConfigLock>,
) -> Result<serde_json::Value, String> {
  use crate::commands::party_mode::send_skin_share_to_paired_friends;

  println!("[Party Mode] Manual resend triggered by user");

  // Read config while holding the lock, then release it before async operations
  let (champion_id, skin_id, chroma_id, skin_file) = {
    let _lock = config_lock
      .0
      .lock()
      .map_err(|_| "Failed to lock config".to_string())?;

    // Get config to find current champion and skin selection
    let config_dir = app
      .path()
      .app_data_dir()
      .unwrap_or_else(|_| PathBuf::from("."))
      .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
      return Err("Config file not found".to_string());
    }

    let config_data =
      std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
    let config: SavedConfig =
      serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

    // Get the last shared champion ID
    let last_shared = LAST_SHARED_CHAMPION_ID.load(Ordering::SeqCst) as u32;

    if last_shared == 0 {
      return Err(
        "No champion has been shared yet in this session. Select a champion first.".to_string(),
      );
    }

    // Find the skin for this champion
    let skin = config
      .skins
      .iter()
      .find(|s| s.champion_id == last_shared)
      .ok_or_else(|| format!("No skin configured for champion {}", last_shared))?;

    println!(
      "[Party Mode] Resending skin {} for champion {} to all friends (force_send_to_all=true)",
      skin.skin_id, skin.champion_id
    );

    // Clear the sent shares so we can resend
    {
      let mut sent = SENT_SKIN_SHARES
        .lock()
        .map_err(|e| format!("Failed to lock SENT_SKIN_SHARES: {}", e))?;
      let before = sent.len();
      sent.clear();
      println!(
        "[Party Mode] Cleared {} sent share entries to allow resend",
        before
      );
    }

    (
      skin.champion_id,
      skin.skin_id,
      skin.chroma_id,
      skin.skin_file.clone(),
    )
  }; // _lock is dropped here

  // Send to all paired friends with force=true to bypass party detection
  match send_skin_share_to_paired_friends(
    &app,
    champion_id,
    skin_id,
    chroma_id,
    skin_file,
    true, // force_send_to_all
  )
  .await
  {
    Ok(_) => {
      println!("[Party Mode] Manual resend completed successfully");
      Ok(serde_json::json!({
        "success": true,
        "champion_id": champion_id,
        "skin_id": skin_id,
        "message": "Skin share resent to all paired friends"
      }))
    }
    Err(e) => {
      println!("[Party Mode] Manual resend failed: {}", e);
      Err(format!("Failed to resend skin: {}", e))
    }
  }
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
      start_hidden: false,
      last_data_commit: None,
      cslol_tools_version: None,
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
