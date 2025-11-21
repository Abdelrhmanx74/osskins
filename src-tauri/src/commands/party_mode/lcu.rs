// LCU connection and API utilities

use base64::{engine::general_purpose, Engine};
use tauri::{AppHandle, Manager};
use crate::commands::types::{SavedConfig, FriendInfo};
use super::types::{LcuConnection, CurrentSummoner};

// Internal function to get LCU connection details
pub async fn get_lcu_connection(app: &AppHandle) -> Result<LcuConnection, String> {
  // Get the league path from config first
  let config_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data dir: {}", e))?
    .join("config");
  let config_file = config_dir.join("config.json");

  let mut search_dirs = vec![];

  // If we have a configured league path, use it (same logic as main watcher)
  if config_file.exists() {
    if let Ok(config_data) = std::fs::read_to_string(&config_file) {
      if let Ok(config) = serde_json::from_str::<SavedConfig>(&config_data) {
        if let Some(league_path) = config.league_path {
          search_dirs.push(std::path::PathBuf::from(league_path));
        }
      }
    }
  }

  // If no configured path, use default directories
  if search_dirs.is_empty() {
    search_dirs = vec![
      std::path::PathBuf::from("C:\\Riot Games\\League of Legends"),
      std::path::PathBuf::from("C:\\Program Files\\Riot Games\\League of Legends"),
      std::path::PathBuf::from("C:\\Program Files (x86)\\Riot Games\\League of Legends"),
    ];

    // Try to find running LeagueClient process dynamically
    #[cfg(target_os = "windows")]
    {
      use std::os::windows::process::CommandExt;
      const CREATE_NO_WINDOW: u32 = 0x08000000;
      
      if let Ok(output) = std::process::Command::new("wmic")
        .args(&["process", "where", "name='LeagueClient.exe'", "get", "ExecutablePath"])
        .creation_flags(CREATE_NO_WINDOW)
        .output() 
      {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
          let line = line.trim();
          if line.is_empty() || line.to_lowercase().contains("executablepath") {
            continue;
          }
          // line is the full path to exe, we need the directory
          if let Some(path) = std::path::PathBuf::from(line).parent() {
             search_dirs.push(path.to_path_buf());
          }
        }
      }
    }
  }

  // Search for lockfile in the directories
  for dir in &search_dirs {
    for name in [
      "lockfile",
      "LeagueClientUx.lockfile",
      "LeagueClient.lockfile",
    ] {
      let path = dir.join(name);
      if let Ok(content) = std::fs::read_to_string(&path) {
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() >= 5 {
          return Ok(LcuConnection {
            port: parts[2].to_string(),
            token: parts[3].to_string(),
          });
        }
      }
    }
  }

  Err("LCU lockfile not found".to_string())
}

// FUNCTION: get_current_summoner
pub async fn get_current_summoner(app: &AppHandle) -> Result<CurrentSummoner, String> {
  let lcu_connection = get_lcu_connection(app).await?;
  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let url = format!(
    "https://127.0.0.1:{}/lol-summoner/v1/current-summoner",
    lcu_connection.port
  );
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));

  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
    .map_err(|e| format!("Failed to get current summoner: {}", e))?;

  if !response.status().is_success() {
    return Err(format!(
      "Failed to get current summoner: {}",
      response.status()
    ));
  }

  let summoner_data: serde_json::Value = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse summoner data: {}", e))?;

  let summoner_id = summoner_data
    .get("summonerId")
    .and_then(|v| v.as_u64())
    .map(|id| id.to_string())
    .ok_or("Summoner ID not found")?;

  // Try multiple fields to get the display name
  let display_name = summoner_data
    .get("displayName")
    .and_then(|v| v.as_str())
    .unwrap_or("");

  // If displayName is empty, try gameName + gameTag
  let final_display_name = if display_name.is_empty() {
    let game_name = summoner_data
      .get("gameName")
      .and_then(|v| v.as_str())
      .unwrap_or("");
    let game_tag = summoner_data
      .get("gameTag")
      .and_then(|v| v.as_str())
      .unwrap_or("");

    if !game_name.is_empty() && !game_tag.is_empty() {
      format!("{}#{}", game_name, game_tag)
    } else if !game_name.is_empty() {
      game_name.to_string()
    } else {
      // If still empty, try summonerName or puuid fields
      summoner_data
        .get("summonerName")
        .and_then(|v| v.as_str())
        .or_else(|| summoner_data.get("name").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("User{}", summoner_id))
    }
  } else {
    display_name.to_string()
  };

  println!(
    "[DEBUG] Current summoner: ID={}, display_name={}",
    summoner_id, final_display_name
  );

  Ok(CurrentSummoner {
    summoner_id,
    display_name: final_display_name,
  })
}

// FUNCTION: get_friends_with_connection
pub async fn get_friends_with_connection(port: &str, token: &str) -> Result<Vec<FriendInfo>, String> {
  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let url = format!("https://127.0.0.1:{}/lol-chat/v1/friends", port);
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
    .map_err(|e| format!("Failed to get friends list: {}", e))?;

  if !response.status().is_success() {
    return Err(format!("LCU API returned error: {}", response.status()));
  }

  let friends_data: serde_json::Value = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse friends data: {}", e))?;

  let mut friends = Vec::new();
  if let Some(friends_array) = friends_data.as_array() {
    for friend in friends_array {
      // Parse the friend data correctly based on the LCU API structure
      let summoner_id = friend
        .get("summonerId")
        .and_then(|v| v.as_u64())
        .map(|id| id.to_string());

      let puuid = friend
        .get("puuid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

      let pid = friend
        .get("pid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

      let game_name = friend
        .get("gameName")
        .and_then(|v| v.as_str())
        .unwrap_or("");

      let game_tag = friend.get("gameTag").and_then(|v| v.as_str()).unwrap_or("");

      // Create display name from gameName#gameTag
      let display_name = if !game_name.is_empty() && !game_tag.is_empty() {
        format!("{}#{}", game_name, game_tag)
      } else if !game_name.is_empty() {
        game_name.to_string()
      } else {
        "Unknown".to_string()
      };

      if let Some(summoner_id) = summoner_id {
        if !puuid.is_empty() && !pid.is_empty() {
          let friend_info = FriendInfo {
            summoner_id,
            summoner_name: game_name.to_string(),
            display_name,
            is_online: friend
              .get("availability")
              .and_then(|v| v.as_str())
              .map_or(false, |s| s != "offline"),
            availability: friend
              .get("availability")
              .and_then(|v| v.as_str())
              .map(|s| s.to_string()),
            puuid,
            pid,
          };
          friends.push(friend_info);
        }
      }
    }
  }

  Ok(friends)
}

// FUNCTION: get_friend_display_name
pub async fn get_friend_display_name(
  app: &AppHandle,
  friend_summoner_id: &str,
) -> Result<String, String> {
  let lcu_connection = get_lcu_connection(app).await?;
  let friends = get_friends_with_connection(&lcu_connection.port, &lcu_connection.token).await?;

  for friend in friends {
    if friend.summoner_id == friend_summoner_id {
      return Ok(friend.display_name);
    }
  }

  Err(format!(
    "Friend with summoner ID {} not found",
    friend_summoner_id
  ))
}

// FUNCTION: get_conversation_id
pub async fn get_conversation_id(
  _app: &AppHandle,
  lcu_connection: &LcuConnection,
  friend_summoner_id: &str,
) -> Result<String, String> {
  println!(
    "[DEBUG] get_conversation_id called for friend_summoner_id: {}",
    friend_summoner_id
  );

  let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  // First, get the friend's PID from the friends list using the summoner ID
  let friends_url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/friends",
    lcu_connection.port
  );
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));

  println!("[DEBUG] Getting friends list from: {}", friends_url);
  let friends_response = client
    .get(&friends_url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
    .map_err(|e| format!("Failed to get friends: {}", e))?;

  if !friends_response.status().is_success() {
    return Err(format!(
      "Failed to get friends: {}",
      friends_response.status()
    ));
  }

  let friends_data: serde_json::Value = friends_response
    .json()
    .await
    .map_err(|e| format!("Failed to parse friends data: {}", e))?;

  println!(
    "[DEBUG] Got friends data, looking for friend with summoner_id: {}",
    friend_summoner_id
  );

  // Find the friend's PID by matching summoner ID
  let mut friend_pid = None;
  if let Some(friends_array) = friends_data.as_array() {
    println!("[DEBUG] Friends array has {} entries", friends_array.len());
    for (index, friend) in friends_array.iter().enumerate() {
      let summoner_id = friend
        .get("summonerId")
        .and_then(|v| v.as_u64())
        .map(|id| id.to_string());
      let pid = friend.get("pid").and_then(|v| v.as_str());
      let game_name = friend
        .get("gameName")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");

      println!(
        "[DEBUG] Friend {}: summoner_id={:?}, pid={:?}, gameName={}",
        index, summoner_id, pid, game_name
      );

      if let Some(sid) = summoner_id {
        if sid == friend_summoner_id {
          friend_pid = pid.map(|s| s.to_string());
          println!("[DEBUG] Found matching friend! PID: {:?}", friend_pid);
          break;
        }
      }
    }
  } else {
    println!("[DEBUG] Friends data is not an array!");
  }

  // If not found in friends list, try to extract PID from conversations where the summoner sent messages
  if friend_pid.is_none() {
    println!("[DEBUG] Friend not found in friends list, checking conversations for messages from this summoner...");

    // Get conversations and check messages to find the sender PID
    let conversations_url = format!(
      "https://127.0.0.1:{}/lol-chat/v1/conversations",
      lcu_connection.port
    );
    let conversations_response = client
      .get(&conversations_url)
      .header("Authorization", format!("Basic {}", auth))
      .send()
      .await
      .map_err(|e| format!("Failed to get conversations: {}", e))?;

    if conversations_response.status().is_success() {
      let conversations: serde_json::Value = conversations_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse conversations: {}", e))?;

      if let Some(conversations_array) = conversations.as_array() {
        for conversation in conversations_array {
          if let Some(conversation_id) = conversation.get("id").and_then(|i| i.as_str()) {
            // Check messages in this conversation
            let messages_url = format!(
              "https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages",
              lcu_connection.port, conversation_id
            );
            if let Ok(messages_response) = client
              .get(&messages_url)
              .header("Authorization", format!("Basic {}", auth))
              .send()
              .await
            {
              if let Ok(messages) = messages_response.json::<serde_json::Value>().await {
                if let Some(messages_array) = messages.as_array() {
                  for message in messages_array {
                    let from_id = message
                      .get("fromSummonerId")
                      .and_then(|id| id.as_str())
                      .or_else(|| message.get("fromId").and_then(|id| id.as_str()))
                      .or_else(|| message.get("senderId").and_then(|id| id.as_str()));

                    if let Some(from_id) = from_id {
                      if from_id == friend_summoner_id {
                        // Found a message from this summoner, get the conversation's PID
                        if let Some(pid) = conversation.get("pid").and_then(|p| p.as_str()) {
                          friend_pid = Some(pid.to_string());
                          println!(
                            "[DEBUG] Found friend PID from conversation messages: {}",
                            pid
                          );
                          break;
                        }
                      }
                    }
                  }
                }
              }
            }

            if friend_pid.is_some() {
              break;
            }
          }
        }
      }
    }
  }

  let friend_pid = friend_pid.ok_or_else(|| {
    format!(
      "Friend with summoner ID {} not found in friends list or conversations",
      friend_summoner_id
    )
  })?;
  println!("[DEBUG] Using friend_pid: {}", friend_pid);

  // Now get conversations and find the one with matching PID
  let conversations_url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/conversations",
    lcu_connection.port
  );

  println!("[DEBUG] Getting conversations from: {}", conversations_url);
  let conversations_response = client
    .get(&conversations_url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .await
    .map_err(|e| format!("Failed to get conversations: {}", e))?;

  if !conversations_response.status().is_success() {
    return Err(format!(
      "Failed to get conversations: {}",
      conversations_response.status()
    ));
  }

  let conversations: serde_json::Value = conversations_response
    .json()
    .await
    .map_err(|e| format!("Failed to parse conversations: {}", e))?;

  if let Some(conversations_array) = conversations.as_array() {
    println!(
      "[DEBUG] Conversations array has {} entries",
      conversations_array.len()
    );
    for (index, conversation) in conversations_array.iter().enumerate() {
      let conversation_pid = conversation.get("pid").and_then(|p| p.as_str());
      let conversation_id = conversation.get("id").and_then(|i| i.as_str());
      let conversation_type = conversation
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

      println!(
        "[DEBUG] Conversation {}: id={:?}, pid={:?}, type={}",
        index, conversation_id, conversation_pid, conversation_type
      );

      if let Some(pid) = conversation_pid {
        if pid == friend_pid {
          if let Some(id) = conversation_id {
            println!("[DEBUG] Found matching conversation! ID: {}", id);
            return Ok(id.to_string());
          }
        }
      }
    }
  } else {
    println!("[DEBUG] Conversations data is not an array!");
  }

  // If no existing conversation found, try multiple approaches to create/find one
  println!(
    "[DEBUG] No existing conversation found, trying to create new conversation with PID: {}",
    friend_pid
  );

  // First try: Use the standard LCU API
  let create_conversation_url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/conversations",
    lcu_connection.port
  );
  let create_payload = serde_json::json!({
      "type": "chat",
      "pid": friend_pid
  });

  println!(
    "[DEBUG] Creating conversation with payload: {}",
    create_payload
  );
  let create_response = client
    .post(&create_conversation_url)
    .header("Authorization", format!("Basic {}", auth))
    .header("Content-Type", "application/json")
    .json(&create_payload)
    .send()
    .await
    .map_err(|e| format!("Failed to create conversation: {}", e))?;

  let status = create_response.status();
  if status.is_success() {
    let created_conversation: serde_json::Value = create_response
      .json()
      .await
      .map_err(|e| format!("Failed to parse created conversation: {}", e))?;

    if let Some(conversation_id) = created_conversation.get("id").and_then(|i| i.as_str()) {
      println!(
        "[DEBUG] Successfully created conversation with ID: {}",
        conversation_id
      );
      return Ok(conversation_id.to_string());
    }
  } else {
    let error_text = create_response
      .text()
      .await
      .unwrap_or_else(|_| "Unknown error".to_string());
    println!(
      "[DEBUG] Failed to create conversation with v1 API: {} - {}",
      status, error_text
    );

    // If conversation creation failed, try a different approach
    // Sometimes we can use the friend's PID directly as conversation ID
    println!("[DEBUG] Trying fallback approach using PID as conversation ID...");

    // First, try sending a test message to see if PID works as conversation ID
    let test_conversation_id = friend_pid.clone();

    // Try to get conversation info using PID as ID
    let test_conversation_url = format!(
      "https://127.0.0.1:{}/lol-chat/v1/conversations/{}",
      lcu_connection.port, test_conversation_id
    );
    let test_response = client
      .get(&test_conversation_url)
      .header("Authorization", format!("Basic {}", auth))
      .send()
      .await;

    if let Ok(response) = test_response {
      if response.status().is_success() {
        println!(
          "[DEBUG] PID works as conversation ID: {}",
          test_conversation_id
        );
        return Ok(test_conversation_id);
      }
    }

    // If that doesn't work, try using summoner ID directly
    println!("[DEBUG] Trying summoner ID as conversation ID...");
    return Ok(friend_summoner_id.to_string());
  }

  Err(format!(
    "Failed to get conversation ID from created conversation"
  ))
}