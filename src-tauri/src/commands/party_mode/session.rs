// Session tracking and skin management

use crate::normal_log;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{engine::general_purpose, Engine};
use tauri::{AppHandle, Manager};
use super::types::{CURRENT_SESSION_ID, RECEIVED_SKINS};
use super::lcu::get_lcu_connection;
use super::utils::get_configured_max_share_age_secs;

pub async fn refresh_session_tracker(app: &AppHandle) {
  let maybe_lcu = get_lcu_connection(app).await;
  let Ok(lcu) = maybe_lcu else {
    return;
  };
  let client = match reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
  {
    Ok(c) => c,
    Err(_) => return,
  };

  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu.token));
  let url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", lcu.port);
  if let Ok(resp) = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
  {
    if let Ok(json) = resp.json::<serde_json::Value>().await {
      let session_id = json
        .get("gameId")
        .and_then(|v| v.as_i64())
        .map(|id| format!("game:{}", id))
        .or_else(|| {
          json
            .get("gameData")
            .and_then(|gd| gd.get("gameId"))
            .and_then(|v| v.as_i64())
            .map(|id| format!("game:{}", id))
        })
        .or_else(|| {
          let bucket = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 600)
            * 600;
          Some(format!("bucket:{}", bucket))
        });

      if let Some(new_id) = session_id {
        let mut guard = CURRENT_SESSION_ID.lock().unwrap();
        if guard.as_ref() != Some(&new_id) {
          normal_log!(
            "[Party Mode][SESSION] session changed {:?} -> {}; clearing received skins",
            *guard,
            new_id
          );
          clear_received_skins();
          *guard = Some(new_id);
        }
      }
    }
  }
}

// FUNCTION: prune_stale_received_skins
pub fn prune_stale_received_skins(app: &AppHandle) {
  let configured_max = get_configured_max_share_age_secs(app);
  let mut map = RECEIVED_SKINS.lock().unwrap();
  if map.is_empty() {
    return;
  }
  let now_secs = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs() as i64;
  let before = map.len();
  map.retain(|_, skin| {
    if let Ok(ts) = i64::try_from(skin.received_at) {
      let age = (now_secs - ts).max(0) as u64;
      age <= configured_max
    } else {
      false
    }
  });
  let after = map.len();
  if after != before {
    normal_log!(
      "[Party Mode][CLEANUP] pruned stale received skins {} -> {} (max_age={}s)",
      before,
      after,
      configured_max
    );
  }
}

// Add a function to clear received skins (call this when leaving champ select or starting a new session)
pub fn clear_received_skins() {
  let mut map = RECEIVED_SKINS.lock().unwrap();
  let before = map.len();
  map.clear();
  normal_log!("[Party Mode][STATE] cleared received skins {} -> 0", before);
}
