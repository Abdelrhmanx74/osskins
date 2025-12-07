// Chat messaging and skin sharing

use super::lcu::get_conversation_id;
use super::types::{log_debug, log_error, log_info, LcuConnection, PARTY_MODE_MESSAGE_PREFIX};
use crate::commands::types::PartyModeMessage;
use base64::{engine::general_purpose, Engine};
use serde_json;
use tauri::AppHandle;

/// Send a chat message to a friend via the LCU API
pub async fn send_chat_message(
  app: &AppHandle,
  lcu_connection: &LcuConnection,
  friend_summoner_id: &str,
  message: &PartyModeMessage,
) -> Result<(), String> {
  log_debug(&format!(
    "send_chat_message called for friend_summoner_id: {}",
    friend_summoner_id
  ));

  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  // Get conversation ID with the friend
  log_debug("Getting conversation ID...");
  let conversation_id = get_conversation_id(app, lcu_connection, friend_summoner_id).await?;
  log_debug(&format!("Got conversation ID: {}", conversation_id));

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

  log_debug(&format!("Sending message to URL: {}", url));
  log_debug(&format!(
    "Message payload length: {} bytes",
    full_message.len()
  ));

  let response = client
    .post(&url)
    .header("Authorization", format!("Basic {}", auth))
    .header("Content-Type", "application/json")
    .json(&message_payload)
    .send()
    .await
    .map_err(|e| format!("Failed to send message: {}", e))?;

  let status = response.status();
  if !status.is_success() {
    let error_body = response.text().await.unwrap_or_default();
    log_error(&format!(
      "Message send failed with status {}: {}",
      status, error_body
    ));
    return Err(format!(
      "Failed to send message: {} - {}",
      status, error_body
    ));
  }

  log_info(&format!(
    "Message sent successfully to conversation {}",
    conversation_id
  ));
  Ok(())
}

/// Note on message cleanup:
///
/// The League client does NOT support deleting individual messages from conversations.
/// The DELETE endpoint at /lol-chat/v1/conversations/{id}/messages deletes ALL messages
/// in a conversation, which would wipe out legitimate user chat history.
///
/// Instead of trying to delete messages, we now use session-based filtering:
/// 1. When ChampSelect starts, we record the timestamp (CHAMP_SELECT_START_TIME_MS)
/// 2. When processing messages, we ignore any messages with timestamps before the session start
/// 3. We also ignore messages older than 5 minutes as an additional safety measure
///
/// This approach:
/// - Preserves user chat history
/// - Effectively ignores stale messages from previous game sessions
/// - Works even if the user restarts the app mid-session
///
/// The old delete_conversation_messages function is kept as a no-op for backwards compatibility.

/// Placeholder for conversation message cleanup (disabled for safety)
///
/// This function is intentionally a no-op. See the note above for details.
#[allow(dead_code)]
pub async fn delete_conversation_messages(
  _app: &tauri::AppHandle,
  _lcu_connection: &LcuConnection,
  friend_summoner_id: &str,
) -> Result<(), String> {
  log_debug(&format!(
    "delete_conversation_messages called for {} - no-op (using session filtering instead)",
    friend_summoner_id
  ));

  // Message cleanup is now handled via session-based timestamp filtering
  // in the message handler, not by deleting messages from the LCU.
  // This preserves user chat history while still ignoring stale party mode messages.

  Ok(())
}
