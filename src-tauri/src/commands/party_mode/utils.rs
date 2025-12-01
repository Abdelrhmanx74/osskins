// Utility functions for party mode

use super::types::{MAX_SHARE_AGE_SECS, SENT_SKIN_SHARES};
use crate::commands::types::SavedConfig;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// Helper to generate key for received skin
pub fn received_skin_key(from_summoner_id: &str, champion_id: u32) -> String {
  format!("{}_{}", from_summoner_id, champion_id)
}

// Helper to generate key for sent share deduplication
pub fn sent_share_key(
  friend_summoner_id: &str,
  champion_id: u32,
  skin_id: u32,
  chroma_id: Option<u32>,
) -> String {
  let chroma = chroma_id.unwrap_or(0);
  format!(
    "{}_{}_{}_{}",
    friend_summoner_id.trim(),
    champion_id,
    skin_id,
    chroma
  )
}

// Exposed so watcher can reset dedupers on phase changes
pub fn clear_sent_shares() {
  if let Ok(mut s) = SENT_SKIN_SHARES.lock() {
    s.clear();
  }
}

pub fn get_configured_max_share_age_secs(app: &AppHandle) -> u64 {
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");
  if !config_file.exists() {
    return MAX_SHARE_AGE_SECS;
  }

  match std::fs::read_to_string(&config_file) {
    Ok(contents) => match serde_json::from_str::<SavedConfig>(&contents) {
      Ok(cfg) => cfg.party_mode.max_share_age_secs,
      Err(_) => MAX_SHARE_AGE_SECS,
    },
    Err(_) => MAX_SHARE_AGE_SECS,
  }
}

// Helper function to get skin name from config
pub fn get_skin_name_from_config(
  app: &AppHandle,
  champion_id: u32,
  skin_id: u32,
) -> Option<String> {
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if let Ok(config_data) = std::fs::read_to_string(&config_file) {
    if let Ok(config) = serde_json::from_str::<SavedConfig>(&config_data) {
      for skin in &config.skins {
        if skin.champion_id == champion_id && skin.skin_id == skin_id {
          if let Some(ref skin_file_path) = skin.skin_file {
            if let Some(file_name) = std::path::Path::new(skin_file_path).file_stem() {
              if let Some(name_str) = file_name.to_str() {
                return Some(name_str.to_string());
              }
            }
          }
          return Some(format!("Champion {} Skin {}", champion_id, skin_id));
        }
      }
    }
  }

  None
}
