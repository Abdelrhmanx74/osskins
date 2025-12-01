// Party member detection and game mode utilities

use super::types::LcuConnection;
use base64::{engine::general_purpose, Engine};
use serde_json;
use std::collections::HashSet;

pub async fn get_current_party_member_summoner_ids(
  lcu: &LcuConnection,
) -> Result<std::collections::HashSet<String>, String> {
  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu.token));

  // Try champ select session first
  let cs_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", lcu.port);
  if let Ok(resp) = client
    .get(&cs_url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
  {
    if resp.status().is_success() {
      if let Ok(json) = resp.json::<serde_json::Value>().await {
        let mut ids = std::collections::HashSet::new();
        if let Some(team) = json.get("myTeam").and_then(|v| v.as_array()) {
          for p in team {
            if let Some(id) = p.get("summonerId").and_then(|v| v.as_i64()) {
              ids.insert(id.to_string());
            }
          }
        }
        if !ids.is_empty() {
          return Ok(ids);
        }
      }
    }
  }

  // Fallback to lobby data
  let lobby_url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", lcu.port);
  if let Ok(resp) = client
    .get(&lobby_url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
  {
    if resp.status().is_success() {
      if let Ok(json) = resp.json::<serde_json::Value>().await {
        let mut ids = std::collections::HashSet::new();
        if let Some(members) = json.get("members").and_then(|v| v.as_array()) {
          for m in members {
            if let Some(id) = m.get("summonerId").and_then(|v| v.as_i64()) {
              ids.insert(id.to_string());
            }
          }
        }
        return Ok(ids);
      }
    }
  }

  Ok(std::collections::HashSet::new())
}

// FUNCTION: get_gameflow_party_member_summoner_ids
pub async fn get_gameflow_party_member_summoner_ids(
  lcu: &LcuConnection,
) -> Result<HashSet<String>, String> {
  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu.token));
  let url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", lcu.port);
  let resp = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
    .map_err(|e| format!("Gameflow request failed: {}", e))?;

  if !resp.status().is_success() {
    return Err(format!("Gameflow status {}", resp.status()));
  }

  let json: serde_json::Value = resp
    .json()
    .await
    .map_err(|e| format!("Failed to parse gameflow JSON: {}", e))?;
  let mut ids: HashSet<String> = HashSet::new();

  if let Some(game_data) = json.get("gameData") {
    collect_ids_from_json(game_data, &mut ids);
  }

  if ids.is_empty() {
    collect_ids_from_json(&json, &mut ids);
  }

  Ok(ids)
}

// FUNCTION: collect_ids_from_json
pub fn collect_ids_from_json(value: &serde_json::Value, acc: &mut HashSet<String>) {
  match value {
    serde_json::Value::Array(arr) => {
      for entry in arr {
        collect_ids_from_json(entry, acc);
      }
    }
    serde_json::Value::Object(map) => {
      if let Some(id) = map.get("summonerId") {
        if let Some(id_num) = id.as_i64() {
          acc.insert(id_num.to_string());
        } else if let Some(id_str) = id.as_str() {
          acc.insert(id_str.to_string());
        }
      }
      if let Some(id_list) = map
        .get("summonerIds")
        .or_else(|| map.get("teamOneSummonerIds"))
        .or_else(|| map.get("teamTwoSummonerIds"))
      {
        collect_ids_from_json(id_list, acc);
      }
      for value in map.values() {
        collect_ids_from_json(value, acc);
      }
    }
    serde_json::Value::Number(num) => {
      if let Some(n) = num.as_i64() {
        acc.insert(n.to_string());
      }
    }
    serde_json::Value::String(s) => {
      if s.chars().all(|c| c.is_numeric()) {
        acc.insert(s.clone());
      }
    }
    _ => {}
  }
}
