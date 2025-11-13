// Chat messaging and skin sharing

use std::path::PathBuf;
use base64::{engine::general_purpose, Engine};
use tauri::{AppHandle, Emitter, Manager};
use serde_json;
use crate::commands::types::{PartyModeMessage, SkinShare};
use super::types::{LcuConnection, PARTY_MODE_MESSAGE_PREFIX};
use super::lcu::get_conversation_id;

// Internal function to send chat message
pub async fn send_chat_message(
  app: &AppHandle,
  lcu_connection: &LcuConnection,
  friend_summoner_id: &str,
  message: &PartyModeMessage,
) -> Result<(), String> {
  println!(
    "[DEBUG] send_chat_message called for friend_summoner_id: {}",
    friend_summoner_id
  );

  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  // Get conversation ID with the friend
  println!("[DEBUG] Getting conversation ID...");
  let conversation_id = get_conversation_id(app, lcu_connection, friend_summoner_id).await?;
  println!("[DEBUG] Got conversation ID: {}", conversation_id);

  let message_json =
    serde_json::to_string(message).map_err(|e| format!("Failed to serialize message: {}", e))?;

  let full_message = format!("{}{}", PARTY_MODE_MESSAGE_PREFIX, message_json);

  let url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages",
    lcu_connection.port, conversation_id
  );
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));

  let message_payload = serde_json::json!({
      "body": full_message,
      "type": "chat"
  });

  println!("[DEBUG] Sending message to URL: {}", url);
  let response = client
    .post(&url)
    .header("Authorization", format!("Basic {}", auth))
    .header("Content-Type", "application/json")
    .json(&message_payload)
    .send()
    .await
    .map_err(|e| format!("Failed to send message: {}", e))?;

  if !response.status().is_success() {
    return Err(format!("Failed to send message: {}", response.status()));
  }

  println!("[DEBUG] Message sent successfully!");
  Ok(())
}