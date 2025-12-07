// Party mode chat monitoring and message handling

use base64::{engine::general_purpose, Engine};
use serde_json;
use std::sync::atomic::Ordering;
use tauri::AppHandle;

use super::types::{current_time_ms, CHAMP_SELECT_START_TIME_MS};
use super::utils::get_lcu_client;
use crate::commands::party_mode::PARTY_MODE_VERBOSE;

// Start monitoring LCU chat messages for party mode
#[tauri::command]
pub fn start_party_mode_chat_monitor(_app: AppHandle) -> Result<(), String> {
  // Party mode monitoring is now integrated into the main LCU watcher
  // This command is kept for backward compatibility but doesn't start a separate thread
  println!("[Party Mode] Chat monitoring is integrated into the main LCU watcher");
  Ok(())
}

// Check for party mode messages using existing connection info
pub fn check_for_party_mode_messages_with_connection(
  app: &AppHandle,
  port: &str,
  token: &str,
  processed_message_ids: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
  let client = get_lcu_client();
  let url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations", port);
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

  let verbose = PARTY_MODE_VERBOSE.load(Ordering::Relaxed);

  if verbose {
    println!("[Party Mode][DEBUG] Fetching conversations from: {}", url);
  }

  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .map_err(|e| format!("Failed to get conversations: {}", e))?;

  if !response.status().is_success() {
    if verbose {
      println!(
        "[Party Mode][DEBUG] Conversations request failed with status: {}",
        response.status()
      );
    }
    return Ok(());
  }

  let conversations: serde_json::Value = response
    .json()
    .map_err(|e| format!("Failed to parse conversations: {}", e))?;

  if let Some(conversations_array) = conversations.as_array() {
    if verbose {
      println!(
        "[Party Mode][DEBUG] Found {} conversations to scan",
        conversations_array.len()
      );
    }

    for conversation in conversations_array {
      if let Some(conversation_id) = conversation.get("id").and_then(|id| id.as_str()) {
        let pid = conversation
          .get("pid")
          .and_then(|v| v.as_str())
          .unwrap_or("");
        let conv_type = conversation
          .get("type")
          .and_then(|v| v.as_str())
          .unwrap_or("unknown");

        if verbose {
          println!(
            "[Party Mode][DEBUG] Scanning conversation: id={}, pid={}, type={}",
            conversation_id, pid, conv_type
          );
        }

        if let Err(e) = check_conversation_for_party_messages(
          app,
          &client,
          port,
          token,
          conversation_id,
          processed_message_ids,
        ) {
          if verbose {
            eprintln!(
              "[Party Mode][DEBUG] Error checking conversation {}: {}",
              conversation_id, e
            );
          }
        }
      }
    }
  } else if verbose {
    println!("[Party Mode][DEBUG] No conversations array in response");
  }

  Ok(())
}

// Check a specific conversation for party mode messages
pub fn check_conversation_for_party_messages(
  app: &AppHandle,
  client: &reqwest::blocking::Client,
  port: &str,
  token: &str,
  conversation_id: &str,
  processed_message_ids: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
  let url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages",
    port, conversation_id
  );
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));
  let verbose = PARTY_MODE_VERBOSE.load(Ordering::Relaxed);

  if verbose {
    println!("[Party Mode][DEBUG] Fetching messages from: {}", url);
  }

  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .map_err(|e| format!("Failed to get messages: {}", e))?;

  if !response.status().is_success() {
    if verbose {
      println!(
        "[Party Mode][DEBUG] Messages request failed with status: {}",
        response.status()
      );
    }
    return Ok(());
  }

  let messages: serde_json::Value = response
    .json()
    .map_err(|e| format!("Failed to parse messages: {}", e))?;

  if let Some(messages_array) = messages.as_array() {
    let total_messages = messages_array.len();
    let mut oss_messages_found = 0;
    let mut oss_messages_processed = 0;
    let mut oss_messages_skipped_processed = 0;
    let mut oss_messages_skipped_stale = 0;

    if verbose {
      println!(
        "[Party Mode][DEBUG] Conversation {} has {} messages",
        conversation_id, total_messages
      );
    }

    // Get session start time for filtering old messages
    let session_start_ms = CHAMP_SELECT_START_TIME_MS.load(Ordering::SeqCst);
    let now_ms = current_time_ms();

    for message in messages_array.iter() {
      // Get message ID to track processed messages (support string or numeric IDs)
      let message_id = message
        .get("id")
        .and_then(|id| {
          id.as_str()
            .map(|s| s.to_string())
            .or_else(|| id.as_u64().map(|n| n.to_string()))
        })
        .unwrap_or_else(|| "unknown".to_string());

      // Get message body
      let body = message.get("body").and_then(|b| b.as_str());

      // Only process OSS: messages
      if let Some(body_str) = body {
        if !body_str.starts_with("OSS:") {
          continue;
        }

        oss_messages_found += 1;

        // Skip if we've already processed this message
        if processed_message_ids.contains(&message_id) {
          oss_messages_skipped_processed += 1;
          continue;
        }

        // Get message timestamp for session validation
        let message_timestamp_ms = message
          .get("timestamp")
          .and_then(|ts| {
            // Try parsing as string first (ISO format), then as number
            ts.as_str()
              .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
              .map(|dt| dt.timestamp_millis() as u64)
              .or_else(|| ts.as_u64())
              .or_else(|| ts.as_i64().map(|i| i as u64))
          })
          .unwrap_or(0);

        // Parse timestamp from the OSS message itself (more reliable)
        let oss_timestamp_ms = extract_oss_message_timestamp(body_str);

        // Use OSS timestamp if available, otherwise use message timestamp
        let effective_timestamp = if oss_timestamp_ms > 0 {
          oss_timestamp_ms
        } else {
          message_timestamp_ms
        };

        if verbose {
          println!(
            "[Party Mode][DEBUG] OSS message id={}: msg_ts={}, oss_ts={}, effective_ts={}, session_start={}",
            message_id, message_timestamp_ms, oss_timestamp_ms, effective_timestamp, session_start_ms
          );
        }

        // Session-based filtering: ignore messages from before current session
        if session_start_ms > 0 && effective_timestamp > 0 && effective_timestamp < session_start_ms
        {
          oss_messages_skipped_stale += 1;
          if verbose {
            let age_secs = (session_start_ms - effective_timestamp) / 1000;
            println!(
              "[Party Mode][DEBUG] Skipping stale message id={} - predates session by {}s",
              message_id, age_secs
            );
          }
          // Still mark as processed so we don't log about it repeatedly
          processed_message_ids.insert(message_id);
          continue;
        }

        // Age-based filtering: ignore messages older than 5 minutes
        let max_age_ms = 5 * 60 * 1000; // 5 minutes
        if effective_timestamp > 0 && now_ms > effective_timestamp {
          let age_ms = now_ms - effective_timestamp;
          if age_ms > max_age_ms {
            oss_messages_skipped_stale += 1;
            if verbose {
              println!(
                "[Party Mode][DEBUG] Skipping old message id={} - age={}s (max={}s)",
                message_id,
                age_ms / 1000,
                max_age_ms / 1000
              );
            }
            // Still mark as processed
            processed_message_ids.insert(message_id);
            continue;
          }
        }

        // Get sender info
        let from_summoner_id = message
          .get("fromSummonerId")
          .and_then(|id| id.as_str())
          .or_else(|| message.get("fromId").and_then(|id| id.as_str()))
          .or_else(|| message.get("senderId").and_then(|id| id.as_str()));

        if let Some(from_id) = from_summoner_id {
          println!(
            "[Party Mode] Processing OSS message id={} from {} (age={}ms)",
            message_id,
            from_id,
            if effective_timestamp > 0 {
              now_ms.saturating_sub(effective_timestamp)
            } else {
              0
            }
          );

          // Mark this message as processed BEFORE handling
          processed_message_ids.insert(message_id.clone());
          oss_messages_processed += 1;

          // Process the message
          let rt = tokio::runtime::Runtime::new().unwrap();
          if let Err(e) = rt.block_on(crate::commands::party_mode::handle_party_mode_message(
            app, body_str, from_id,
          )) {
            eprintln!(
              "[Party Mode][ERROR] Failed to handle message id={}: {}",
              message_id, e
            );
          }
        } else {
          if verbose {
            println!(
              "[Party Mode][DEBUG] OSS message id={} has no sender ID, skipping",
              message_id
            );
          }
          // Mark as processed anyway
          processed_message_ids.insert(message_id);
        }
      }
    }

    // Summary logging
    if oss_messages_found > 0 {
      println!(
        "[Party Mode] Message scan complete: found={}, processed={}, skipped_already={}, skipped_stale={}",
        oss_messages_found, oss_messages_processed, oss_messages_skipped_processed, oss_messages_skipped_stale
      );
    }

    // Clean up old message IDs to prevent memory growth
    // Keep only the last 200 message IDs
    if processed_message_ids.len() > 200 {
      let mut ids_vec: Vec<String> = processed_message_ids.iter().cloned().collect();
      ids_vec.sort(); // Not perfect ordering, but good enough for cleanup
      let keep_count = 100;
      processed_message_ids.clear();
      for id in ids_vec.into_iter().rev().take(keep_count) {
        processed_message_ids.insert(id);
      }
      if verbose {
        println!(
          "[Party Mode][DEBUG] Trimmed processed IDs cache to {}",
          processed_message_ids.len()
        );
      }
    }
  } else if verbose {
    println!(
      "[Party Mode][DEBUG] No messages array in response for conversation {}",
      conversation_id
    );
  }

  Ok(())
}

/// Extract timestamp from OSS message JSON payload
fn extract_oss_message_timestamp(body: &str) -> u64 {
  if !body.starts_with("OSS:") {
    return 0;
  }

  let json_str = &body[4..]; // Skip "OSS:" prefix
  if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
    // Look for timestamp in the data field (skin_share messages have it there)
    if let Some(data) = value.get("data") {
      if let Some(ts) = data.get("timestamp").and_then(|v| v.as_u64()) {
        return ts;
      }
    }
    // Also try top-level timestamp
    if let Some(ts) = value.get("timestamp").and_then(|v| v.as_u64()) {
      return ts;
    }
  }

  0
}
