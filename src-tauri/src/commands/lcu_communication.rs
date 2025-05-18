use super::types::*;
use tauri::{AppHandle, Manager, Emitter};
use std::path::{Path, PathBuf};
use std::fs;
use reqwest;
use base64;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use chrono;
use serde_json;
use crate::injection::Skin;
use crate::get_selected_champion_id;
use crate::inject_skins_for_champions;

#[tauri::command]
pub fn start_lcu_watcher(app: AppHandle, leaguePath: String) -> Result<(), String> {
    println!("Starting LCU status watcher for path: {}", leaguePath);
    let app_handle = app.clone();
    let league_path_clone = leaguePath.clone();
    
    thread::spawn(move || {
        let mut last_phase = String::new();
        let mut was_in_game = false;
        let mut was_reconnecting = false;
        let _ = app_handle.emit("lcu-status", "None".to_string());
        
        // Track last seen selections to detect changes
        let mut last_selected_skins: std::collections::HashMap<u32, SkinData> = std::collections::HashMap::new();
        let mut last_skin_check_time = std::time::Instant::now();
        let mut last_champion_id: Option<u32> = None;
        
        loop {
            let mut sleep_duration = Duration::from_secs(5);
            
            let log_msg = format!("Monitoring directory: {}", league_path_clone);
            println!("{}", log_msg);
            emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");
            
            // Only check the configured League directory for lockfile
            let search_dirs = [PathBuf::from(&league_path_clone)];
            let mut port = None;
            let mut token = None;
            let mut found_any_lockfile = false;
            let mut lockfile_path = None;
            
            // Rest of the lockfile detection code remains the same
            for dir in &search_dirs {
                let log_msg = format!("Looking for lockfiles in: {}", dir.display());
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");
                
                // Check each possible lockfile name
                for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
                    let path = dir.join(name);
                    if path.exists() {
                        found_any_lockfile = true;
                        lockfile_path = Some(path.clone());
                        println!("Found lockfile: {}", path.display());
                        emit_terminal_log(&app_handle, &format!("Found lockfile: {}", path.display()), "lcu-watcher");
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        let parts: Vec<&str> = content.split(':').collect();
                        if parts.len() >= 5 {
                            port = Some(parts[2].to_string());
                            token = Some(parts[3].to_string());
                            found_any_lockfile = true;
                            break;
                        }
                    }
                }
                
                if port.is_some() && token.is_some() {
                    break;
                }
            }
            
            if (!found_any_lockfile) {
                // Handle no lockfile found cases...
                if was_in_game && (last_phase == "InProgress" || was_reconnecting) {
                    thread::sleep(Duration::from_secs(5));
                    continue;
                } else if was_in_game && last_phase == "None" {
                    if let Err(e) = crate::injection::cleanup_injection(&app_handle, &league_path_clone) {
                        println!("Error cleaning up injection: {}", e);
                        emit_terminal_log(&app_handle, &format!("Error cleaning up injection: {}", e), "error");
                    }
                    was_in_game = false;
                }
                
                let log_msg = format!("No valid lockfile found. Is League running? The lockfile should be at: {}", league_path_clone);
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");
                thread::sleep(Duration::from_secs(5));
                continue;
            }
            
            let port = port.unwrap();
            let token = token.unwrap();
            let lockfile_path = lockfile_path.unwrap();
            
            'lcu_connected: loop {
                if !lockfile_path.exists() {
                    break 'lcu_connected;
                }

                match reqwest::blocking::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build() 
                {
                    Ok(client) => {
                        let endpoints = [
                            "/lol-gameflow/v1/session",
                            "/lol-gameflow/v1/gameflow-phase",
                        ];
                        
                        let mut connected = false;
                        let mut phase_value: Option<String> = None;
                        
                        for endpoint in endpoints {
                            let url = format!("https://127.0.0.1:{}{}", port, endpoint);
                            let auth = base64::encode(format!("riot:{}", token));
                            
                            match client.get(&url)
                                .header("Authorization", format!("Basic {}", auth))
                                .send() 
                            {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        connected = true;
                                        
                                        match resp.json::<serde_json::Value>() {
                                            Ok(json) => {
                                                if endpoint == "/lol-gameflow/v1/gameflow-phase" {
                                                    if let Some(phase) = json.as_str() {
                                                        phase_value = Some(phase.to_string());
                                                        break;
                                                    }
                                                } else {
                                                    if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
                                                        phase_value = Some(phase.to_string());
                                                        break;
                                                    }
                                                }
                                            },
                                            Err(e) => println!("Failed to parse response from {}: {}", endpoint, e),
                                        }
                                    }
                                },
                                Err(e) => println!("Failed to connect to endpoint {}: {}", endpoint, e),
                            }
                        }

                        if (!connected) {
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                        
                        let phase = phase_value.unwrap_or_else(|| "None".to_string());
                        
                        if phase != last_phase {
                            println!("LCU status changed: {} -> {}", last_phase, phase);
                            let log_msg = format!("LCU status changed: {} -> {}", last_phase, phase);
                            emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");

                            // If entering ChampSelect, preload assets to speed up injection later
                            if phase == "ChampSelect" {
                                let champions_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."))
                                    .join("champions");
                                
                                if !champions_dir.exists() {
                                    if let Err(e) = fs::create_dir_all(&champions_dir) {
                                        println!("Failed to create champions directory: {}", e);
                                    }
                                }
                                
                                let app_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."));
                                let overlay_dir = app_dir.join("overlay");
                                if overlay_dir.exists() {
                                    if let Err(e) = fs::remove_dir_all(&overlay_dir) {
                                        println!("Failed to clean overlay directory: {}", e);
                                    }
                                }
                            }
                        }
                        
                        if phase == "ChampSelect" {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_skin_check_time).as_secs() >= 1 {
                                last_skin_check_time = now;
                                
                                let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
                                let auth = base64::encode(format!("riot:{}", token));
                                
                                if let Ok(resp) = client.get(&session_url)
                                    .header("Authorization", format!("Basic {}", auth))
                                    .send() 
                                {
                                    if resp.status().is_success() {
                                        if let Ok(json) = resp.json::<serde_json::Value>() {
                                            
                                            if let Some(selected_champ_id) = get_selected_champion_id(&json) {
                                                let current_champion_id = selected_champ_id as u32;
                                                
                                                let champion_changed = last_champion_id != Some(current_champion_id) && current_champion_id > 0;
                                                if champion_changed {
                                                    last_champion_id = Some(current_champion_id);
                                                    
                                                    let config_dir = app_handle.path().app_data_dir()
                                                        .unwrap_or_else(|_| PathBuf::from("."))
                                                        .join("config");
                                                    let cfg_file = config_dir.join("config.json");
                                                    
                                                    if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                                                        if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                                                            if let Some(skin) = config.skins.iter().find(|s| s.champion_id == current_champion_id) {
                                                                
                                                                let skins = vec![Skin {
                                                                    champion_id: skin.champion_id,
                                                                    skin_id: skin.skin_id,
                                                                    chroma_id: skin.chroma_id,
                                                                    fantome_path: skin.fantome.clone(),
                                                                }];
                                                                
                                                                let champions_dir = app_handle.path().app_data_dir()
                                                                    .unwrap_or_else(|_| PathBuf::from("."))
                                                                    .join("champions");
                                                                
                                                                match crate::injection::inject_skins(
                                                                    &app_handle,
                                                                    &league_path_clone,
                                                                    &skins,
                                                                    &champions_dir
                                                                ) {
                                                                    Ok(_) => {
                                                                        let _ = app_handle.emit("injection-status", "success");
                                                                    },
                                                                    Err(e) => {
                                                                        let _ = app_handle.emit("skin-injection-error", format!(
                                                                            "Failed to inject pre-selected skin for champion {}: {}", current_champion_id, e
                                                                        ));
                                                                        let _ = app_handle.emit("injection-status", "error");
                                                                    }
                                                                }
                                                                
                                                                last_selected_skins.insert(current_champion_id, skin.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                last_champion_id = None;
                                            }
                                        }
                                    }
                                }
                                
                                let config_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."))
                                    .join("config");
                                let cfg_file = config_dir.join("config.json");
                                
                                if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                                    if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                                        let mut skin_changes = false;
                                        
                                        for skin in &config.skins {
                                            let champ_id = skin.champion_id;
                                            
                                            if last_champion_id == Some(champ_id) {
                                                if !last_selected_skins.contains_key(&champ_id) ||
                                                   last_selected_skins.get(&champ_id).map_or(true, |old_skin| {
                                                       old_skin.skin_id != skin.skin_id || 
                                                       old_skin.chroma_id != skin.chroma_id || 
                                                       old_skin.fantome != skin.fantome
                                                   }) 
                                                {
                                                    
                                                    let skins = vec![Skin {
                                                        champion_id: skin.champion_id,
                                                        skin_id: skin.skin_id,
                                                        chroma_id: skin.chroma_id,
                                                        fantome_path: skin.fantome.clone(),
                                                    }];
                                                    
                                                    let champions_dir = app_handle.path().app_data_dir()
                                                        .unwrap_or_else(|_| PathBuf::from("."))
                                                        .join("champions");
                                                    
                                                    if phase != "ChampSelect" {
                                                        continue;
                                                    }
                                                    
                                                    match crate::injection::inject_skins(
                                                        &app_handle,
                                                        &league_path_clone,
                                                        &skins,
                                                        &champions_dir
                                                    ) {
                                                        Ok(_) => {
                                                            let _ = app_handle.emit("injection-status", "success");
                                                        },
                                                        Err(e) => {
                                                            let _ = app_handle.emit("skin-injection-error", format!(
                                                                "Failed to inject skin for champion {}: {}", champ_id, e
                                                            ));
                                                            let _ = app_handle.emit("injection-status", "error");
                                                        }
                                                    }
                                                    
                                                    last_selected_skins.insert(champ_id, skin.clone());
                                                    skin_changes = true;
                                                }
                                            }
                                        }
                                        
                                        if skin_changes {
                                            emit_terminal_log(&app_handle, "Updated skin selection tracking", "lcu-watcher");
                                        }
                                    }
                                }
                            }
                            
                            if phase != "ChampSelect" && phase != "None" && last_phase == "ChampSelect" {
                                let _ = crate::injection::cleanup_injection(&app_handle, &league_path_clone);
                            }
                            
                            sleep_duration = Duration::from_secs(1);
                        } else if phase == "InProgress" {
                            // Keep existing in-game phase behavior
                        }

                        // Handle Swift Play mode - detect Lobby -> Matchmaking transition
                        if last_phase == "Lobby" && phase == "Matchmaking" {
                            emit_terminal_log(&app_handle, "Detected transition from Lobby to Matchmaking, checking for Swift Play mode", "lcu-watcher");
                            
                            // Check current queue information
                            let queue_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", port);
                            let auth = base64::encode(format!("riot:{}", token));
                            
                            match client.get(&queue_url)
                                .header("Authorization", format!("Basic {}", auth))
                                .send()
                            {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        match resp.json::<serde_json::Value>() {
                                            Ok(json) => {
                                                // Log the full response structure for debugging
        // Add this line to log the full session data
        emit_terminal_log(&app_handle, &format!("[LCU Debug] Full session data: {}", 
            serde_json::to_string_pretty(&json).unwrap_or_default()), "debug");

        // Log the full response structure for debugging
        emit_terminal_log(&app_handle, "[LCU Debug] Swift Play session structure:", "debug");
                                                // Print important paths in the JSON that might contain champion selections
                                                if let Some(game_data) = json.get("gameData") {
                                                    if let Some(queue) = game_data.get("queue") {
                                                        if let Some(queue_id) = queue.get("id").and_then(|id| id.as_i64()) {
                                                            emit_terminal_log(&app_handle, &format!("[LCU Debug] Queue ID: {}", queue_id), "debug");
                                                            
                                                            // Check if this is Swift Play queue (both ID 1700 and ID 480)
                                                            if queue_id == 1700 || queue_id == 480 {
                                                                emit_terminal_log(&app_handle, &format!("Confirmed Swift Play queue or compatible queue (ID: {})", queue_id), "lcu-watcher");
                                                                
                                                                // Log player selection data
                                                                if let Some(player_selections) = game_data.get("playerChampionSelections") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] playerChampionSelections: {}", player_selections), "debug");
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Debug] No playerChampionSelections found", "debug");
                                                                }
                                                                
                                                                if let Some(selected_champs) = game_data.get("selectedChampions") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] selectedChampions: {}", selected_champs), "debug");
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Debug] No selectedChampions found", "debug");
                                                                }
                                                                
                                                                // Check for local player data
                                                                if let Some(local_player) = json.get("localPlayerSelection") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] localPlayerSelection: {}", local_player), "debug");
                                                                }
                                                                
                                                                // Check for team data
                                                                if let Some(my_team) = json.get("myTeam") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] myTeam: {}", my_team), "debug");
                                                                }
                                                                
                                                                // Check for role assignments
                                                                if let Some(roles) = json.get("roleAssignments") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] roleAssignments: {}", roles), "debug");
                                                                }
                                                                
                                                                // Get player champion selections for Swift Play
                                                                let swift_play_champion_ids = get_swift_play_champion_selections(&json);
                                                                
                                                                if !swift_play_champion_ids.is_empty() {
                                                                    emit_terminal_log(&app_handle, &format!(
                                                                        "Swift Play: Found {} champion selections: {:?}", 
                                                                        swift_play_champion_ids.len(), 
                                                                        swift_play_champion_ids
                                                                    ), "lcu-watcher");
                                                                    
                                                                    // Inject skins for all selected champions
                                                                    inject_skins_for_champions(&app_handle, &league_path_clone, &swift_play_champion_ids);
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "Swift Play: No champion selections found in session data", "lcu-watcher");
                                                                    
                                                                    // Try checking additional endpoints to find Swift Play champions
                                                                    let swift_play_url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
                                                                    match client.get(&swift_play_url)
                                                                        .header("Authorization", format!("Basic {}", auth))
                                                                        .send()
                                                                    {
                                                                        Ok(swift_resp) => {
                                                                            if swift_resp.status().is_success() {
                                                                                if let Ok(lobby_json) = swift_resp.json::<serde_json::Value>() {
                                                                                    emit_terminal_log(&app_handle, "[LCU Debug] Swift Play lobby data found", "debug");
                                                                                    
                                                                                    // Try to extract champion IDs from lobby data
                                                                                    let lobby_champion_ids = extract_swift_play_champions_from_lobby(&lobby_json);
                                                                                    
                                                                                    if !lobby_champion_ids.is_empty() {
                                                                                        emit_terminal_log(&app_handle, &format!(
                                                                                            "Swift Play: Found {} champion selections from lobby: {:?}", 
                                                                                            lobby_champion_ids.len(), 
                                                                                            lobby_champion_ids
                                                                                        ), "lcu-watcher");
                                                                                        
                                                                                        // Inject skins for all selected champions from lobby
                                                                                        inject_skins_for_champions(&app_handle, &league_path_clone, &lobby_champion_ids);
                                                                                    } else {
                                                                                        emit_terminal_log(&app_handle, "Swift Play: No champion selections found in lobby data", "lcu-watcher");
                                                                                        emit_terminal_log(&app_handle, &format!("[LCU Debug] Full lobby data: {}", 
                                                                                            serde_json::to_string_pretty(&lobby_json).unwrap_or_default()), "debug");
                                                                                    }
                                                                                }
                                                                            }
                                                                        },
                                                                        Err(e) => emit_terminal_log(&app_handle, &format!("[LCU Debug] Failed to get lobby data: {}", e), "debug"),
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                            Err(e) => println!("Failed to parse queue info: {}", e),
                                        }
                                    }
                                },
                                Err(e) => println!("Failed to get queue information: {}", e),
                            }
                        }
                        
                        last_phase = phase.to_string();
                        was_reconnecting = phase == "Reconnect";
                        was_in_game = phase == "InProgress" || was_reconnecting;
                    },
                    Err(e) => println!("Failed to build HTTP client: {}", e),
                }
                
                thread::sleep(sleep_duration);
            }
        }
    });                       

    println!("LCU status watcher thread started");
    Ok(())
}

// New command to get the friends list from LCU
#[tauri::command]
pub fn get_lcu_friends(app: AppHandle, league_path: String) -> Result<Vec<Friend>, String> {
    // Find the lockfile to get auth details
    let lockfile_path = find_lockfile(&league_path)?;
    let (port, token) = get_auth_from_lockfile(&lockfile_path)?;
    
    let client = get_lcu_client();
    let url = format!("https://127.0.0.1:{}/lol-chat/v1/friends", port);
    let auth = base64::encode(format!("riot:{}", token));
    
    match client.get(&url)
        .header("Authorization", format!("Basic {}", auth))
        .send() 
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<Vec<Friend>>() {
                    Ok(friends) => {
                        // Filter out friends without proper data
                        let valid_friends: Vec<Friend> = friends.into_iter()
                            .filter(|f| !f.id.is_empty() && !f.name.is_empty())
                            .collect();
                        
                        emit_terminal_log(&app, &format!("Found {} friends in LCU", valid_friends.len()), "info");
                        Ok(valid_friends)
                    },
                    Err(e) => Err(format!("Failed to parse friends data: {}", e)),
                }
            } else {
                Err(format!("LCU API returned error: {}", resp.status()))
            }
        },
        Err(e) => Err(format!("Failed to connect to LCU API: {}", e)),
    }
}

// New command to send a message to a friend
#[tauri::command]
pub fn send_lcu_message(app: AppHandle, league_path: String, friend_id: String, message: String) -> Result<(), String> {
    // Find the lockfile to get auth details
    let lockfile_path = find_lockfile(&league_path)?;
    let (port, token) = get_auth_from_lockfile(&lockfile_path)?;
    
    let client = get_lcu_client();
    let url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages", port, friend_id);
    let auth = base64::encode(format!("riot:{}", token));
    
    // Create the message payload
    let payload = serde_json::json!({
        "body": message,
        "type": "chat"
    });
    
    match client.post(&url)
        .header("Authorization", format!("Basic {}", auth))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&payload).unwrap())
        .send() 
    {
        Ok(resp) => {
            if resp.status().is_success() {
                emit_terminal_log(&app, &format!("Message sent to friend {}", friend_id), "info");
                Ok(())
            } else {
                Err(format!("LCU API returned error: {}", resp.status()))
            }
        },
        Err(e) => Err(format!("Failed to connect to LCU API: {}", e)),
    }
}

// New command to get messages from a conversation
#[tauri::command]
pub fn get_lcu_messages(app: AppHandle, league_path: String, friend_id: String) -> Result<serde_json::Value, String> {
    // Find the lockfile to get auth details
    emit_terminal_log(&app, &format!("Attempting to get messages with friend_id: {}", friend_id), "debug");
    let lockfile_path = match find_lockfile(&league_path) {
        Ok(path) => path,
        Err(e) => {
            emit_terminal_log(&app, &format!("Failed to find lockfile: {}", e), "error");
            return Err(format!("Failed to find lockfile: {}", e));
        }
    };
    
    let (port, token) = match get_auth_from_lockfile(&lockfile_path) {
        Ok((p, t)) => (p, t),
        Err(e) => {
            emit_terminal_log(&app, &format!("Failed to get auth from lockfile: {}", e), "error");
            return Err(format!("Failed to get auth from lockfile: {}", e));
        }
    };
    
    let client = get_lcu_client();
    
    // First, get the summoner ID for the local player to form the conversation ID
    let summoner_url = format!("https://127.0.0.1:{}/lol-summoner/v1/current-summoner", port);
    let auth = base64::encode(format!("riot:{}", token));
    
    emit_terminal_log(&app, "Requesting current summoner data...", "debug");
    let my_summoner = match client.get(&summoner_url)
        .header("Authorization", format!("Basic {}", auth))
        .send() 
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>() {
                    Ok(data) => data,
                    Err(e) => {
                        emit_terminal_log(&app, &format!("Failed to parse summoner data: {}", e), "error");
                        return Err(format!("Failed to parse summoner data: {}", e));
                    }
                }
            } else {
                let error_msg = format!("LCU API returned error: {} when fetching summoner data", resp.status());
                emit_terminal_log(&app, &error_msg, "error");
                return Err(error_msg);
            }
        },
        Err(e) => {
            let error_msg = format!("Failed to connect to LCU API for summoner data: {}", e);
            emit_terminal_log(&app, &error_msg, "error");
            return Err(error_msg);
        }
    };
    
    // Get the local summoner's ID (puuid)
    let my_puuid = match my_summoner.get("puuid") {
        Some(id) => id.as_str().unwrap_or(""),
        None => {
            emit_terminal_log(&app, "Failed to get current summoner's puuid", "error");
            return Err("Failed to get current summoner's puuid".to_string());
        }
    };
    
    if my_puuid.is_empty() {
        emit_terminal_log(&app, "Invalid summoner puuid (empty string)", "error");
        return Err("Invalid summoner puuid".to_string());
    }
    
    emit_terminal_log(&app, &format!("Local summoner PUUID: {}", my_puuid), "debug");
    emit_terminal_log(&app, &format!("Friend ID with suffix: {}", friend_id), "debug");
    
    // Clean the friend ID by removing the server suffix (e.g., @eu1.pvp.net)
    let clean_friend_id = if friend_id.contains('@') {
        friend_id.split('@').next().unwrap_or(&friend_id).to_string()
    } else {
        friend_id.clone()
    };
    
    emit_terminal_log(&app, &format!("Friend ID after cleaning: {}", clean_friend_id), "debug");
    
    // Form the conversation ID from summoner IDs
    // The conversation ID is formed by sorting the puuids and joining with underscore
    let mut ids = vec![my_puuid.to_string(), clean_friend_id];
    ids.sort();
    let conversation_id = ids.join("_");
    
    emit_terminal_log(&app, &format!("Using conversation_id: {}", conversation_id), "info");
    
    // Now use the conversation ID to get messages
    let url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages", port, conversation_id);
    emit_terminal_log(&app, &format!("Requesting messages from URL: {}", url), "debug");
    
    match client.get(&url)
        .header("Authorization", format!("Basic {}", auth))
        .send() 
    {
        Ok(resp) => {
            emit_terminal_log(&app, &format!("LCU API response status: {}", resp.status()), "debug");
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>() {
                    Ok(messages) => {
                        let msg_count = messages.as_array().map_or(0, |arr| arr.len());
                        emit_terminal_log(&app, &format!("Retrieved {} messages from conversation", msg_count), "info");
                        Ok(messages)
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to parse messages data: {}", e);
                        emit_terminal_log(&app, &error_msg, "error");
                        Err(error_msg)
                    }
                }
            } else {
                // If 404 or other error, return an empty array instead of error
                if resp.status() == 404 {
                    emit_terminal_log(&app, &format!("Conversation not found with ID: {}", conversation_id), "info");
                    emit_terminal_log(&app, "This could be normal for new conversations or if these users have never chatted. Returning empty array.", "info");
                    Ok(serde_json::json!([]))
                } else {
                    let error_msg = format!("LCU API returned error: {} when fetching messages", resp.status());
                    emit_terminal_log(&app, &error_msg, "error");
                    Err(error_msg)
                }
            }
        },
        Err(e) => {
            let error_msg = format!("Failed to connect to LCU API for messages: {}", e);
            emit_terminal_log(&app, &error_msg, "error");
            Err(error_msg)
        }
    }
}

// Create a persistent HTTP client to avoid recreating it every time
fn get_lcu_client() -> reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    }).clone()
}

// Helper function to get authentication details from lockfile
fn get_auth_from_lockfile(path: &PathBuf) -> Result<(String, String), String> {
    if let Ok(content) = fs::read_to_string(path) {
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() >= 5 {
            return Ok((parts[2].to_string(), parts[3].to_string()));
        }
    }
    
    Err("Failed to parse lockfile".to_string())
}

// Helper function to find the lockfile
fn find_lockfile(league_path: &str) -> Result<PathBuf, String> {
    let search_dirs = [PathBuf::from(league_path)];
    
    for dir in &search_dirs {
        for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
            let path = dir.join(name);
            if path.exists() {
                return Ok(path);
            }
        }
    }
    
    Err("Lockfile not found. Is League of Legends running?".to_string())
}

pub fn emit_terminal_log(app: &AppHandle, message: &str, log_type: &str) {
    let log = TerminalLog {
        message: message.to_string(),
        log_type: log_type.to_string(),
        timestamp: chrono::Local::now().to_rfc3339(),
    };
    let _ = app.emit("terminal-log", log);
}

// Add helper function for cleaner log messages
fn format_json_summary(json: &serde_json::Value) -> String {
    let mut summary = String::new();
    
    if let Some(phase) = json.get("phase") {
        summary.push_str(&format!("phase: {}, ", phase.as_str().unwrap_or("unknown")));
    }
    
    if let Some(game_data) = json.get("gameData") {
        summary.push_str("gameData: {...}, ");
    }
    
    if let Some(actions) = json.get("actions") {
        summary.push_str(&format!("actions: [{} items], ", actions.as_array().map_or(0, |a| a.len())));
    }
    
    if summary.is_empty() {
        summary = "[Response summary unavailable]".to_string();
    }
    
    summary
}

// Helper function for delayed logging
fn delayed_log(app: &AppHandle, message: &str) {
    emit_terminal_log(app, message, "debug");
    thread::sleep(Duration::from_millis(100)); // Small delay for better readability
}

pub fn get_swift_play_champion_selections(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    if let Some(game_data) = json.get("gameData") {
        if let Some(selections) = game_data.get("playerChampionSelections") {
            if let Some(selections_array) = selections.as_array() {
                for selection in selections_array {
                    if let Some(champion_id) = selection.get("championId").and_then(|v| v.as_i64()) {
                        if champion_id > 0 {
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        }
    }
    
    champion_ids
}

pub fn extract_swift_play_champions_from_lobby(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    if let Some(members) = json.get("members") {
        if let Some(members_array) = members.as_array() {
            for member in members_array {
                if let Some(champion_id) = member.get("championId").and_then(|v| v.as_i64()) {
                    if champion_id > 0 {
                        champion_ids.push(champion_id);
                    }
                }
            }
        }
    }
    
    champion_ids
}