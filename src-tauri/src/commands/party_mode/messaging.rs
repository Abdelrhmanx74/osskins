// Chat messaging and skin sharing

use base64::{engine::general_purpose, Engine};
use serde_json;
use tauri::AppHandle;
use crate::commands::types::PartyModeMessage;
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

// Delete all messages in a conversation (cleans both sides in a 1:1 chat)
pub async fn delete_conversation_messages(
  app: &tauri::AppHandle,
  lcu_connection: &LcuConnection,
  friend_summoner_id: &str,
) -> Result<(), String> {
  println!(
    "[DEBUG] delete_conversation_messages called for friend_summoner_id: {}",
    friend_summoner_id
  );

  // SAFETY: The previous implementation deleted ALL messages in the conversation,
  // which wipes out legitimate user chat history.
  // For now, we disable this automatic cleanup until we can implement a safer
  // version that only deletes messages with the "OSS:" prefix.
  println!("[Party Mode] Automatic message cleanup is currently disabled for safety.");
  
  /* 
  // Resolve conversation ID (may create one as a fallback, but deletion will work only if conversation exists)
  let conversation_id = get_conversation_id(app, lcu_connection, friend_summoner_id).await?;

  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages",
    lcu_connection.port, conversation_id
  );
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));

  println!("[DEBUG] Deleting messages at URL: {}", url);
  let response = client
    .delete(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
    .map_err(|e| format!("Failed to delete messages: {}", e))?;

  if !response.status().is_success() {
    return Err(format!("Failed to delete messages: {}", response.status()));
  }
  */

  Ok(())
}