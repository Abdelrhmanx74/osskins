// Party mode chat monitoring and message handling

use base64::{engine::general_purpose, Engine};
use serde_json;
use std::sync::atomic::Ordering;
use tauri::AppHandle;

use super::utils::get_lcu_client;
use crate::commands::party_mode::PARTY_MODE_VERBOSE;

// Start monitoring LCU chat messages for party mode
#[tauri::command]
pub fn start_party_mode_chat_monitor(_app: AppHandle) -> Result<(), String> {
  // Party mode monitoring is now integrated into the main LCU watcher
  // This command is kept for backward compatibility but doesn't start a separate thread
  println!("Party mode chat monitoring is integrated into the main LCU watcher");
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

  if PARTY_MODE_VERBOSE.load(Ordering::Relaxed) {
    println!(
      "[Party Mode][DEBUG] Fetching conversations for OSS scan: {}",
      url
    );
  }
  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .map_err(|e| format!("Failed to get conversations: {}", e))?;

  if !response.status().is_success() {
    return Ok(());
  }

  let conversations: serde_json::Value = response
    .json()
    .map_err(|e| format!("Failed to parse conversations: {}", e))?;

  if let Some(conversations_array) = conversations.as_array() {
    if PARTY_MODE_VERBOSE.load(Ordering::Relaxed) {
      println!(
        "[Party Mode][DEBUG] Conversations found: {}",
        conversations_array.len()
      );
    }
    for conversation in conversations_array {
      if let Some(conversation_id) = conversation.get("id").and_then(|id| id.as_str()) {
        let pid = conversation
          .get("pid")
          .and_then(|v| v.as_str())
          .unwrap_or("");
        if PARTY_MODE_VERBOSE.load(Ordering::Relaxed) {
          println!(
            "[Party Mode][DEBUG] Scanning conversation id={} pid={}",
            conversation_id, pid
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
          eprintln!("Error checking conversation {}: {}", conversation_id, e);
        }
      }
    }
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

  if PARTY_MODE_VERBOSE.load(Ordering::Relaxed) {
    println!(
      "[Party Mode][DEBUG] Fetching messages for conversation: {}",
      url
    );
  }
  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .map_err(|e| format!("Failed to get messages: {}", e))?;

  if !response.status().is_success() {
    return Ok(());
  }

  let messages: serde_json::Value = response
    .json()
    .map_err(|e| format!("Failed to parse messages: {}", e))?;

  if let Some(messages_array) = messages.as_array() {
    if PARTY_MODE_VERBOSE.load(Ordering::Relaxed) {
      println!(
        "[Party Mode][DEBUG] Messages count in conversation: {}",
        messages_array.len()
      );
    }
    // Check all messages, not just recent ones, but skip already processed messages
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

      // Skip if we've already processed this message
      if processed_message_ids.contains(&message_id) {
        // skip silently, but we can log verbose when debugging
        continue;
      }

      let body = message.get("body").and_then(|b| b.as_str());
      let from_summoner_id = message
        .get("fromSummonerId")
        .and_then(|id| id.as_str())
        .or_else(|| message.get("fromId").and_then(|id| id.as_str()))
        .or_else(|| message.get("senderId").and_then(|id| id.as_str()));

      if let (Some(body), Some(from_summoner_id)) = (body, from_summoner_id) {
        // Only print debug info for OSS messages to reduce noise
        if body.starts_with("OSS:") {
          println!(
            "[Party Mode] Found OSS message from {}: {}",
            from_summoner_id, body
          );
          println!(
            "[Party Mode][DEBUG] Marking message id={} as processed",
            message_id
          );

          // Mark this message as processed before handling it
          processed_message_ids.insert(message_id);

          let rt = tokio::runtime::Runtime::new().unwrap();
          if let Err(e) = rt.block_on(crate::commands::party_mode::handle_party_mode_message(
            app,
            body,
            from_summoner_id,
          )) {
            eprintln!("Error handling party mode message: {}", e);
          }
        }
      } else {
        // Debug: Check what fields are available in the message
        if message.as_object().is_some() {
          let available_fields: Vec<String> =
            message.as_object().unwrap().keys().cloned().collect();
          println!(
            "[Party Mode] Debug: Message has fields: {:?}",
            available_fields
          );
        }
      }
    }

    // Clean up old message IDs to prevent memory growth
    // Keep only the last 100 message IDs
    if processed_message_ids.len() > 100 {
      let mut ids_vec: Vec<String> = processed_message_ids.iter().cloned().collect();
      ids_vec.sort(); // Not perfect ordering, but good enough for cleanup
      let keep_count = 50;
      processed_message_ids.clear();
      for id in ids_vec.into_iter().rev().take(keep_count) {
        processed_message_ids.insert(id);
      }
      println!(
        "[Party Mode][DEBUG] Processed IDs trimmed to {}",
        processed_message_ids.len()
      );
    }
  } else {
    println!(
      "[Party Mode] No messages array found in response for conversation {}",
      conversation_id
    );
  }

  Ok(())
}
