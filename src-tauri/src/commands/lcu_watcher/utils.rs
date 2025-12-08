// Utility functions for LCU watcher

use serde_json;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

use super::types::{
  InjectionMode, LAST_INSTANT_ASSIGN_CHAMPIONS, LAST_SHARED_CHAMPION_ID, PHASE_STATE,
};
use crate::commands::party_mode::RECEIVED_SKINS;
use crate::commands::types::SavedConfig;
use crate::injection::MiscItem;
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

/// Compute a signature for the current party mode injection state.
/// This signature is used to detect when re-injection is needed.
///
/// The signature changes when:
/// 1. The local champion changes (reroll in ARAM/URF)
/// 2. A friend's shared skin changes (they rerolled)
/// 3. New friend skins are received
///
/// This allows re-injection when champions change during ARAM/URF rerolls.
pub fn compute_party_injection_signature(current_champion_id: u32) -> String {
  let map = RECEIVED_SKINS.lock().unwrap();

  // Track the last champion we computed a signature for
  let last_champ = LAST_SHARED_CHAMPION_ID.load(Ordering::SeqCst) as u32;
  let champion_changed = last_champ != 0 && last_champ != current_champion_id;

  if champion_changed {
    println!(
      "[Party Mode][SIGNATURE] Champion changed from {} to {} - this will trigger re-injection",
      last_champ, current_champion_id
    );
  }

  // Update the last shared champion
  LAST_SHARED_CHAMPION_ID.store(current_champion_id as u64, Ordering::SeqCst);

  // Build signature from received skins
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

  // Include the number of received skins to detect new arrivals
  let received_count = map.len();

  // Build the final signature
  let signature = if parts.is_empty() {
    format!(
      "champion:{}:received:{}",
      current_champion_id, received_count
    )
  } else {
    format!(
      "champion:{}:received:{}|{}",
      current_champion_id,
      received_count,
      parts.join("|")
    )
  };

  println!(
    "[Party Mode][SIGNATURE] Computed signature for champ {}: {} ({} received skins)",
    current_champion_id,
    &signature[..signature.len().min(80)],
    received_count
  );

  signature
}

/// Compute a signature for instant-assign (multi-champion) injections.
/// Captures champion selections, local/custom skin choices, and misc selections
/// so we can re-inject if the user picks a new skin after the first injection.
pub fn compute_instant_assign_signature(
  champion_ids: &[u32],
  config: &SavedConfig,
  misc_items: &[MiscItem],
) -> String {
  let mut champs = champion_ids.to_vec();
  champs.sort_unstable();
  champs.dedup();

  let mut selections: Vec<String> = Vec::new();
  for cid in champs.iter() {
    if let Some(skin) = config.skins.iter().find(|s| s.champion_id == *cid) {
      selections.push(format!(
        "{}:skin:{}:{}:{}",
        cid,
        skin.skin_id,
        skin.chroma_id.unwrap_or(0),
        skin.skin_file.clone().unwrap_or_default()
      ));
    } else if let Some(custom) = config.custom_skins.iter().find(|s| s.champion_id == *cid) {
      selections.push(format!("{}:custom:{}", cid, custom.file_path));
    } else {
      selections.push(format!("{}:none", cid));
    }
  }
  selections.sort();

  let mut misc_keys: Vec<String> = misc_items
    .iter()
    .map(|m| format!("{}:{}", m.item_type, m.id))
    .collect();
  misc_keys.sort();

  format!(
    "champs:{}|selections:{}|misc:{}",
    champs
      .iter()
      .map(|c| c.to_string())
      .collect::<Vec<_>>()
      .join(","),
    selections.join("|"),
    misc_keys.join(",")
  )
}

/// Check if a champion change occurred (useful for ARAM/URF reroll detection)
#[allow(dead_code)]
pub fn did_champion_change(new_champion_id: u32) -> bool {
  let last_champ = LAST_SHARED_CHAMPION_ID.load(Ordering::SeqCst) as u32;
  last_champ != 0 && last_champ != new_champion_id
}

/// Reset the last shared champion tracking (call when entering new ChampSelect)
#[allow(dead_code)]
pub fn reset_champion_tracking() {
  LAST_SHARED_CHAMPION_ID.store(0, Ordering::SeqCst);
  println!("[Party Mode][TRACKING] Reset champion tracking for new session");
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

/// Log a summary of received skins for debugging
#[allow(dead_code)]
pub fn log_received_skins_summary() {
  let map = RECEIVED_SKINS.lock().unwrap();
  if map.is_empty() {
    println!("[Party Mode][DEBUG] No received skins in cache");
    return;
  }

  println!(
    "[Party Mode][DEBUG] Received skins cache ({} entries):",
    map.len()
  );
  for (key, skin) in map.iter() {
    println!(
      "[Party Mode][DEBUG]   {} -> {} from {} (champ {} skin {})",
      key, skin.from_summoner_name, skin.from_summoner_id, skin.champion_id, skin.skin_id
    );
  }
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
