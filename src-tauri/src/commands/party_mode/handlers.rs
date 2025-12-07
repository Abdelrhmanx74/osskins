// Message handlers and skin sharing logic

use super::lcu::{get_current_summoner, get_lcu_connection};
use super::messaging::send_chat_message;
use super::party_detection::{
  get_current_party_member_summoner_ids, get_gameflow_party_member_summoner_ids,
};
use super::session::{prune_stale_received_skins, refresh_session_tracker};
use super::types::{
  is_message_from_current_session, log_debug, log_error, log_info, log_verbose, log_warn,
  CurrentSummoner, InMemoryReceivedSkin, PARTY_MODE_MESSAGE_PREFIX, RECEIVED_SKINS,
  SENT_SKIN_SHARES,
};
use super::utils::{get_skin_name_from_config, received_skin_key, sent_share_key};
use crate::commands::lcu_watcher::types::{
  current_time_ms, CHAMP_SELECT_SESSION_COUNTER, CHAMP_SELECT_START_TIME_MS,
};
use crate::commands::types::{PairedFriend, PartyModeMessage, SavedConfig, SkinShare};
use base64::{engine::general_purpose, Engine};
use serde_json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

// Function to handle incoming party mode messages (called from LCU watcher)
pub async fn handle_party_mode_message(
  app: &AppHandle,
  message_body: &str,
  from_summoner_id: &str,
) -> Result<(), String> {
  // Log entry for debugging
  log_debug(&format!(
    "handle_party_mode_message called: from={}, body_len={}",
    from_summoner_id,
    message_body.len()
  ));

  if !message_body.starts_with(PARTY_MODE_MESSAGE_PREFIX) {
    log_verbose(&format!(
      "Message does not start with OSS prefix, ignoring: {}",
      &message_body[..message_body.len().min(50)]
    ));
    return Ok(()); // Not a party mode message
  }

  let message_json = &message_body[PARTY_MODE_MESSAGE_PREFIX.len()..];
  log_debug(&format!("Parsing message JSON: {}", message_json));

  let message: PartyModeMessage = match serde_json::from_str(message_json) {
    Ok(m) => m,
    Err(e) => {
      log_error(&format!("Failed to parse party mode message: {}", e));
      return Err(format!("Failed to parse party mode message: {}", e));
    }
  };

  log_debug(&format!("Parsed message type: {}", message.message_type));

  // Get current summoner to filter out our own messages
  let current_summoner = match get_current_summoner(app).await {
    Ok(s) => {
      log_debug(&format!(
        "Current summoner: {} ({})",
        s.display_name, s.summoner_id
      ));
      s
    }
    Err(e) => {
      log_warn(&format!(
        "Could not resolve current summoner ({}); using placeholder",
        e
      ));
      CurrentSummoner {
        summoner_id: "unknown".into(),
        display_name: "Unknown".into(),
      }
    }
  };

  // Refresh session tracker
  refresh_session_tracker(app).await;

  match message.message_type.as_str() {
    "skin_share" => {
      let skin_share: SkinShare = match serde_json::from_value(message.data) {
        Ok(s) => s,
        Err(e) => {
          log_error(&format!("Failed to parse skin_share data: {}", e));
          return Err(format!("Failed to parse skin share: {}", e));
        }
      };

      log_info(&format!(
        "Received skin_share from {} ({}) for champion {} skin {}",
        skin_share.from_summoner_name,
        skin_share.from_summoner_id,
        skin_share.champion_id,
        skin_share.skin_id
      ));

      // Filter out our own messages
      if skin_share.from_summoner_id == current_summoner.summoner_id {
        log_debug(&format!(
          "Ignoring skin_share from self ({})",
          current_summoner.display_name
        ));
        return Ok(());
      }

      // Validate timestamp - is this message from the current champ select session?
      let message_timestamp = skin_share.timestamp;
      let session_start = CHAMP_SELECT_START_TIME_MS.load(Ordering::SeqCst);
      let now_ms = current_time_ms();

      log_debug(&format!(
        "Message timestamp validation: msg_ts={}, session_start={}, now={}, diff_from_session={}ms",
        message_timestamp,
        session_start,
        now_ms,
        if message_timestamp >= session_start {
          (message_timestamp - session_start) as i64
        } else {
          -((session_start - message_timestamp) as i64)
        }
      ));

      // Check if message is from current session using the helper
      if !is_message_from_current_session(message_timestamp) {
        log_info(&format!(
          "Ignoring stale skin_share from {} - message timestamp {} predates session start {}",
          skin_share.from_summoner_name, message_timestamp, session_start
        ));
        return Ok(());
      }

      // Additional sanity check: message shouldn't be too old (5 minutes max)
      let age_ms = now_ms.saturating_sub(message_timestamp);
      let max_age_ms = 5 * 60 * 1000; // 5 minutes
      if age_ms > max_age_ms {
        log_info(&format!(
          "Ignoring old skin_share from {} - message is {}s old (max {}s)",
          skin_share.from_summoner_name,
          age_ms / 1000,
          max_age_ms / 1000
        ));
        return Ok(());
      }

      log_debug(&format!(
        "Message passed timestamp validation (age={}ms)",
        age_ms
      ));

      // Store the skin share
      let key = received_skin_key(&skin_share.from_summoner_id, skin_share.champion_id);
      let mut map = RECEIVED_SKINS
        .lock()
        .map_err(|e| format!("Failed to lock RECEIVED_SKINS: {}", e))?;

      let before = map.len();
      let existing = map.get(&key).map(|s| (s.skin_id, s.champion_id));

      map.insert(
        key.clone(),
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

      let after = map.len();

      if let Some((old_skin_id, old_champ_id)) = existing {
        if old_skin_id != skin_share.skin_id || old_champ_id != skin_share.champion_id {
          log_info(&format!(
            "Updated skin share from {} - champion {} skin {} -> {} (reroll/swap detected)",
            skin_share.from_summoner_name, old_champ_id, old_skin_id, skin_share.skin_id
          ));
        } else {
          log_debug(&format!(
            "Re-received same skin share from {} (possibly resent)",
            skin_share.from_summoner_name
          ));
        }
      } else {
        log_info(&format!(
          "Stored NEW skin share from {} for champion {} (cache: {} -> {})",
          skin_share.from_summoner_name, skin_share.champion_id, before, after
        ));
      }

      drop(map);

      // Emit events for the frontend
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
            "session_counter": CHAMP_SELECT_SESSION_COUNTER.load(Ordering::SeqCst),
        }),
      );

      log_info(&format!(
        "✅ Processed skin share from {} for champion {} skin {}",
        skin_share.from_summoner_name, skin_share.champion_id, skin_share.skin_id
      ));
    }
    other => {
      log_debug(&format!("Ignoring message type: {}", other));
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
  log_info(&format!(
    "=== send_skin_share_to_paired_friends called: champ={}, skin={}, chroma={:?}, force={} ===",
    champion_id, skin_id, chroma_id, force_send_to_all
  ));

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if !config_file.exists() {
    log_warn("Config file does not exist, cannot send skin shares");
    return Ok(());
  }

  let config_data = match std::fs::read_to_string(&config_file) {
    Ok(data) => data,
    Err(e) => {
      log_error(&format!("Failed to read config: {}", e));
      return Err(format!("Failed to read config: {}", e));
    }
  };

  let config: SavedConfig = match serde_json::from_str(&config_data) {
    Ok(c) => c,
    Err(e) => {
      log_error(&format!("Failed to parse config: {}", e));
      return Err(format!("Failed to parse config: {}", e));
    }
  };

  // Get friends with sharing enabled
  let sharing_friends: Vec<&PairedFriend> = config
    .party_mode
    .paired_friends
    .iter()
    .filter(|friend| friend.share_enabled)
    .collect();

  log_debug(&format!(
    "Paired friends with sharing enabled: {} total",
    sharing_friends.len()
  ));

  for friend in &sharing_friends {
    log_debug(&format!(
      "  - {} ({}) share_enabled={}",
      friend.display_name, friend.summoner_id, friend.share_enabled
    ));
  }

  if sharing_friends.is_empty() {
    log_info("No paired friends with sharing enabled; skipping broadcast");
    return Ok(());
  }

  // Get LCU connection
  let lcu_conn = match get_lcu_connection(app).await {
    Ok(conn) => {
      log_debug(&format!("LCU connection established on port {}", conn.port));
      conn
    }
    Err(e) => {
      log_warn(&format!(
        "Unable to reach LCU ({}); skipping share to avoid spamming friends",
        e
      ));
      return Ok(());
    }
  };

  // Get party member IDs to filter who we send to
  let mut party_member_ids: HashSet<String> = HashSet::new();
  let mut membership_source = "none";

  match get_current_party_member_summoner_ids(&lcu_conn).await {
    Ok(ids) if !ids.is_empty() => {
      membership_source = "champ-select/lobby";
      log_debug(&format!(
        "Got {} party members from champ-select/lobby",
        ids.len()
      ));
      party_member_ids = ids;
    }
    Ok(_) => {
      log_debug("Champ-select/lobby returned no party members");
    }
    Err(e) => {
      log_debug(&format!("Champ-select/lobby lookup failed: {}", e));
    }
  }

  // Fallback to gameflow if champ-select didn't work
  if party_member_ids.is_empty() {
    match get_gameflow_party_member_summoner_ids(&lcu_conn).await {
      Ok(ids) if !ids.is_empty() => {
        membership_source = "gameflow";
        log_debug(&format!("Got {} party members from gameflow", ids.len()));
        party_member_ids = ids;
      }
      Ok(_) => {
        log_debug("Gameflow did not expose teammate IDs");
      }
      Err(e) => {
        log_debug(&format!("Gameflow lookup failed: {}", e));
      }
    }
  }

  log_info(&format!(
    "Party membership source: {}, members: {:?}",
    membership_source,
    party_member_ids.iter().take(5).collect::<Vec<_>>()
  ));

  if party_member_ids.is_empty() && !force_send_to_all {
    log_warn(
      "No party membership data available; skipping skin_share to avoid spamming unrelated friends",
    );
    return Ok(());
  }

  // Separate friends into those in party and those outside
  let mut sharing_friends_in_party: Vec<&PairedFriend> = Vec::new();
  let mut sharing_friends_outside: Vec<&PairedFriend> = Vec::new();

  for friend in &sharing_friends {
    let friend_id_trimmed = friend.summoner_id.trim().to_string();
    let in_party = party_member_ids
      .iter()
      .any(|id| id.trim() == friend_id_trimmed);
    if in_party {
      log_debug(&format!(
        "Friend {} ({}) IS in current party",
        friend.display_name, friend.summoner_id
      ));
      sharing_friends_in_party.push(*friend);
    } else {
      log_debug(&format!(
        "Friend {} ({}) is NOT in current party",
        friend.display_name, friend.summoner_id
      ));
      sharing_friends_outside.push(*friend);
    }
  }

  let sharing_in_party_count = sharing_friends_in_party.len();
  let sharing_outside_count = sharing_friends_outside.len();

  log_info(&format!(
    "Friends in party: {}, friends outside party: {}",
    sharing_in_party_count, sharing_outside_count
  ));

  if sharing_in_party_count == 0 && !force_send_to_all {
    log_info("No paired friends detected in your confirmed party; not sending skin_share");
    if sharing_outside_count > 0 {
      log_debug(&format!(
        "Would have sent to {} friends outside party but force_send_to_all=false",
        sharing_outside_count
      ));
    }
    return Ok(());
  }

  // Determine target friends
  let use_party_targets = sharing_in_party_count > 0;
  let target_friends = if use_party_targets {
    sharing_friends_in_party
  } else {
    sharing_friends_outside
  };

  if force_send_to_all && !use_party_targets {
    log_info(&format!(
      "Force-sending skin_share to {} paired friend(s) outside the detected party",
      target_friends.len()
    ));
  }

  // Get current summoner info for the message
  let current_summoner = get_current_summoner(app).await?;
  let skin_name = get_skin_name_from_config(app, champion_id, skin_id)
    .unwrap_or_else(|| format!("Skin {}", skin_id));

  log_debug(&format!(
    "Creating skin_share message: summoner={}, champ={}, skin={} ({}), chroma={:?}",
    current_summoner.display_name, champion_id, skin_id, skin_name, chroma_id
  ));

  // Create the skin share message
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

  log_info(&format!(
    "📤 Sending skin_share to {} friend(s)...",
    target_friends.len()
  ));

  // Send to each target friend
  let mut success_count = 0;
  let mut skip_count = 0;
  let mut error_count = 0;

  for friend in target_friends {
    let key = sent_share_key(&friend.summoner_id, champion_id, skin_id, chroma_id);

    // Check if we already sent this share
    {
      let sent = SENT_SKIN_SHARES
        .lock()
        .map_err(|e| format!("Failed to lock SENT_SKIN_SHARES: {}", e))?;
      if sent.contains(&key) {
        log_debug(&format!(
          "Skipping {} - already sent this share this phase",
          friend.summoner_name
        ));
        skip_count += 1;
        continue;
      }
    }

    // Mark as sent BEFORE sending (to prevent duplicates)
    {
      let mut sent = SENT_SKIN_SHARES
        .lock()
        .map_err(|e| format!("Failed to lock SENT_SKIN_SHARES: {}", e))?;
      sent.insert(key.clone());
    }

    log_debug(&format!(
      "Sending to {} ({})...",
      friend.summoner_name, friend.summoner_id
    ));

    match send_chat_message(app, &lcu_conn, &friend.summoner_id, &message).await {
      Ok(_) => {
        success_count += 1;
        log_info(&format!(
          "✅ Sent skin {} (champ {}) to {}",
          skin_name, champion_id, friend.summoner_name
        ));

        // Emit success events
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
      Err(e) => {
        error_count += 1;
        log_error(&format!(
          "Failed to send to {} ({}): {}",
          friend.summoner_name, friend.summoner_id, e
        ));

        // Remove from sent set so we can retry
        if let Ok(mut sent) = SENT_SKIN_SHARES.lock() {
          sent.remove(&key);
        }
      }
    }
  }

  log_info(&format!(
    "=== Skin share complete: {} sent, {} skipped, {} errors ===",
    success_count, skip_count, error_count
  ));

  Ok(())
}

// Helper function to check if all paired friends with sharing enabled have shared their skins for a specific champion
pub async fn should_inject_now(app: &AppHandle, champion_id: u32) -> Result<bool, String> {
  log_debug(&format!(
    "should_inject_now called: champion_id={}",
    champion_id
  ));

  if champion_id == 0 {
    log_debug("Champion not locked (id=0); holding injection");
    return Ok(false);
  }

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if !config_file.exists() {
    log_debug("Config file not found; allowing injection");
    return Ok(true);
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  // Get friends with sharing enabled
  let friends_with_sharing: Vec<&PairedFriend> = config
    .party_mode
    .paired_friends
    .iter()
    .filter(|f| f.share_enabled)
    .collect();

  log_debug(&format!(
    "Friends with sharing enabled: {} total",
    friends_with_sharing.len()
  ));

  for friend in &friends_with_sharing {
    log_verbose(&format!(
      "  - {} ({})",
      friend.display_name, friend.summoner_id
    ));
  }

  if friends_with_sharing.is_empty() {
    log_info(&format!(
      "No paired friends have sharing enabled; injecting local skins for champion {}",
      champion_id
    ));
    return Ok(true);
  }

  // Get LCU connection to check party membership
  let lcu_connection = match get_lcu_connection(app).await {
    Ok(conn) => conn,
    Err(e) => {
      log_warn(&format!(
        "Could not reach LCU ({}); defaulting to inject to avoid blocking",
        e
      ));
      return Ok(true);
    }
  };

  // Get party member IDs
  let mut party_member_ids = get_current_party_member_summoner_ids(&lcu_connection)
    .await
    .unwrap_or_default();

  if party_member_ids.is_empty() {
    log_debug("Champ-select party lookup empty, trying gameflow");
    party_member_ids = get_gameflow_party_member_summoner_ids(&lcu_connection)
      .await
      .unwrap_or_default();
  }

  log_debug(&format!("Party member IDs: {:?}", party_member_ids));

  // Normalize IDs for comparison
  let party_ids_normalized: HashSet<String> = party_member_ids
    .into_iter()
    .map(|s| s.trim().to_string())
    .collect();

  // Filter to friends actually in the party
  let friends_in_party: Vec<&PairedFriend> = friends_with_sharing
    .into_iter()
    .filter(|f| party_ids_normalized.contains(&f.summoner_id.trim().to_string()))
    .collect();

  log_debug(&format!(
    "Friends in current party: {} of sharing-enabled friends",
    friends_in_party.len()
  ));

  if friends_in_party.is_empty() {
    log_info(&format!(
      "No paired friends detected in your current party; injecting champion {} with local cosmetics",
      champion_id
    ));
    return Ok(true);
  }

  // Detect game mode for special handling
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

        // Check for ARAM
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

        // Check for Swift Play (multiple champion selections)
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

  log_debug(&format!(
    "Game mode detection: queueId={:?}, mode={:?}, is_aram={}, is_swift={}",
    queue_id, game_mode, is_aram, is_swift
  ));

  // Check which friends have shared their skins
  let mut friends_who_shared: HashSet<String> = HashSet::new();
  {
    let map = RECEIVED_SKINS
      .lock()
      .map_err(|e| format!("Failed to lock RECEIVED_SKINS: {}", e))?;

    log_debug(&format!("RECEIVED_SKINS cache has {} entries", map.len()));

    for (key, value) in map.iter() {
      log_verbose(&format!(
        "  Cache entry: key={}, from={}, champ={}, skin={}",
        key, value.from_summoner_name, value.champion_id, value.skin_id
      ));
      friends_who_shared.insert(value.from_summoner_id.clone());
    }
  }

  let total = friends_in_party.len();
  let shared = friends_in_party
    .iter()
    .filter(|f| friends_who_shared.contains(&f.summoner_id))
    .count();

  log_info(&format!(
    "Party mode status: {}/{} friends have shared their skins",
    shared, total
  ));

  // Log individual friend status
  for friend in &friends_in_party {
    let status = if friends_who_shared.contains(&friend.summoner_id) {
      "✅ shared"
    } else {
      "⏳ waiting"
    };
    log_debug(&format!(
      "  {} ({}) -> {}",
      friend.display_name, friend.summoner_id, status
    ));
  }

  // Prune stale received skins
  prune_stale_received_skins(app);

  // Check if we should inject
  if shared == total {
    log_info(&format!(
      "✅ All paired friends have shared; injecting champion {}",
      champion_id
    ));
    return Ok(true);
  }

  // ARAM special handling - inject early to avoid delays
  if is_aram {
    if shared > 0 {
      log_info(&format!(
        "[ARAM] Partial shares {}/{}; injecting early to avoid ARAM delays",
        shared, total
      ));
      return Ok(true);
    }
  }

  // Swift Play special handling - inject at 50% threshold
  if is_swift && total > 0 && shared * 2 >= total {
    log_info(&format!(
      "[SwiftPlay] >=50% shares ({}/{}); injecting",
      shared, total
    ));
    return Ok(true);
  }

  log_info(&format!(
    "⏳ Waiting for {} more friend share(s) before injecting champion {}",
    total - shared,
    champion_id
  ));
  Ok(false)
}
