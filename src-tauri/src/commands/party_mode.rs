use tauri::{AppHandle, Emitter, Manager};
use serde_json;
use base64::{Engine, engine::general_purpose};
use std::path::PathBuf;
use crate::commands::types::{
    SavedConfig, PartyModeConfig, PairedFriend, FriendInfo, ConnectionRequest, 
    PartyModeMessage, PairingRequest, PairingResponse, SkinShare, SentPairingRequest
};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct InMemoryReceivedSkin {
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome_path: Option<String>,
    pub received_at: u64,
}

// Global in-memory map for received skins (key: summoner+champion)
pub static RECEIVED_SKINS: Lazy<Mutex<HashMap<String, InMemoryReceivedSkin>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Helper to generate key
fn received_skin_key(from_summoner_id: &str, champion_id: u32) -> String {
    format!("{}_{}", from_summoner_id, champion_id)
}

const PARTY_MODE_MESSAGE_PREFIX: &str = "OSS:";

// Tauri command to get friends list from LCU
#[tauri::command]
pub async fn get_lcu_friends(app: AppHandle) -> Result<Vec<FriendInfo>, String> {
    let lcu_connection = get_lcu_connection(&app).await?;
    return get_friends_with_connection(&lcu_connection.port, &lcu_connection.token).await;
}

// Helper function to get friends with existing connection info
async fn get_friends_with_connection(port: &str, token: &str) -> Result<Vec<FriendInfo>, String> {
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
            
            let game_tag = friend
                .get("gameTag")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
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

// Tauri command to send pairing request
#[tauri::command]
pub async fn send_pairing_request(app: AppHandle, friend_summoner_id: String) -> Result<String, String> {
    println!("[DEBUG] Starting send_pairing_request for friend_summoner_id: {}", friend_summoner_id);
    
    // Check if there's already a pending request to this user
    if is_request_already_sent(&app, &friend_summoner_id).await? {
        return Err("Request already sent to this user".to_string());
    }
    
    let lcu_connection = get_lcu_connection(&app).await?;
    println!("[DEBUG] Got LCU connection - port: {}", lcu_connection.port);
    
    let current_summoner = get_current_summoner(&app).await?;
    println!("[DEBUG] Got current summoner - id: {}, name: '{}'", current_summoner.summoner_id, current_summoner.display_name);
    
    // Ensure we have a valid display name
    let display_name = if current_summoner.display_name.is_empty() || current_summoner.display_name == "Unknown" {
        println!("[DEBUG] Display name is empty or unknown, trying to get it from friends list...");
        // Try to get our own name from the friends list or use a fallback
        format!("User{}", current_summoner.summoner_id)
    } else {
        current_summoner.display_name.clone()
    };
    
    println!("[DEBUG] Using display name: '{}'", display_name);
    
    let request_id = uuid::Uuid::new_v4().to_string();
    let pairing_request = PairingRequest {
        request_id: request_id.clone(),
        from_summoner_id: current_summoner.summoner_id,
        from_summoner_name: display_name,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let message = PartyModeMessage {
        message_type: "pairing_request".to_string(),
        data: serde_json::to_value(pairing_request)
            .map_err(|e| format!("Failed to serialize pairing request: {}", e))?,
    };
    
    println!("[DEBUG] About to send chat message with data: {:?}", serde_json::to_string(&message).unwrap_or_else(|_| "Failed to serialize".to_string()));
    
    // Store the sent request before sending
    store_sent_request(&app, &request_id, &friend_summoner_id).await?;
    
    send_chat_message(&app, &lcu_connection, &friend_summoner_id, &message).await?;
    println!("[DEBUG] Successfully sent pairing request!");
    Ok(request_id)
}// Tauri command to respond to pairing request
#[tauri::command]
pub async fn respond_to_pairing_request(
    app: AppHandle,
    request_id: String,
    friend_summoner_id: String,
    accepted: bool,
) -> Result<(), String> {
    let lcu_connection = get_lcu_connection(&app).await?;
    let current_summoner = get_current_summoner(&app).await?;

    let pairing_response = PairingResponse {
        request_id: request_id.clone(),
        accepted,
        from_summoner_id: current_summoner.summoner_id,
        from_summoner_name: current_summoner.display_name,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let message = PartyModeMessage {
        message_type: "pairing_response".to_string(),
        data: serde_json::to_value(pairing_response)
            .map_err(|e| format!("Failed to serialize pairing response: {}", e))?,
    };

    send_chat_message(&app, &lcu_connection, &friend_summoner_id, &message).await?;

    // If declined, add the request ID and summoner to ignored lists
    if !accepted {
        add_ignored_request(&app, &request_id, &friend_summoner_id).await?;
    }

    // If accepted, add to paired friends
    if accepted {
        // Try to get the friend's proper display name instead of using "Unknown"
        let friend_display_name = get_friend_display_name(&app, &friend_summoner_id).await
            .unwrap_or_else(|_| format!("User {}", friend_summoner_id));
        
        add_paired_friend(&app, &friend_summoner_id, &friend_display_name).await?;
    }

    Ok(())
}

// Tauri command to remove paired friend
#[tauri::command]
pub async fn remove_paired_friend(app: AppHandle, friend_summoner_id: String) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let mut config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    config.party_mode.paired_friends.retain(|f| f.summoner_id != friend_summoner_id);

    // Also add this summoner to ignored list to prevent future automatic pairing
    if !config.party_mode.ignored_summoners.contains(&friend_summoner_id) {
        config.party_mode.ignored_summoners.push(friend_summoner_id);
    }

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    // Emit event to update UI components
    let _ = app.emit("party-mode-paired-friends-updated", ());

    Ok(())
}

// Tauri command to get paired friends
#[tauri::command]
pub async fn get_paired_friends(app: AppHandle) -> Result<Vec<PairedFriend>, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    println!("[Party Mode] Loading paired friends from: {:?}", config_file);

    if !config_file.exists() {
        println!("[Party Mode] Config file does not exist, returning empty list");
        return Ok(Vec::new());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    println!("[Party Mode] Loaded {} paired friends from config", config.party_mode.paired_friends.len());
    for friend in &config.party_mode.paired_friends {
        println!("[Party Mode] - Friend: {} ({})", friend.display_name, friend.summoner_id);
    }

    Ok(config.party_mode.paired_friends)
}

// Tauri command to get sent pairing requests
#[tauri::command]
pub async fn get_sent_requests(app: AppHandle) -> Result<std::collections::HashMap<String, SentPairingRequest>, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(config.party_mode.sent_requests)
}

#[tauri::command]
pub async fn get_party_mode_settings(app: AppHandle) -> Result<(bool, bool), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok((true, true)); // Default values
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok((config.party_mode.auto_share, config.party_mode.notifications))
}

#[tauri::command]
pub async fn update_party_mode_settings(
    app: AppHandle,
    auto_share: bool,
    notifications: bool,
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let mut config = if config_file.exists() {
        let config_data = std::fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        SavedConfig {
            league_path: None,
            skins: Vec::new(),
            favorites: Vec::new(),
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
        }
    };

    config.party_mode.auto_share = auto_share;
    config.party_mode.notifications = notifications;

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Tauri command to clear ignored summoners (allow re-pairing)
#[tauri::command]
pub async fn clear_ignored_summoner(app: AppHandle, summoner_id: String) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let mut config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Remove summoner from ignored list
    config.party_mode.ignored_summoners.retain(|s| s != &summoner_id);

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Internal function to send chat message
async fn send_chat_message(
    app: &AppHandle,
    lcu_connection: &LcuConnection,
    friend_summoner_id: &str,
    message: &PartyModeMessage,
) -> Result<(), String> {
    println!("[DEBUG] send_chat_message called for friend_summoner_id: {}", friend_summoner_id);
    
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Get conversation ID with the friend
    println!("[DEBUG] Getting conversation ID...");
    let conversation_id = get_conversation_id(app, lcu_connection, friend_summoner_id).await?;
    println!("[DEBUG] Got conversation ID: {}", conversation_id);

    let message_json = serde_json::to_string(message)
        .map_err(|e| format!("Failed to serialize message: {}", e))?;
    
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

// Internal function to get conversation ID
async fn get_conversation_id(
    _app: &AppHandle,
    lcu_connection: &LcuConnection,
    friend_summoner_id: &str,
) -> Result<String, String> {
    println!("[DEBUG] get_conversation_id called for friend_summoner_id: {}", friend_summoner_id);
    
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // First, get the friend's PID from the friends list using the summoner ID
    let friends_url = format!("https://127.0.0.1:{}/lol-chat/v1/friends", lcu_connection.port);
    let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));

    println!("[DEBUG] Getting friends list from: {}", friends_url);
    let friends_response = client
        .get(&friends_url)
        .header("Authorization", format!("Basic {}", auth))
        .send()
        .await
        .map_err(|e| format!("Failed to get friends: {}", e))?;

    if !friends_response.status().is_success() {
        return Err(format!("Failed to get friends: {}", friends_response.status()));
    }

    let friends_data: serde_json::Value = friends_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse friends data: {}", e))?;

    println!("[DEBUG] Got friends data, looking for friend with summoner_id: {}", friend_summoner_id);

    // Find the friend's PID by matching summoner ID
    let mut friend_pid = None;
    if let Some(friends_array) = friends_data.as_array() {
        println!("[DEBUG] Friends array has {} entries", friends_array.len());
        for (index, friend) in friends_array.iter().enumerate() {
            let summoner_id = friend.get("summonerId").and_then(|v| v.as_u64()).map(|id| id.to_string());
            let pid = friend.get("pid").and_then(|v| v.as_str());
            let game_name = friend.get("gameName").and_then(|v| v.as_str()).unwrap_or("N/A");
            
            println!("[DEBUG] Friend {}: summoner_id={:?}, pid={:?}, gameName={}", 
                index, summoner_id, pid, game_name);
            
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
        let conversations_url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations", lcu_connection.port);
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
                        let messages_url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages", lcu_connection.port, conversation_id);
                        if let Ok(messages_response) = client
                            .get(&messages_url)
                            .header("Authorization", format!("Basic {}", auth))
                            .send()
                            .await {
                            
                            if let Ok(messages) = messages_response.json::<serde_json::Value>().await {
                                if let Some(messages_array) = messages.as_array() {
                                    for message in messages_array {
                                        let from_id = message.get("fromSummonerId")
                                            .and_then(|id| id.as_str())
                                            .or_else(|| message.get("fromId").and_then(|id| id.as_str()))
                                            .or_else(|| message.get("senderId").and_then(|id| id.as_str()));
                                        
                                        if let Some(from_id) = from_id {
                                            if from_id == friend_summoner_id {
                                                // Found a message from this summoner, get the conversation's PID
                                                if let Some(pid) = conversation.get("pid").and_then(|p| p.as_str()) {
                                                    friend_pid = Some(pid.to_string());
                                                    println!("[DEBUG] Found friend PID from conversation messages: {}", pid);
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

    let friend_pid = friend_pid.ok_or_else(|| format!("Friend with summoner ID {} not found in friends list or conversations", friend_summoner_id))?;
    println!("[DEBUG] Using friend_pid: {}", friend_pid);

    // Now get conversations and find the one with matching PID
    let conversations_url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations", lcu_connection.port);

    println!("[DEBUG] Getting conversations from: {}", conversations_url);
    let conversations_response = client
        .get(&conversations_url)
        .header("Authorization", format!("Basic {}", auth))
        .send()
        .await
        .map_err(|e| format!("Failed to get conversations: {}", e))?;

    if !conversations_response.status().is_success() {
        return Err(format!("Failed to get conversations: {}", conversations_response.status()));
    }

    let conversations: serde_json::Value = conversations_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse conversations: {}", e))?;

    if let Some(conversations_array) = conversations.as_array() {
        println!("[DEBUG] Conversations array has {} entries", conversations_array.len());
        for (index, conversation) in conversations_array.iter().enumerate() {
            let conversation_pid = conversation.get("pid").and_then(|p| p.as_str());
            let conversation_id = conversation.get("id").and_then(|i| i.as_str());
            let conversation_type = conversation.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
            
            println!("[DEBUG] Conversation {}: id={:?}, pid={:?}, type={}", 
                index, conversation_id, conversation_pid, conversation_type);
            
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
    println!("[DEBUG] No existing conversation found, trying to create new conversation with PID: {}", friend_pid);
    
    // First try: Use the standard LCU API
    let create_conversation_url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations", lcu_connection.port);
    let create_payload = serde_json::json!({
        "type": "chat",
        "pid": friend_pid
    });

    println!("[DEBUG] Creating conversation with payload: {}", create_payload);
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
            println!("[DEBUG] Successfully created conversation with ID: {}", conversation_id);
            return Ok(conversation_id.to_string());
        }
    } else {
        let error_text = create_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        println!("[DEBUG] Failed to create conversation with v1 API: {} - {}", status, error_text);
        
        // If conversation creation failed, try a different approach
        // Sometimes we can use the friend's PID directly as conversation ID
        println!("[DEBUG] Trying fallback approach using PID as conversation ID...");
        
        // First, try sending a test message to see if PID works as conversation ID
        let test_conversation_id = friend_pid.clone();
        
        // Try to get conversation info using PID as ID
        let test_conversation_url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations/{}", lcu_connection.port, test_conversation_id);
        let test_response = client
            .get(&test_conversation_url)
            .header("Authorization", format!("Basic {}", auth))
            .send()
            .await;
        
        if let Ok(response) = test_response {
            if response.status().is_success() {
                println!("[DEBUG] PID works as conversation ID: {}", test_conversation_id);
                return Ok(test_conversation_id);
            }
        }
        
        // If that doesn't work, try using summoner ID directly
        println!("[DEBUG] Trying summoner ID as conversation ID...");
        return Ok(friend_summoner_id.to_string());
    }

    Err(format!("Failed to get conversation ID from created conversation"))
}

// Internal function to add ignored request
async fn add_ignored_request(
    app: &AppHandle,
    request_id: &str,
    friend_summoner_id: &str,
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let mut config = if config_file.exists() {
        let config_data = std::fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        SavedConfig {
            league_path: None,
            skins: Vec::new(),
            favorites: Vec::new(),
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
        }
    };

    // Add request ID to ignored list
    if !config.party_mode.ignored_request_ids.contains(&request_id.to_string()) {
        config.party_mode.ignored_request_ids.push(request_id.to_string());
    }

    // Also add summoner to ignored list temporarily (can be removed manually later)
    if !config.party_mode.ignored_summoners.contains(&friend_summoner_id.to_string()) {
        config.party_mode.ignored_summoners.push(friend_summoner_id.to_string());
    }

    // Keep only the last 100 ignored request IDs to prevent config file from growing too large
    if config.party_mode.ignored_request_ids.len() > 100 {
        let current_len = config.party_mode.ignored_request_ids.len();
        config.party_mode.ignored_request_ids = config.party_mode.ignored_request_ids
            .into_iter()
            .skip(current_len - 50)
            .collect();
    }

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Internal function to check if a request is already sent to a user
async fn is_request_already_sent(
    app: &AppHandle,
    friend_summoner_id: &str,
) -> Result<bool, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(false);
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Check if there's a pending request to this summoner
    Ok(config.party_mode.sent_requests.contains_key(friend_summoner_id))
}

// Internal function to store a sent pairing request
async fn store_sent_request(
    app: &AppHandle,
    request_id: &str,
    friend_summoner_id: &str,
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let mut config = if config_file.exists() {
        let config_data = std::fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        SavedConfig {
            league_path: None,
            skins: Vec::new(),
            favorites: Vec::new(),
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
        }
    };

    // Get friend display name for storage
    let friend_display_name = get_friend_display_name(app, friend_summoner_id).await
        .unwrap_or_else(|_| format!("User {}", friend_summoner_id));

    // Store the sent request
    let sent_request = SentPairingRequest {
        request_id: request_id.to_string(),
        to_summoner_id: friend_summoner_id.to_string(),
        to_summoner_name: friend_display_name,
        sent_at: chrono::Utc::now().timestamp_millis() as u64,
    };

    config.party_mode.sent_requests.insert(friend_summoner_id.to_string(), sent_request);

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Internal function to remove sent request (on response or cleanup)
async fn remove_sent_request(
    app: &AppHandle,
    friend_summoner_id: &str,
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let mut config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Remove the sent request
    config.party_mode.sent_requests.remove(friend_summoner_id);

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Internal function to check if a request should be ignored
async fn is_request_ignored(
    app: &AppHandle,
    request_id: &str,
    summoner_id: &str,
) -> Result<bool, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(false);
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Check if request ID is in ignored list
    if config.party_mode.ignored_request_ids.contains(&request_id.to_string()) {
        return Ok(true);
    }

    // Check if summoner is in ignored list
    if config.party_mode.ignored_summoners.contains(&summoner_id.to_string()) {
        return Ok(true);
    }

    Ok(false)
}

// Internal function to add paired friend
async fn add_paired_friend(
    app: &AppHandle,
    friend_summoner_id: &str,
    friend_name: &str,
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let mut config = if config_file.exists() {
        let config_data = std::fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        SavedConfig {
            league_path: None,
            skins: Vec::new(),
            favorites: Vec::new(),
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
        }
    };

    // Check if friend is already paired
    if !config.party_mode.paired_friends.iter().any(|f| f.summoner_id == friend_summoner_id) {
        config.party_mode.paired_friends.push(PairedFriend {
            summoner_id: friend_summoner_id.to_string(),
            summoner_name: friend_name.to_string(),
            display_name: friend_name.to_string(),
            paired_at: chrono::Utc::now().timestamp_millis() as u64,
        });

        let updated_config = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&config_file, updated_config)
            .map_err(|e| format!("Failed to save config: {}", e))?;

        // Emit event to update UI components
        let _ = app.emit("party-mode-paired-friends-updated", ());
    }

    Ok(())
}

// Helper structs
struct LcuConnection {
    port: String,
    token: String,
}

struct CurrentSummoner {
    summoner_id: String,
    display_name: String,
}

// Internal function to get LCU connection details
async fn get_lcu_connection(app: &AppHandle) -> Result<LcuConnection, String> {
    // Get the league path from config first
    let config_dir = app.path().app_data_dir()
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
    }
    
    // Search for lockfile in the directories
    for dir in &search_dirs {
        for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
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

// Internal function to get current summoner info
async fn get_current_summoner(app: &AppHandle) -> Result<CurrentSummoner, String> {
    let lcu_connection = get_lcu_connection(app).await?;
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!("https://127.0.0.1:{}/lol-summoner/v1/current-summoner", lcu_connection.port);
    let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu_connection.token));

    let response = client
        .get(&url)
        .header("Authorization", format!("Basic {}", auth))
        .send()
        .await
        .map_err(|e| format!("Failed to get current summoner: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to get current summoner: {}", response.status()));
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

    println!("[DEBUG] Current summoner: ID={}, display_name={}", summoner_id, final_display_name);

    Ok(CurrentSummoner {
        summoner_id,
        display_name: final_display_name,
    })
}

// Internal function to get friend display name by summoner ID
async fn get_friend_display_name(app: &AppHandle, friend_summoner_id: &str) -> Result<String, String> {
    let lcu_connection = get_lcu_connection(app).await?;
    let friends = get_friends_with_connection(&lcu_connection.port, &lcu_connection.token).await?;
    
    for friend in friends {
        if friend.summoner_id == friend_summoner_id {
            return Ok(friend.display_name);
        }
    }
    
    Err(format!("Friend with summoner ID {} not found", friend_summoner_id))
}

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
    let message: PartyModeMessage = serde_json::from_str(message_json)
        .map_err(|e| format!("Failed to parse party mode message: {}", e))?;

    // Get current user's summoner ID to filter out self-sent messages
    let current_summoner = match get_current_summoner(app).await {
        Ok(summoner) => summoner,
        Err(e) => {
            println!("[Party Mode] Warning: Could not get current summoner, proceeding anyway: {}", e);
            CurrentSummoner {
                summoner_id: "unknown".to_string(),
                display_name: "Unknown".to_string(),
            }
        }
    };

    match message.message_type.as_str() {
        "pairing_request" => {
            let request: PairingRequest = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse pairing request: {}", e))?;
            
            // Don't show requests that the current user sent
            if request.from_summoner_id == current_summoner.summoner_id {
                println!("[Party Mode] Ignoring pairing request from self ({})", current_summoner.summoner_id);
                return Ok(());
            }
            
            // Check if this request or summoner is in the ignored list
            if is_request_ignored(app, &request.request_id, &request.from_summoner_id).await? {
                println!("[Party Mode] Ignoring previously declined request {} from summoner {}", request.request_id, request.from_summoner_id);
                return Ok(());
            }
            
            // Get the friend's display name from the friends list if the message doesn't have it
            let friend_display_name = if request.from_summoner_name.is_empty() {
                get_friend_display_name(app, &request.from_summoner_id).await
                    .unwrap_or_else(|_| format!("User {}", request.from_summoner_id))
            } else {
                request.from_summoner_name.clone()
            };
            
            let connection_request = ConnectionRequest {
                from_summoner_id: request.from_summoner_id.clone(),
                from_summoner_name: friend_display_name.clone(),
                timestamp: request.timestamp,
            };

            println!("[Party Mode] Emitting connection request event for: {}", friend_display_name);
            match app.emit("party-mode-connection-request", &connection_request) {
                Ok(_) => println!("[Party Mode] Successfully emitted connection request event"),
                Err(e) => eprintln!("[Party Mode] Failed to emit connection request event: {}", e),
            }
        }
        "pairing_response" => {
            let response: PairingResponse = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse pairing response: {}", e))?;
            
            // Don't process responses from the current user (self-sent)
            if response.from_summoner_id == current_summoner.summoner_id {
                println!("[Party Mode] Ignoring pairing response from self ({})", current_summoner.summoner_id);
                return Ok(());
            }
            
            // Remove the sent request since we got a response
            if let Err(e) = remove_sent_request(app, &response.from_summoner_id).await {
                println!("[Party Mode] Warning: Failed to remove sent request: {}", e);
            }
            
            if response.accepted {
                // Get the friend's proper display name for storing
                let friend_display_name = if response.from_summoner_name.is_empty() {
                    get_friend_display_name(app, &response.from_summoner_id).await
                        .unwrap_or_else(|_| format!("User {}", response.from_summoner_id))
                } else {
                    response.from_summoner_name.clone()
                };
                
                add_paired_friend(app, &response.from_summoner_id, &friend_display_name).await?;
                let _ = app.emit("party-mode-pairing-accepted", response);
            } else {
                let _ = app.emit("party-mode-pairing-declined", response);
            }
        }
        "skin_share" => {
            let skin_share: SkinShare = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse skin share: {}", e))?;
            // Only process skin_share if in ChampSelect phase
            if !crate::commands::lcu_watcher::is_in_champ_select() {
                println!("[Party Mode] Ignoring skin_share from {} for champion {} because not in ChampSelect phase", 
                    skin_share.from_summoner_name, skin_share.champion_id);
                return Ok(());
            }
            println!("[Party Mode] Processing skin share from {}: Champion {}, Skin {}, Skin Name: {}",
                skin_share.from_summoner_name,
                skin_share.champion_id,
                skin_share.skin_id,
                skin_share.skin_name);
            // Store received skin data in memory only
            let key = received_skin_key(&skin_share.from_summoner_id, skin_share.champion_id);
            let mut map = RECEIVED_SKINS.lock().unwrap();
            map.insert(key, InMemoryReceivedSkin {
                from_summoner_id: skin_share.from_summoner_id.clone(),
                from_summoner_name: skin_share.from_summoner_name.clone(),
                champion_id: skin_share.champion_id,
                skin_id: skin_share.skin_id,
                chroma_id: skin_share.chroma_id,
                fantome_path: skin_share.fantome_path.clone(),
                received_at: skin_share.timestamp,
            });
            println!("[Party Mode] Received skin from {} for champion {} (stored in memory for this session only)", 
                     skin_share.from_summoner_name, skin_share.champion_id);
            let _ = app.emit("party-mode-skin-received", skin_share);
        }
        _ => {
            // Unknown message type, ignore
        }
    }

    Ok(())
}

// Function to send skin share to paired friends (called from LCU watcher on champion lock)
pub async fn send_skin_share_to_paired_friends(
    app: &AppHandle,
    champion_id: u32,
    skin_id: u32,
    chroma_id: Option<u32>,
    fantome_path: Option<String>,
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    if !config.party_mode.auto_share || config.party_mode.paired_friends.is_empty() {
        return Ok(());
    }

    let lcu_connection = get_lcu_connection(app).await?;
    let current_summoner = get_current_summoner(app).await?;

    // Try to get the actual skin name from the skin data if possible
    let skin_name = get_skin_name_from_config(app, champion_id, skin_id).unwrap_or_else(|| format!("Skin {}", skin_id));
    
    let skin_share = SkinShare {
        from_summoner_id: current_summoner.summoner_id,
        from_summoner_name: current_summoner.display_name,
        champion_id,
        skin_id,
        skin_name: skin_name.clone(),
        chroma_id,
        fantome_path,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let message = PartyModeMessage {
        message_type: "skin_share".to_string(),
        data: serde_json::to_value(skin_share)
            .map_err(|e| format!("Failed to serialize skin share: {}", e))?,
    };

    for friend in &config.party_mode.paired_friends {
        if let Err(e) = send_chat_message(app, &lcu_connection, &friend.summoner_id, &message).await {
            eprintln!("Failed to send skin share to {}: {}", friend.summoner_name, e);
        } else {
            println!("[Party Mode] Successfully sent skin share to {}: Champion {}, Skin {}", 
                     friend.summoner_name, champion_id, skin_name);
            
            // Emit event for UI to show toast notification
            let _ = app.emit("party-mode-skin-sent", serde_json::json!({
                "to_friend": friend.summoner_name,
                "champion_id": champion_id,
                "skin_name": skin_name
            }));
        }
    }

    Ok(())
}

// Helper function to get skin name from config
pub fn get_skin_name_from_config(app: &AppHandle, champion_id: u32, skin_id: u32) -> Option<String> {
    // Try to read the config to get skin names
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");
    
    if let Ok(config_data) = std::fs::read_to_string(&config_file) {
        if let Ok(config) = serde_json::from_str::<SavedConfig>(&config_data) {
            // Look for this specific skin in the user's selected skins
            for skin in &config.skins {
                if skin.champion_id == champion_id && skin.skin_id == skin_id {
                    // Try to extract skin name from fantome path if available
                    if let Some(ref fantome_path) = skin.fantome {
                        if let Some(file_name) = std::path::Path::new(fantome_path).file_stem() {
                            if let Some(name_str) = file_name.to_str() {
                                return Some(name_str.to_string());
                            }
                        }
                    }
                    // Fallback to a generic name
                    return Some(format!("Champion {} Skin {}", champion_id, skin_id));
                }
            }
        }
    }
    
    None
}

// Helper function to check if all paired friends have shared their skins for a specific champion
pub async fn should_inject_now(app: &AppHandle, champion_id: u32) -> Result<bool, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(true); // No config means no friends, so inject immediately
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // If no paired friends, inject immediately
    if config.party_mode.paired_friends.is_empty() {
        println!("[Party Mode] No paired friends - injecting immediately for champion {}", champion_id);
        return Ok(true);
    }

    // Check if we have received skins from all paired friends for this champion
    let friends_who_shared: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    // This section is no longer needed as skins are stored in memory
    // for (_key, received_skin) in &config.party_mode.received_skins {
    //     if received_skin.champion_id == champion_id {
    //         friends_who_shared.insert(&received_skin.from_summoner_id);
            
    //         // Track the earliest received time for this champion (approximates champion select start)
    //         if champion_select_start_time.is_none() || received_skin.received_at < champion_select_start_time.unwrap() {
    //             champion_select_start_time = Some(received_skin.received_at);
    //         }
    //     }
    // }

    // Count how many paired friends we have vs how many shared
    let total_friends = config.party_mode.paired_friends.len();
    let friends_shared = friends_who_shared.len();

    println!("[Party Mode] Injection timing check for champion {}: {}/{} friends have shared skins", 
             champion_id, friends_shared, total_friends);

    // List which friends have shared and which haven't
    for friend in &config.party_mode.paired_friends {
        let has_shared = friends_who_shared.contains(&friend.summoner_id);
        println!("[Party Mode] Friend {} ({}): {}", 
                 friend.display_name, 
                 friend.summoner_id,
                 if has_shared { "✅ shared skin" } else { "⏳ waiting for skin" });
    }

    // Wait for ALL friends to share their skins before injecting
    let should_inject = friends_shared == total_friends;
    
    if should_inject {
        println!("[Party Mode] ✅ All friends have shared - proceeding with injection for champion {}", champion_id);
    } else {
        println!("[Party Mode] ⏳ Waiting for {} more friends to share their skins for champion {}", 
                 total_friends - friends_shared, champion_id);
    }

    Ok(should_inject)
}

// Add a function to clear received skins (call this when leaving champ select or starting a new session)
pub fn clear_received_skins() {
    let mut map = RECEIVED_SKINS.lock().unwrap();
    map.clear();
}
