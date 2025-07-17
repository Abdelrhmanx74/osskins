use tauri::{AppHandle, Emitter, Manager};
use std::collections::HashMap;
use std::io::Write;
use serde_json;
use base64;
use std::path::PathBuf;
use crate::commands::types::{
    SavedConfig, PartyModeConfig, PairedFriend, FriendInfo, ConnectionRequest, 
    PartyModeMessage, PairingRequest, PairingResponse, SkinShare, ReceivedSkinData
};

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
    let auth = base64::encode(format!("riot:{}", token));

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
    let lcu_connection = get_lcu_connection(&app).await?;
    let current_summoner = get_current_summoner(&app).await?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    let pairing_request = PairingRequest {
        request_id: request_id.clone(),
        from_summoner_id: current_summoner.summoner_id,
        from_summoner_name: current_summoner.display_name,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let message = PartyModeMessage {
        message_type: "pairing_request".to_string(),
        data: serde_json::to_value(pairing_request)
            .map_err(|e| format!("Failed to serialize pairing request: {}", e))?,
    };

    send_chat_message(&app, &lcu_connection, &friend_summoner_id, &message).await?;
    Ok(request_id)
}

// Tauri command to respond to pairing request
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
        request_id,
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

    // If accepted, add to paired friends
    if accepted {
        add_paired_friend(&app, &friend_summoner_id, "Unknown").await?;
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

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Tauri command to get paired friends
#[tauri::command]
pub async fn get_paired_friends(app: AppHandle) -> Result<Vec<PairedFriend>, String> {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");

    if !config_file.exists() {
        return Ok(Vec::new());
    }

    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(config.party_mode.paired_friends)
}

// Tauri command to update party mode settings
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

// Internal function to send chat message
async fn send_chat_message(
    app: &AppHandle,
    lcu_connection: &LcuConnection,
    friend_summoner_id: &str,
    message: &PartyModeMessage,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Get conversation ID with the friend
    let conversation_id = get_conversation_id(app, lcu_connection, friend_summoner_id).await?;

    let message_json = serde_json::to_string(message)
        .map_err(|e| format!("Failed to serialize message: {}", e))?;
    
    let full_message = format!("{}{}", PARTY_MODE_MESSAGE_PREFIX, message_json);

    let url = format!(
        "https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages",
        lcu_connection.port, conversation_id
    );
    let auth = base64::encode(format!("riot:{}", lcu_connection.token));

    let message_payload = serde_json::json!({
        "body": full_message,
        "type": "chat"
    });

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

    Ok(())
}

// Internal function to get conversation ID
async fn get_conversation_id(
    app: &AppHandle,
    lcu_connection: &LcuConnection,
    friend_summoner_id: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // First, get the friend's PID from the friends list using the summoner ID
    let friends_url = format!("https://127.0.0.1:{}/lol-chat/v1/friends", lcu_connection.port);
    let auth = base64::encode(format!("riot:{}", lcu_connection.token));

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

    // Find the friend's PID by matching summoner ID
    let mut friend_pid = None;
    if let Some(friends_array) = friends_data.as_array() {
        for friend in friends_array {
            if let Some(summoner_id) = friend.get("summonerId").and_then(|v| v.as_u64()).map(|id| id.to_string()) {
                if summoner_id == friend_summoner_id {
                    friend_pid = friend.get("pid").and_then(|v| v.as_str()).map(|s| s.to_string());
                    break;
                }
            }
        }
    }

    let friend_pid = friend_pid.ok_or_else(|| format!("Friend with summoner ID {} not found in friends list", friend_summoner_id))?;

    // Now get conversations and find the one with matching PID
    let conversations_url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations", lcu_connection.port);

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
        for conversation in conversations_array {
            if let Some(pid) = conversation.get("pid").and_then(|p| p.as_str()) {
                if pid == friend_pid {
                    if let Some(id) = conversation.get("id").and_then(|i| i.as_str()) {
                        return Ok(id.to_string());
                    }
                }
            }
        }
    }

    Err(format!("Conversation not found for friend with PID: {}", friend_pid))
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
    let auth = base64::encode(format!("riot:{}", lcu_connection.token));

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

    let display_name = summoner_data
        .get("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    Ok(CurrentSummoner {
        summoner_id,
        display_name: display_name.to_string(),
    })
}

// Function to handle incoming party mode messages (called from LCU watcher)
pub async fn handle_party_mode_message(
    app: &AppHandle,
    message_body: &str,
    from_summoner_id: &str,
) -> Result<(), String> {
    if !message_body.starts_with(PARTY_MODE_MESSAGE_PREFIX) {
        return Ok(()); // Not a party mode message
    }

    let message_json = &message_body[PARTY_MODE_MESSAGE_PREFIX.len()..];
    let message: PartyModeMessage = serde_json::from_str(message_json)
        .map_err(|e| format!("Failed to parse party mode message: {}", e))?;

    match message.message_type.as_str() {
        "pairing_request" => {
            let request: PairingRequest = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse pairing request: {}", e))?;
            
            let connection_request = ConnectionRequest {
                from_summoner_id: request.from_summoner_id,
                from_summoner_name: request.from_summoner_name,
                timestamp: request.timestamp,
            };

            let _ = app.emit("party-mode-connection-request", connection_request);
        }
        "pairing_response" => {
            let response: PairingResponse = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse pairing response: {}", e))?;
            
            if response.accepted {
                add_paired_friend(app, &response.from_summoner_id, &response.from_summoner_name).await?;
                let _ = app.emit("party-mode-pairing-accepted", response);
            } else {
                let _ = app.emit("party-mode-pairing-declined", response);
            }
        }
        "skin_share" => {
            let skin_share: SkinShare = serde_json::from_value(message.data)
                .map_err(|e| format!("Failed to parse skin share: {}", e))?;
            
            // Store received skin data
            store_received_skin(app, &skin_share).await?;
            
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

    let skin_name = "Unknown Skin".to_string(); // TODO: Look up real skin name from champion_id/skin_id if available
    let skin_share = SkinShare {
        from_summoner_id: current_summoner.summoner_id,
        from_summoner_name: current_summoner.display_name,
        champion_id,
        skin_id,
        skin_name,
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
        }
    }

    Ok(())
}

// Internal function to store received skin data
async fn store_received_skin(app: &AppHandle, skin_share: &SkinShare) -> Result<(), String> {
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
        }
    };

    let key = format!("{}_{}", skin_share.from_summoner_id, skin_share.champion_id);
    config.party_mode.received_skins.insert(key, ReceivedSkinData {
        from_summoner_id: skin_share.from_summoner_id.clone(),
        from_summoner_name: skin_share.from_summoner_name.clone(),
        champion_id: skin_share.champion_id,
        skin_id: skin_share.skin_id,
        chroma_id: skin_share.chroma_id,
        fantome_path: skin_share.fantome_path.clone(),
        received_at: skin_share.timestamp,
    });

    let updated_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_file, updated_config)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

// Test function to clear all test data
#[tauri::command]
pub async fn clear_test_data(app: AppHandle) -> Result<(), String> {
    println!("[Party Mode Test] Clearing test data...");
    
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");
    
    if config_file.exists() {
        let config_data = std::fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        
        let mut config: SavedConfig = serde_json::from_str(&config_data)
            .map_err(|e| format!("Failed to parse config: {}", e))?;
        
        // Clear paired friends and received skins
        config.party_mode.paired_friends.clear();
        config.party_mode.received_skins.clear();
        
        let updated_config = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        std::fs::write(&config_file, updated_config)
            .map_err(|e| format!("Failed to save config: {}", e))?;
    }
    
    println!("[Party Mode Test] Test data cleared!");
    Ok(())
}

#[tauri::command]
pub async fn simulate_party_mode_test(app: tauri::AppHandle) -> Result<(), String> {
    println!("[Party Mode Test] Starting simulation...");

    // Fetch real friends list
    let friends = get_lcu_friends(app.clone()).await?;
    let friend = friends.get(0).ok_or("No friends found in your League client")?;

    // Simulate a pairing request from a real friend
    let test_request = PairingRequest {
        request_id: "test-request-123".to_string(),
        from_summoner_id: friend.summoner_id.clone(),
        from_summoner_name: friend.display_name.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };
    let test_message = PartyModeMessage {
        message_type: "pairing_request".to_string(),
        data: serde_json::to_value(test_request).map_err(|e| format!("Failed to serialize test request: {}", e))?,
    };
    let message_str = serde_json::to_string(&test_message).map_err(|e| format!("Failed to serialize test message: {}", e))?;
    handle_party_mode_message(&app, &format!("OSS:{}", message_str), &friend.summoner_id).await?;

    // Simulate a skin share from the same friend (Ezreal - Battle Academia)
    let test_skin_share = SkinShare {
        from_summoner_id: friend.summoner_id.clone(),
        from_summoner_name: friend.display_name.clone(),
        champion_id: 81, // Ezreal
        skin_id: 81021, // Battle Academia Ezreal
        skin_name: "Ezreal - Battle Academia".to_string(),
        chroma_id: Some(8132),
        fantome_path: None,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };
    let test_skin_message = PartyModeMessage {
        message_type: "skin_share".to_string(),
        data: serde_json::to_value(test_skin_share).map_err(|e| format!("Failed to serialize test skin share: {}", e))?,
    };
    let skin_str = serde_json::to_string(&test_skin_message).map_err(|e| format!("Failed to serialize test skin message: {}", e))?;
    handle_party_mode_message(&app, &format!("OSS:{}", skin_str), &friend.summoner_id).await?;

    println!("[Party Mode Test] Simulation completed!");
    Ok(())
}

#[derive(Clone)]
struct MockFriendSkin {
    summoner_id: String,
    summoner_name: String,
    champion_id: u32,
    skin_id: u32,
    skin_name: String,
    chroma_id: Option<u32>,
    fantome_path: Option<String>,
}

#[tauri::command]
pub async fn simulate_multiple_skin_shares(app: tauri::AppHandle) -> Result<(), String> {
    // Always clear test data first to avoid duplicates
    let _ = clear_test_data(app.clone()).await;
    println!("[Party Mode Test] Starting multiple skin shares simulation...");

    let mock_friends = vec![
        MockFriendSkin {
            summoner_id: "user1".to_string(),
            summoner_name: "You#1234".to_string(),
            champion_id: 1,
            skin_id: 1003,
            skin_name: "Annie in Wonderland".to_string(),
            chroma_id: None,
            fantome_path: Some("annie/annie_in_wonderland.zip".to_string()),
        },
        MockFriendSkin {
            summoner_id: "friend1".to_string(),
            summoner_name: "rogolax#EUW".to_string(),
            champion_id: 81,
            skin_id: 81021,
            skin_name: "Battle Academia Ezreal".to_string(),
            chroma_id: None,
            fantome_path: Some("ezreal/battle_academia_ezreal.zip".to_string()),
        },
    ];

    for friend in &mock_friends {
        println!("[Party Mode Test] Simulating skin share from {}: {}", friend.summoner_name, friend.skin_name);
        let test_skin_share = SkinShare {
            from_summoner_id: friend.summoner_id.clone(),
            from_summoner_name: friend.summoner_name.clone(),
            champion_id: friend.champion_id,
            skin_id: friend.skin_id,
            skin_name: friend.skin_name.clone(),
            chroma_id: friend.chroma_id,
            fantome_path: friend.fantome_path.clone(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };
        let test_skin_message = PartyModeMessage {
            message_type: "skin_share".to_string(),
            data: serde_json::to_value(test_skin_share).map_err(|e| format!("Failed to serialize test skin share: {}", e))?,
        };
        let skin_str = serde_json::to_string(&test_skin_message).map_err(|e| format!("Failed to serialize test skin message: {}", e))?;
        handle_party_mode_message(&app, &format!("OSS:{}", skin_str), &friend.summoner_id).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    println!("[Party Mode Test] Multiple skin shares simulation completed!");
    Ok(())
}

#[tauri::command]
pub async fn simulate_multiple_skin_injection(app: tauri::AppHandle) -> Result<(), String> {
    println!("[Party Mode Test] Starting multiple skin injection simulation...");
    simulate_multiple_skin_shares(app.clone()).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let config_file = config_dir.join("config.json");
    if !config_file.exists() {
        return Err("No config file found".to_string());
    }
    let config_data = std::fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let config: SavedConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    if config.party_mode.received_skins.is_empty() {
        return Err("No received skins found. Run multiple skin shares first.".to_string());
    }
    // Debug printout of all received skins
    println!("[Party Mode Test] All received skins:");
    for (key, skin) in &config.party_mode.received_skins {
        println!("Key: {}, champion_id: {}, skin_id: {}, chroma_id: {:?}, fantome_path: {:?}", key, skin.champion_id, skin.skin_id, skin.chroma_id, skin.fantome_path);
    }
    let league_path = config.league_path.ok_or("No league path configured")?;
    println!("[Party Mode Test] Injecting {} received skins...", config.party_mode.received_skins.len());
    let mut skins_to_inject = Vec::new();
    for (_key, received_skin) in &config.party_mode.received_skins {
        println!("[Party Mode Test] Injecting skin from {}: Champion {}, Skin {}", received_skin.from_summoner_name, received_skin.champion_id, received_skin.skin_id);
        let skin = crate::injection::Skin {
            champion_id: received_skin.champion_id,
            skin_id: received_skin.skin_id,
            chroma_id: received_skin.chroma_id,
            fantome_path: received_skin.fantome_path.clone(),
        };
        skins_to_inject.push(skin);
    }
    let champions_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("champions");
    match crate::injection::inject_skins(
        &app,
        &league_path,
        &skins_to_inject,
        &champions_dir
    ) {
        Ok(_) => {
            println!("[Party Mode Test] Multiple skin injection completed successfully!");
            println!("[Party Mode Test] All {} have been injected!", skins_to_inject.len());
            Ok(())
        },
        Err(e) => {
            println!("[Party Mode Test] Failed to inject skins: {}", e);
            Err(format!("Failed to inject skins: {}", e))
        }
    }
}
