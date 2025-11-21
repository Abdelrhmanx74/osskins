// Message handlers and skin sharing logic

use crate::{verbose_log, normal_log};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use serde_json;
use base64::{engine::general_purpose, Engine};
use crate::commands::types::{
  PartyModeMessage, PairedFriend, SavedConfig, SkinShare,
};
use super::types::{RECEIVED_SKINS, SENT_SKIN_SHARES, PARTY_MODE_MESSAGE_PREFIX, CurrentSummoner, InMemoryReceivedSkin};
use super::utils::{received_skin_key, sent_share_key, get_skin_name_from_config};
use super::lcu::{get_lcu_connection, get_current_summoner};
use super::messaging::send_chat_message;
use super::party_detection::{get_current_party_member_summoner_ids, get_gameflow_party_member_summoner_ids};
use super::session::{refresh_session_tracker, prune_stale_received_skins};


// Function to handle incoming party mode messages (called from LCU watcher)
pub async fn handle_party_mode_message(
  app: &AppHandle,
  message_body: &str,
  _from_summoner_id: &str,
) -> Result<(), String> {
  if !message_body.starts_with(PARTY_MODE_MESSAGE_PREFIX) {
    return Ok(()); // Not a party mode message
  }
  let message_json = &message_body[PARTY_MODE_MESSAGE_PREFIX.len()..];
  verbose_log!("[Party Mode][INBOUND][RAW] {}", message_json);
  let message: PartyModeMessage = serde_json::from_str(message_json)
    .map_err(|e| format!("Failed to parse party mode message: {}", e))?;
  verbose_log!(
    "[Party Mode][INBOUND][PARSED] type={}",
    message.message_type
  );

  let current_summoner = match get_current_summoner(app).await {
    Ok(s) => s,
    Err(e) => {
      normal_log!(
        "[Party Mode][WARN] Could not resolve current summoner ({}); continuing",
        e
      );
      CurrentSummoner {
        summoner_id: "unknown".into(),
        display_name: "Unknown".into(),
      }
    }
  };

  refresh_session_tracker(app).await;

  match message.message_type.as_str() {
    "skin_share" => {
      let skin_share: SkinShare = serde_json::from_value(message.data)
        .map_err(|e| format!("Failed to parse skin share: {}", e))?;

      if skin_share.from_summoner_id == current_summoner.summoner_id {
        verbose_log!(
          "[Party Mode][INBOUND] Ignoring self skin_share from {}",
          current_summoner.display_name
        );
        return Ok(());
      }

  let share_ts = match i64::try_from(skin_share.timestamp) {
    Ok(ts) => ts,
    Err(_) => {
      verbose_log!(
        "[Party Mode][INBOUND][SKIP] timestamp {} overflows i64",
        skin_share.timestamp
      );
      return Ok(());
    }
  };

  // Note: Previous implementation enforced message staleness checks here and skipped
  // messages older than a configured maximum or a previous session. That logic has been
  // removed in favour of explicit cleanup via the LCU API DELETE endpoint after
  // friend skin injections. Keep receiving all messages (it's up to other logic to
  // handle session transitions / pruning).

  // Still compute message age for logging purposes but do not skip messages
  let age_secs: u64 = (SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs() as i64 - share_ts).max(0) as u64;

      let key = received_skin_key(&skin_share.from_summoner_id, skin_share.champion_id);
      let mut map = RECEIVED_SKINS.lock().map_err(|e| format!("Failed to lock RECEIVED_SKINS: {}", e))?;
      let before = map.len();
      map.insert(
        key,
        InMemoryReceivedSkin {
          from_summoner_id: skin_share.from_summoner_id.clone(),
          from_summoner_name: skin_share.from_summoner_name.clone(),
          champion_id: skin_share.champion_id,
          skin_id: skin_share.skin_id,
          chroma_id: skin_share.chroma_id,
          skin_file_path: skin_share.skin_file_path.clone(),
          received_at: skin_share.timestamp,
        },
      );
      verbose_log!(
        "[Party Mode][INBOUND][STORE] cache size {} -> {}",
        before,
        map.len()
      );
      drop(map);

      let _ = app.emit("party-mode-skin-received", skin_share.clone());
      let _ = app.emit(
        "party-mode-chat-log",
        serde_json::json!({
            "direction": "received",
            "from": skin_share.from_summoner_name,
            "from_id": skin_share.from_summoner_id,
            "champion_id": skin_share.champion_id,
            "skin_id": skin_share.skin_id,
            "skin_name": skin_share.skin_name,
            "chroma_id": skin_share.chroma_id,
            "skin_file_path": skin_share.skin_file_path,
            "timestamp": skin_share.timestamp,
        }),
      );

      normal_log!(
        "[Party Mode] Stored skin share from {} for champion {}",
        skin_share.from_summoner_name,
        skin_share.champion_id
      );
    }
    other => {
      verbose_log!("[Party Mode][INBOUND] Ignoring message type {}", other);
    }
  }

  Ok(())
}

// Function to send skin share to paired friends with sharing enabled (called from LCU watcher on champion lock)
pub async fn send_skin_share_to_paired_friends(
  app: &AppHandle,
  champion_id: u32,
  skin_id: u32,
  chroma_id: Option<u32>,
  skin_file_path: Option<String>,
  force_send_to_all: bool,
) -> Result<(), String> {
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
  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  let sharing_friends: Vec<&PairedFriend> = config
    .party_mode
    .paired_friends
    .iter()
    .filter(|friend| friend.share_enabled)
    .collect();

  if sharing_friends.is_empty() {
    verbose_log!(
      "[Party Mode][OUTBOUND] No paired friends with sharing enabled; skipping broadcast"
    );
    return Ok(());
  }

  verbose_log!("[Party Mode][OUTBOUND] Preparing share payload champ={} skin={} chroma={:?} skin_file={:?} to {} friends",
        champion_id, skin_id, chroma_id, skin_file_path, sharing_friends.len());

  let lcu_conn = match get_lcu_connection(app).await {
    Ok(conn) => conn,
    Err(e) => {
      normal_log!("[Party Mode][OUTBOUND][WARN] Unable to reach LCU ({}); skipping share to avoid spamming friends", e);
      return Ok(());
    }
  };

  let mut party_member_ids: HashSet<String> = HashSet::new();
  let mut membership_source = None;

  match get_current_party_member_summoner_ids(&lcu_conn).await {
    Ok(ids) if !ids.is_empty() => {
      membership_source = Some("champ-select/lobby");
      party_member_ids = ids;
    }
    _ => match get_gameflow_party_member_summoner_ids(&lcu_conn).await {
      Ok(ids) if !ids.is_empty() => {
        membership_source = Some("gameflow");
        party_member_ids = ids;
      }
      Ok(_) => {
        verbose_log!("[Party Mode][OUTBOUND] Gameflow did not expose teammate IDs");
      }
      Err(e) => {
        verbose_log!("[Party Mode][OUTBOUND] Gameflow lookup failed: {}", e);
      }
    },
  }

  if party_member_ids.is_empty() {
    normal_log!("[Party Mode][OUTBOUND] No party membership data available; skipping skin_share to avoid spamming unrelated friends. Enable verbose logging for diagnostics.");
    if !force_send_to_all {
      return Ok(());
    }
  }

  verbose_log!(
    "[Party Mode][OUTBOUND] Party member IDs source={:?} values={:?}",
    membership_source,
    party_member_ids
  );

  let mut sharing_friends_in_party: Vec<&PairedFriend> = Vec::new();
  let mut sharing_friends_outside: Vec<&PairedFriend> = Vec::new();

  for friend in &sharing_friends {
    if party_member_ids.contains(&friend.summoner_id) {
      sharing_friends_in_party.push(*friend);
    } else {
      sharing_friends_outside.push(*friend);
    }
  }

  let sharing_in_party_count = sharing_friends_in_party.len();
  let sharing_outside_count = sharing_friends_outside.len();

  if sharing_in_party_count == 0 && !force_send_to_all {
    normal_log!("[Party Mode] Sharing enabled but none of the paired friends are in your confirmed party; not sending skin_share.");
    if sharing_outside_count > 0 {
      normal_log!(
        "[Party Mode][INFO] Not sending to {} paired friend(s) outside your party: {:?}",
        sharing_outside_count,
        sharing_friends_outside
          .iter()
          .map(|f| format!("{}({})", f.display_name, f.summoner_id))
          .collect::<Vec<_>>()
      );
    }
    return Ok(());
  }

  if sharing_outside_count > 0 && !force_send_to_all {
    normal_log!(
      "[Party Mode][INFO] Not sending to {} paired friend(s) outside your party: {:?}",
      sharing_outside_count,
      sharing_friends_outside
        .iter()
        .map(|f| format!("{}({})", f.display_name, f.summoner_id))
        .collect::<Vec<_>>()
    );
  }

  let use_party_targets = sharing_in_party_count > 0;
  let target_friends = if use_party_targets {
    sharing_friends_in_party
  } else {
    sharing_friends_outside
  };

  if force_send_to_all && !use_party_targets {
    normal_log!(
      "[Party Mode] Force-sending skin_share to {} paired friend(s) outside the detected party",
      target_friends.len()
    );
  }

  let current_summoner = get_current_summoner(app).await?;
  let skin_name = get_skin_name_from_config(app, champion_id, skin_id)
    .unwrap_or_else(|| format!("Skin {}", skin_id));

  let skin_share = SkinShare {
    from_summoner_id: current_summoner.summoner_id.clone(),
    from_summoner_name: current_summoner.display_name.clone(),
    champion_id,
    skin_id,
    skin_name: skin_name.clone(),
    chroma_id,
    skin_file_path: skin_file_path.clone(),
    timestamp: chrono::Utc::now().timestamp_millis() as u64,
  };

  let message = PartyModeMessage {
    message_type: "skin_share".into(),
    data: serde_json::to_value(&skin_share)
      .map_err(|e| format!("Failed to serialize skin share: {}", e))?,
  };

  verbose_log!(
    "[Party Mode][OUTBOUND] Delivering skin_share to {} paired friend(s)",
    target_friends.len()
  );

  for friend in target_friends {
    let key = sent_share_key(&friend.summoner_id, champion_id, skin_id, chroma_id);
    {
      let mut sent = SENT_SKIN_SHARES.lock().map_err(|e| format!("Failed to lock SENT_SKIN_SHARES: {}", e))?;
      if sent.contains(&key) {
        verbose_log!(
          "[Party Mode][OUTBOUND] Skipping {} (already sent this phase)",
          friend.summoner_name
        );
        continue;
      }
      sent.insert(key.clone());
    }

    verbose_log!(
      "[Party Mode][OUTBOUND] -> {}({})",
      friend.summoner_name,
      friend.summoner_id
    );
    if let Err(e) = send_chat_message(app, &lcu_conn, &friend.summoner_id, &message).await {
      let mut sent = SENT_SKIN_SHARES.lock().map_err(|e| format!("Failed to lock SENT_SKIN_SHARES: {}", e))?;
      sent.remove(&key);
      eprintln!(
        "[Party Mode][OUTBOUND][ERROR] Failed to send to {}({}): {}",
        friend.summoner_name, friend.summoner_id, e
      );
      continue;
    }

    normal_log!(
      "[Party Mode] Shared skin {} (champ {} skin {}) with {}",
      skin_name,
      champion_id,
      skin_id,
      friend.summoner_name
    );

    let _ = app.emit(
      "party-mode-skin-sent",
      serde_json::json!({
          "to_friend": friend.summoner_name,
          "champion_id": champion_id,
          "skin_name": skin_name
      }),
    );
    let _ = app.emit(
      "party-mode-chat-log",
      serde_json::json!({
          "direction": "sent",
          "to": friend.summoner_name,
          "to_id": friend.summoner_id,
          "champion_id": champion_id,
          "skin_id": skin_id,
          "skin_name": skin_name,
          "chroma_id": chroma_id,
          "skin_file_path": skin_file_path.clone(),
          "timestamp": skin_share.timestamp,
      }),
    );
  }

  Ok(())
}

// Helper function to check if all paired friends with sharing enabled have shared their skins for a specific champion
pub async fn should_inject_now(app: &AppHandle, champion_id: u32) -> Result<bool, String> {
  if champion_id == 0 {
    verbose_log!("[Party Mode][INJECT] champion not locked; holding");
    return Ok(false);
  }

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if !config_file.exists() {
    return Ok(true);
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  let friends_with_sharing: Vec<&PairedFriend> = config
    .party_mode
    .paired_friends
    .iter()
    .filter(|f| f.share_enabled)
    .collect();

  verbose_log!(
    "[Party Mode][INJECT] sharing-enabled friends: {:?}",
    friends_with_sharing
      .iter()
      .map(|f| (&f.display_name, &f.summoner_id))
      .collect::<Vec<_>>()
  );

  if friends_with_sharing.is_empty() {
    normal_log!(
      "[Party Mode] No paired friends have sharing enabled; injecting local skins for champion {}",
      champion_id
    );
    return Ok(true);
  }

  let lcu_connection = match get_lcu_connection(app).await {
    Ok(conn) => conn,
    Err(e) => {
      normal_log!("[Party Mode][INJECT][WARN] Could not reach LCU ({}); defaulting to inject to avoid blocking", e);
      return Ok(true);
    }
  };

  let mut party_member_ids = get_current_party_member_summoner_ids(&lcu_connection)
    .await
    .unwrap_or_default();
  if party_member_ids.is_empty() {
    party_member_ids = get_gameflow_party_member_summoner_ids(&lcu_connection)
      .await
      .unwrap_or_default();
  }

  verbose_log!(
    "[Party Mode][INJECT] party member IDs: {:?}",
    party_member_ids
  );

  let party_ids_normalized: HashSet<String> = party_member_ids
    .into_iter()
    .map(|s| s.trim().to_string())
    .collect();
  let friends_in_party: Vec<&PairedFriend> = friends_with_sharing
    .into_iter()
    .filter(|f| party_ids_normalized.contains(&f.summoner_id.trim().to_string()))
    .collect();

  if friends_in_party.is_empty() {
    normal_log!("[Party Mode] No paired friends detected in your current party; injecting champion {} with local cosmetics", champion_id);
    return Ok(true);
  }

  let mut queue_id = None;
  let mut game_mode = None;
  let mut is_aram = false;
  let mut is_swift = false;

  if let Ok(client) = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
  {
    let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));
    let url = format!(
      "https://127.0.0.1:{}/lol-gameflow/v1/session",
      lcu_connection.port
    );
    if let Ok(resp) = client
      .get(&url)
      .header("Authorization", format!("Basic {}", auth))
      .send()
      .await
    {
      if let Ok(json) = resp.json::<serde_json::Value>().await {
        queue_id = json
          .get("gameData")
          .and_then(|gd| gd.get("queue"))
          .and_then(|q| q.get("id"))
          .and_then(|v| v.as_i64());
        game_mode = json
          .get("gameData")
          .and_then(|gd| gd.get("gameMode"))
          .and_then(|v| v.as_str())
          .map(|s| s.to_string());
        if let Some(q) = queue_id {
          if q == 450 {
            is_aram = true;
          }
        }
        if let Some(mode) = &game_mode {
          if mode.eq_ignore_ascii_case("aram") {
            is_aram = true;
          }
        }

        if let Some(gd) = json.get("gameData") {
          if let Some(pcs) = gd
            .get("playerChampionSelections")
            .and_then(|v| v.as_array())
          {
            for selection in pcs {
              if let Some(ids) = selection.get("championIds").and_then(|v| v.as_array()) {
                if ids.len() >= 2 {
                  is_swift = true;
                  break;
                }
              }
            }
          }
          if let Some(selected) = gd.get("selectedChampions").and_then(|v| v.as_array()) {
            if selected.len() >= 2 {
              is_swift = true;
            }
          }
        }
      }
    }
  }

  verbose_log!(
    "[Party Mode][INJECT] queueId={:?} mode={:?} is_aram={} is_swift_play={}",
    queue_id,
    game_mode,
    is_aram,
    is_swift
  );

  let mut friends_who_shared: HashSet<String> = HashSet::new();
  {
    let map = RECEIVED_SKINS.lock().map_err(|e| format!("Failed to lock RECEIVED_SKINS: {}", e))?;
    for value in map.values() {
      friends_who_shared.insert(value.from_summoner_id.clone());
    }
  }

  let total = friends_in_party.len();
  let shared = friends_in_party
    .iter()
    .filter(|f| friends_who_shared.contains(&f.summoner_id))
    .count();

  verbose_log!("[Party Mode][INJECT] friends shared {}/{}", shared, total);

  for friend in &friends_in_party {
    let status = if friends_who_shared.contains(&friend.summoner_id) {
      "shared"
    } else {
      "waiting"
    };
    verbose_log!(
      "[Party Mode][INJECT][FRIEND] {}({}) -> {}",
      friend.display_name,
      friend.summoner_id,
      status
    );
  }

  prune_stale_received_skins(app);

  if shared == total {
    normal_log!(
      "[Party Mode] All paired friends have shared; injecting champion {}",
      champion_id
    );
    return Ok(true);
  }

  if is_aram {
    if shared > 0 {
      normal_log!(
        "[Party Mode][ARAM] Partial shares {}/{}; injecting early to avoid ARAM delays",
        shared,
        total
      );
      return Ok(true);
    }
  }

  if is_swift && total > 0 && shared * 2 >= total {
    normal_log!(
      "[Party Mode][SwiftPlay] >=50% shares ({}/{}); injecting",
      shared,
      total
    );
    return Ok(true);
  }

  normal_log!(
    "[Party Mode] Waiting for {} more friend share(s) before injecting champion {}",
    total - shared,
    champion_id
  );
  Ok(false)
}
