// Utility functions for LCU watcher

use serde_json;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

use super::types::{InjectionMode, PHASE_STATE};
use crate::commands::party_mode::RECEIVED_SKINS;
use std::sync::atomic::Ordering;

pub fn is_in_champ_select() -> bool {
  PHASE_STATE.load(Ordering::Relaxed) == 1
}

pub fn read_injection_mode(app: &AppHandle) -> InjectionMode {
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let cfg_file = config_dir.join("config.json");
  if let Ok(data) = std::fs::read_to_string(&cfg_file) {
    if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
      if let Some(mode_val) = cfg.get("injection_mode").and_then(|v| v.as_str()) {
        match mode_val.to_lowercase().as_str() {
          "lobby" | "lobby_mode" => return InjectionMode::Lobby,
          "champselect" | "champ_select" | "champselect_mode" | "champ" => {
            return InjectionMode::ChampSelect
          }
          _ => {}
        }
      }
    }
  }
  // Default to ChampSelect mode
  InjectionMode::ChampSelect
}

pub fn compute_party_injection_signature(current_champion_id: u32) -> String {
  // Build a stable signature that includes the local locked champion id plus the
  // currently received friend skins. This ensures a champion change forces
  // injection even if the set of received friend skins didn't change.
  let map = RECEIVED_SKINS.lock().unwrap();
  let mut parts: Vec<String> = map
    .values()
    .map(|s| {
      format!(
        "{}:{}:{}:{}",
        s.from_summoner_id,
        s.champion_id,
        s.skin_id,
        s.chroma_id.unwrap_or(0)
      )
    })
    .collect();
  parts.sort();
  // Prefix with champion id so local champion selection influences the signature
  if parts.is_empty() {
    format!("champion:{}", current_champion_id)
  } else {
    format!("champion:{}|{}", current_champion_id, parts.join("|"))
  }
}

pub fn get_lcu_client() -> reqwest::blocking::Client {
  static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

  CLIENT
    .get_or_init(|| {
      reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
    .clone()
}

#[allow(dead_code)]
pub fn format_json_summary(json: &serde_json::Value) -> String {
  let mut summary = String::new();

  if let Some(phase) = json.get("phase") {
    summary.push_str(&format!("phase: {}, ", phase.as_str().unwrap_or("unknown")));
  }

  if let Some(_game_data) = json.get("gameData") {
    summary.push_str("gameData: {...}, ");
  }

  if let Some(actions) = json.get("actions") {
    summary.push_str(&format!(
      "actions: [{} items], ",
      actions.as_array().map_or(0, |a| a.len())
    ));
  }

  if summary.is_empty() {
    summary = "[Response summary unavailable]".to_string();
  }

  summary
}
