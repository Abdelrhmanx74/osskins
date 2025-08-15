use tauri::{AppHandle, Emitter, Manager};
use serde_json;
use base64::{Engine, engine::general_purpose};
use std::path::PathBuf;
use crate::commands::types::{
    PartyModeMessage, SkinShare, FriendInfo, PairedFriend, SavedConfig, PartyModeConfig
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

// Tauri command to add a friend directly to party mode
#[tauri::command]
pub async fn add_party_friend(app: AppHandle, friend_summoner_id: String) -> Result<String, String> {
    println!("[DEBUG] Adding friend to party mode: {}", friend_summoner_id);
    
    // Get friend display name from LCU
    let friend_display_name = get_friend_display_name(&app, &friend_summoner_id).await
        .unwrap_or_else(|_| format!("User {}", friend_summoner_id));
    
    // Add to paired friends with sharing enabled by default - NO CHAT MESSAGE SENT
    add_paired_friend(&app, &friend_summoner_id, &friend_display_name, true).await?;
    
    println!("[DEBUG] Successfully added friend to party mode silently!");
    Ok(friend_summoner_id)
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
        println!("[Party Mode] - Friend: {} ({}) - Sharing: {}", 
                 friend.display_name, friend.summoner_id, friend.share_enabled);
    }

    Ok(config.party_mode.paired_friends)
}

#[tauri::command]
pub async fn get_party_mode_settings(app: AppHandle) -> Result<bool, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(true); // Default notifications enabled
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(config.party_mode.notifications)
}

#[tauri::command]
pub async fn update_party_mode_settings(
    app: AppHandle,
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
        serde_json::from_str::<SavedConfig>(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        SavedConfig {
            league_path: None,
            skins: Vec::new(),
            custom_skins: Vec::new(),
            favorites: Vec::new(),
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
            auto_update_data: true,
            last_data_commit: None,
        }
    };

    config.party_mode.notifications = notifications;

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

// Internal function to add paired friend
async fn add_paired_friend(
    app: &AppHandle,
    friend_summoner_id: &str,
    friend_name: &str,
    share_enabled: bool,
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
        serde_json::from_str::<SavedConfig>(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        SavedConfig {
            league_path: None,
            skins: Vec::new(),
            custom_skins: Vec::new(),
            favorites: Vec::new(),
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
            auto_update_data: true,
            last_data_commit: None,
        }
    };

    // Check if friend is already paired
    if !config.party_mode.paired_friends.iter().any(|f| f.summoner_id == friend_summoner_id) {
        config.party_mode.paired_friends.push(PairedFriend {
            summoner_id: friend_summoner_id.to_string(),
            summoner_name: friend_name.to_string(),
            display_name: friend_name.to_string(),
            paired_at: chrono::Utc::now().timestamp_millis() as u64,
            share_enabled,
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
        "skin_share" => {
            let skin_share: SkinShare = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse skin share: {}", e))?;
            
            // Filter out messages from the current user (don't process your own skin shares)
            if skin_share.from_summoner_id == current_summoner.summoner_id {
                println!("[Party Mode] Ignoring skin_share from self ({})", current_summoner.display_name);
                return Ok(());
            }
            
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
            
            // Drop the lock before emitting the event
            drop(map);
            
            // Emit the skin received event
            let _ = app.emit("party-mode-skin-received", skin_share.clone());
            // Also emit a chat log event for dashboard visibility
            let _ = app.emit("party-mode-chat-log", serde_json::json!({
                "direction": "received",
                "from": skin_share.from_summoner_name,
                "from_id": skin_share.from_summoner_id,
                "champion_id": skin_share.champion_id,
                "skin_id": skin_share.skin_id,
                "skin_name": skin_share.skin_name,
                "chroma_id": skin_share.chroma_id,
                "fantome_path": skin_share.fantome_path,
                "timestamp": skin_share.timestamp,
            }));
            
            // Note: Injection timing is handled by the main LCU watcher logic
            // which will collect all friend skins and inject them at the appropriate time
            println!("[Party Mode] Skin share received and stored. Injection will be handled by LCU watcher timing logic.");
        }
        _ => {
            // Unknown message type, ignore
            println!("[Party Mode] Ignoring unknown message type: {}", message.message_type);
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

    // Filter friends to only those with sharing enabled
    let sharing_friends: Vec<&PairedFriend> = config.party_mode.paired_friends
        .iter()
        .filter(|friend| friend.share_enabled)
        .collect();

    if sharing_friends.is_empty() {
        println!("[Party Mode] No friends with sharing enabled");
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
        fantome_path: fantome_path.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let message = PartyModeMessage {
        message_type: "skin_share".to_string(),
        data: serde_json::to_value(skin_share)
            .map_err(|e| format!("Failed to serialize skin share: {}", e))?,
    };
    println!("[Party Mode][DEBUG] Prepared skin_share payload for champion_id={}, skin_id={}, chroma_id={:?}", champion_id, skin_id, chroma_id);

    for friend in &sharing_friends {
        println!("[Party Mode][DEBUG] Sending skin_share to friend {} ({}), share_enabled={} ", friend.summoner_name, friend.summoner_id, friend.share_enabled);
        if let Err(e) = send_chat_message(app, &lcu_connection, &friend.summoner_id, &message).await {
            eprintln!("Failed to send skin share to {}: {}", friend.summoner_name, e);
        } else {
            println!("[Party Mode] Successfully sent skin share to {}: Champion {}, Skin {}", 
                     friend.summoner_name, champion_id, skin_name);
            
            // Emit events for UI: toast + chat log line
            let _ = app.emit("party-mode-skin-sent", serde_json::json!({
                "to_friend": friend.summoner_name,
                "champion_id": champion_id,
                "skin_name": skin_name
            }));
            let _ = app.emit("party-mode-chat-log", serde_json::json!({
                "direction": "sent",
                "to": friend.summoner_name,
                "to_id": friend.summoner_id,
                "champion_id": champion_id,
                "skin_id": skin_id,
                "skin_name": skin_name,
                "chroma_id": chroma_id,
                "fantome_path": fantome_path.clone(),
                "timestamp": chrono::Utc::now().timestamp_millis(),
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

// Helper function to check if all paired friends with sharing enabled have shared their skins for a specific champion
pub async fn should_inject_now(app: &AppHandle, champion_id: u32) -> Result<bool, String> {
    // Always require a locked-in champion before proceeding
    if champion_id == 0 {
        println!("[Party Mode] ⏳ Local player has not locked in a champion yet - waiting");
        return Ok(false);
    }
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

    // Get friends with sharing enabled
    let friends_with_sharing: Vec<&PairedFriend> = config.party_mode.paired_friends
        .iter()
        .filter(|friend| friend.share_enabled)
        .collect();

    // If no friends have sharing enabled, inject immediately
    if friends_with_sharing.is_empty() {
        println!("[Party Mode] No friends with sharing enabled - injecting immediately for champion {}", champion_id);
        return Ok(true);
    }

    // Only consider friends who are actually in the same party (lobby or champ select) as the local player
    let lcu_connection = match get_lcu_connection(app).await {
        Ok(conn) => conn,
        Err(e) => {
            println!("[Party Mode] Could not get LCU connection ({}), defaulting to inject to avoid blocking", e);
            return Ok(true);
        }
    };

    let party_member_ids = get_current_party_member_summoner_ids(&lcu_connection).await.unwrap_or_default();
    println!("[Party Mode][DEBUG] Party members (summoner IDs) in current session: {:?}", party_member_ids);
    let mut friends_in_same_party: Vec<&PairedFriend> = friends_with_sharing
        .into_iter()
        .filter(|f| party_member_ids.contains(&f.summoner_id))
        .collect();

    // Remove self if present just in case
    if friends_in_same_party.is_empty() {
        // You're solo or no paired friends in party; don't block injection
        println!("[Party Mode] Solo or no paired friends in current party - injecting immediately for champion {}", champion_id);
        return Ok(true);
    }

    // Check if we have received skins from all friends with sharing enabled (for ANY champion)
    let mut friends_who_shared = std::collections::HashSet::new();
    
    let received_skins = RECEIVED_SKINS.lock().unwrap();
    for (_key, received_skin) in received_skins.iter() {
        // Count friends who shared ANY skin, not just for the current champion
        friends_who_shared.insert(received_skin.from_summoner_id.clone());
    }
    println!("[Party Mode][DEBUG] Friends who shared (IDs): {:?}", friends_who_shared);

    // Count how many friends with sharing enabled we have vs how many shared
    let total_sharing_friends = friends_in_same_party.len();
    let sharing_friends_who_shared: usize = friends_in_same_party
        .iter()
        .filter(|friend| friends_who_shared.contains(&friend.summoner_id))
        .count();

    println!("[Party Mode] Injection timing check: {}/{} friends with sharing enabled have shared skins (any champion)", 
             sharing_friends_who_shared, total_sharing_friends);

    // List which friends have shared and which haven't
    for friend in &friends_in_same_party {
        let has_shared = friends_who_shared.contains(&friend.summoner_id);
        println!("[Party Mode] Friend {} ({}): {}", 
                 friend.display_name, 
                 friend.summoner_id,
                 if has_shared { "✅ shared skin" } else { "⏳ waiting for skin" });
    }

    // Locked-in champion enforced at top

    // Wait for ALL friends with sharing enabled to share their skins before injecting
    let should_inject = sharing_friends_who_shared == total_sharing_friends;
    
    if should_inject {
        println!("[Party Mode] ✅ All friends with sharing enabled have shared AND local player has locked in champion {} - proceeding with injection", champion_id);
    } else {
        println!("[Party Mode] ⏳ Local player locked in champion {}, but waiting for {} more friends with sharing enabled to share their skins", 
                 champion_id, total_sharing_friends - sharing_friends_who_shared);
        println!("[Party Mode][DEBUG] total_sharing_friends={} shared={}", total_sharing_friends, sharing_friends_who_shared);
    }

    Ok(should_inject)
}

// Determine summoner IDs in the same party (lobby or champ select team) as the local player
async fn get_current_party_member_summoner_ids(lcu: &LcuConnection) -> Result<std::collections::HashSet<String>, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let auth = general_purpose::STANDARD.encode(format!("riot:{}", lcu.token));

    // Try champ select session first
    let cs_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", lcu.port);
    if let Ok(resp) = client.get(&cs_url)
        .header("Authorization", format!("Basic {}", auth))
        .send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let mut ids = std::collections::HashSet::new();
                if let Some(team) = json.get("myTeam").and_then(|v| v.as_array()) {
                    for p in team {
                        if let Some(id) = p.get("summonerId").and_then(|v| v.as_i64()) {
                            ids.insert(id.to_string());
                        }
                    }
                }
                if !ids.is_empty() { return Ok(ids); }
            }
        }
    }

    // Fallback to lobby data
    let lobby_url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", lcu.port);
    if let Ok(resp) = client.get(&lobby_url)
        .header("Authorization", format!("Basic {}", auth))
        .send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let mut ids = std::collections::HashSet::new();
                if let Some(members) = json.get("members").and_then(|v| v.as_array()) {
                    for m in members {
                        if let Some(id) = m.get("summonerId").and_then(|v| v.as_i64()) {
                            ids.insert(id.to_string());
                        }
                    }
                }
                return Ok(ids);
            }
        }
    }

    Ok(std::collections::HashSet::new())
}

// Add a function to clear received skins (call this when leaving champ select or starting a new session)
pub fn clear_received_skins() {
    let mut map = RECEIVED_SKINS.lock().unwrap();
    map.clear();
}
