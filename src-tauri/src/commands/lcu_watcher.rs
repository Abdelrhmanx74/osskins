use tauri::{AppHandle, Emitter, Manager};
use std::{thread, time::Duration};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use base64;
use serde_json;
use crate::injection::Skin;
use crate::commands::types::{SavedConfig, SkinData};

// LCU (League Client) watcher and communication

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
            
            let log_msg = format!("[LCU Watcher] Monitoring directory: {}", league_path_clone);
            println!("{}", log_msg);
            emit_terminal_log(&app_handle, &log_msg);
            
            // Only check the configured League directory for lockfile
            let search_dirs = [PathBuf::from(&league_path_clone)];
            let mut port = None;
            let mut token = None;
            let mut found_any_lockfile = false;
            let mut lockfile_path = None;
            
            // Rest of the lockfile detection code remains the same
            for dir in &search_dirs {
                let log_msg = format!("[LCU Watcher] Looking for lockfiles in: {}", dir.display());
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg);
                
                // Check each possible lockfile name
                for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
                    let path = dir.join(name);
                    if path.exists() {
                        found_any_lockfile = true;
                        lockfile_path = Some(path.clone());
                        println!("[LCU Watcher] Found lockfile: {}", path.display());
                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Found lockfile: {}", path.display()));
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
                        println!("[LCU Watcher] Error cleaning up injection: {}", e);
                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Error cleaning up injection: {}", e));
                    }
                    was_in_game = false;
                }
                
                let log_msg = format!("[LCU Watcher] No valid lockfile found. Is League running? The lockfile should be at: {}", league_path_clone);
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg);
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
                                            Err(e) => println!("[LCU Watcher] Failed to parse response from {}: {}", endpoint, e),
                                        }
                                    }
                                },
                                Err(e) => println!("[LCU Watcher] Failed to connect to endpoint {}: {}", endpoint, e),
                            }
                        }
                        
                        if (!connected) {
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                        
                        let phase = phase_value.unwrap_or_else(|| "None".to_string());
                        
                        if phase != last_phase {
                            println!("[LCU Watcher] LCU status changed: {} -> {}", last_phase, phase);
                            
                            // If entering ChampSelect, preload assets to speed up injection later
                            if phase == "ChampSelect" {
                                let champions_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."))
                                    .join("champions");
                                
                                if !champions_dir.exists() {
                                    if let Err(e) = fs::create_dir_all(&champions_dir) {
                                        println!("[LCU Watcher] Failed to create champions directory: {}", e);
                                    }
                                }
                                
                                let app_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."));
                                let overlay_dir = app_dir.join("overlay");
                                if overlay_dir.exists() {
                                    if let Err(e) = fs::remove_dir_all(&overlay_dir) {
                                        println!("[LCU Watcher] Failed to clean overlay directory: {}", e);
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
                                            emit_terminal_log(&app_handle, "[LCU Watcher] Updated skin selection tracking");
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
                            emit_terminal_log(&app_handle, "[LCU Watcher] Detected transition from Lobby to Matchmaking, checking for Swift Play mode");
                            
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
                                                emit_terminal_log(&app_handle, "[LCU Debug] Swift Play session structure:");
                                                // Print important paths in the JSON that might contain champion selections
                                                if let Some(game_data) = json.get("gameData") {
                                                    if let Some(queue) = game_data.get("queue") {
                                                        if let Some(queue_id) = queue.get("id").and_then(|id| id.as_i64()) {
                                                            emit_terminal_log(&app_handle, &format!("[LCU Debug] Queue ID: {}", queue_id));
                                                            
                                                            // Check if this is Swift Play queue (both ID 1700 and ID 480)
                                                            if queue_id == 1700 || queue_id == 480 {
                                                                emit_terminal_log(&app_handle, &format!("[LCU Watcher] Confirmed Swift Play queue or compatible queue (ID: {})", queue_id));
                                                                
                                                                // Log player selection data
                                                                if let Some(player_selections) = game_data.get("playerChampionSelections") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] playerChampionSelections: {}", player_selections));
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Debug] No playerChampionSelections found");
                                                                }
                                                                
                                                                if let Some(selected_champs) = game_data.get("selectedChampions") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] selectedChampions: {}", selected_champs));
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Debug] No selectedChampions found");
                                                                }
                                                                
                                                                // Check for local player data
                                                                if let Some(local_player) = json.get("localPlayerSelection") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] localPlayerSelection: {}", local_player));
                                                                }
                                                                
                                                                // Check for team data
                                                                if let Some(my_team) = json.get("myTeam") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] myTeam: {}", my_team));
                                                                }
                                                                
                                                                // Check for role assignments
                                                                if let Some(roles) = json.get("roleAssignments") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] roleAssignments: {}", roles));
                                                                }
                                                                
                                                                // Get player champion selections for Swift Play
                                                                let swift_play_champion_ids = get_swift_play_champion_selections(&json);
                                                                
                                                                if !swift_play_champion_ids.is_empty() {
                                                                    emit_terminal_log(&app_handle, &format!(
                                                                        "[LCU Watcher] Swift Play: Found {} champion selections: {:?}", 
                                                                        swift_play_champion_ids.len(), 
                                                                        swift_play_champion_ids
                                                                    ));
                                                                    
                                                                    // Inject skins for all selected champions
                                                                    inject_skins_for_champions(&app_handle, &league_path_clone, &swift_play_champion_ids);
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Watcher] Swift Play: No champion selections found in session data");
                                                                    
                                                                    // Try checking additional endpoints to find Swift Play champions
                                                                    let swift_play_url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
                                                                    match client.get(&swift_play_url)
                                                                        .header("Authorization", format!("Basic {}", auth))
                                                                        .send()
                                                                    {
                                                                        Ok(swift_resp) => {
                                                                            if swift_resp.status().is_success() {
                                                                                if let Ok(lobby_json) = swift_resp.json::<serde_json::Value>() {
                                                                                    emit_terminal_log(&app_handle, "[LCU Debug] Swift Play lobby data found");
                                                                                    
                                                                                    // Try to extract champion IDs from lobby data
                                                                                    let lobby_champion_ids = extract_swift_play_champions_from_lobby(&lobby_json);
                                                                                    
                                                                                    if !lobby_champion_ids.is_empty() {
                                                                                        emit_terminal_log(&app_handle, &format!(
                                                                                            "[LCU Watcher] Swift Play: Found {} champion selections from lobby: {:?}", 
                                                                                            lobby_champion_ids.len(), 
                                                                                            lobby_champion_ids
                                                                                        ));
                                                                                        
                                                                                        // Inject skins for all selected champions from lobby
                                                                                        inject_skins_for_champions(&app_handle, &league_path_clone, &lobby_champion_ids);
                                                                                    } else {
                                                                                        emit_terminal_log(&app_handle, "[LCU Watcher] Swift Play: No champion selections found in lobby data");
                                                                                        emit_terminal_log(&app_handle, &format!("[LCU Debug] Full lobby data: {}", 
                                                                                            serde_json::to_string_pretty(&lobby_json).unwrap_or_default()));
                                                                                    }
                                                                                }
                                                                            }
                                                                        },
                                                                        Err(e) => emit_terminal_log(&app_handle, &format!("[LCU Debug] Failed to get lobby data: {}", e)),
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                            Err(e) => println!("[LCU Watcher] Failed to parse queue info: {}", e),
                                        }
                                    }
                                },
                                Err(e) => println!("[LCU Watcher] Failed to get queue information: {}", e),
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

fn emit_terminal_log(app: &AppHandle, message: &str) {
    let _ = app.emit("terminal-log", message);
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
    emit_terminal_log(app, message);
    thread::sleep(Duration::from_millis(100)); // Small delay for better readability
}

// Helper function to get selected champion ID from session JSON
fn get_selected_champion_id(session_json: &serde_json::Value) -> Option<i64> {
    // Get local player cell ID
    if let Some(local_player_cell_id) = session_json.get("localPlayerCellId").and_then(|v| v.as_i64()) {
        // First, find our current active action
        if let Some(actions) = session_json.get("actions").and_then(|v| v.as_array()) {
            // Track if we found any pick in progress
            let mut has_pick_in_progress = false;
            
            // First pass: check if we have any pick in progress
            for action_group in actions.iter() {
                if let Some(actions) = action_group.as_array() {
                    for action in actions {
                        if let Some(actor_cell_id) = action.get("actorCellId").and_then(|v| v.as_i64()) {
                            if actor_cell_id == local_player_cell_id {
                                let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let is_in_progress = action.get("isInProgress").and_then(|v| v.as_bool()).unwrap_or(false);
                                
                                if action_type == "pick" && is_in_progress {
                                    has_pick_in_progress = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            
            // If we have a pick in progress, don't return any champion ID
            if has_pick_in_progress {
                return None;
            }
            
            // Second pass: look for completed pick
            for action_group in actions {
                if let Some(actions) = action_group.as_array() {
                    for action in actions {
                        if let Some(actor_cell_id) = action.get("actorCellId").and_then(|v| v.as_i64()) {
                            if actor_cell_id == local_player_cell_id {
                                let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let is_completed = action.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
                                let champion_id = action.get("championId").and_then(|v| v.as_i64()).unwrap_or(0);
                                
                                // Only return champion ID if:
                                // 1. It's a pick action (not ban)
                                // 2. Action is completed
                                // 3. Valid champion ID
                                if action_type == "pick" && is_completed && champion_id > 0 {
                                    return Some(champion_id);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // As a backup, check myTeam data, but only if we have a completed pick
        if let Some(my_team) = session_json.get("myTeam").and_then(|v| v.as_array()) {
            for player in my_team {
                if let Some(cell_id) = player.get("cellId").and_then(|v| v.as_i64()) {
                    if cell_id == local_player_cell_id {
                        let champion_id = player.get("championId").and_then(|v| v.as_i64()).unwrap_or(0);
                        let pick_intent = player.get("championPickIntent").and_then(|v| v.as_i64()).unwrap_or(0);
                        
                        // Only consider it selected if:
                        // 1. Has valid champion ID
                        // 2. No pick intent (not hovering)
                        if champion_id > 0 && pick_intent == 0 {
                            // Verify in actions that this is a completed pick
                            if let Some(actions) = session_json.get("actions").and_then(|v| v.as_array()) {
                                for action_group in actions {
                                    if let Some(actions) = action_group.as_array() {
                                        for action in actions {
                                            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                            let is_completed = action.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
                                            let act_champion_id = action.get("championId").and_then(|v| v.as_i64()).unwrap_or(0);
                                            let actor_cell_id = action.get("actorCellId").and_then(|v| v.as_i64());
                                            
                                            if action_type == "pick" && 
                                               is_completed && 
                                               act_champion_id == champion_id && 
                                               actor_cell_id == Some(local_player_cell_id) {
                                                return Some(champion_id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
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

// Helper function to get Swift Play champion selections from session JSON
fn get_swift_play_champion_selections(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    // Method 1: Look in gameData -> playerChampionSelections
    if let Some(game_data) = json.get("gameData") {
        if let Some(selections) = game_data.get("playerChampionSelections").and_then(|p| p.as_array()) {
            // Get local player's summoner ID first
            let local_summoner_id = json.get("localPlayerSelection")
                .and_then(|lp| lp.get("summonerId"))
                .and_then(|id| id.as_i64());
                
            if let Some(local_id) = local_summoner_id {
                for selection in selections {
                    // Check if this is the local player
                    if let Some(player_id) = selection.get("summonerId").and_then(|id| id.as_i64()) {
                        if player_id == local_id {
                            // Extract champion IDs
                            if let Some(champs) = selection.get("championIds").and_then(|ids| ids.as_array()) {
                                for champ in champs {
                                    if let Some(id) = champ.as_i64() {
                                        if id > 0 && !champion_ids.contains(&id) {
                                            champion_ids.push(id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Method 2: Look in gameData -> selectedChampions
    if champion_ids.is_empty() {
        if let Some(game_data) = json.get("gameData") {
            if let Some(selected_champions) = game_data.get("selectedChampions").and_then(|sc| sc.as_array()) {
                for selection in selected_champions {
                    if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        }
    }
    
    // Method 3: Look in the player's team data
    if champion_ids.is_empty() {
        if let Some(team) = json.get("myTeam").and_then(|t| t.as_array()) {
            let player_name = json.get("playerName").and_then(|p| p.as_str()).unwrap_or("");
            
            for player in team {
                let is_local_player = player.get("summonerName")
                    .and_then(|n| n.as_str())
                    .map_or(false, |name| name == player_name);
                
                if is_local_player {
                    // Primary champion
                    if let Some(champion_id) = player.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    }
                    
                    // Secondary champion
                    if let Some(secondary_id) = player.get("secondaryChampionId").and_then(|id| id.as_i64()) {
                        if secondary_id > 0 && !champion_ids.contains(&secondary_id) {
                            champion_ids.push(secondary_id);
                        }
                    }
                }
            }
        }
    }
    
    // Try one more method for Swift Play
    if champion_ids.is_empty() {
        if let Some(roles) = json.get("roleAssignments").and_then(|r| r.as_array()) {
            for role in roles {
                if let Some(champion_id) = role.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                        champion_ids.push(champion_id);
                    }
                }
            }
        }
    }
    
    // Method 4: Check lobby data playerSlots for Swift Play
    if champion_ids.is_empty() {
        // Try to find champions in localMember.playerSlots (common in Swift Play)
        if let Some(local_member) = json.get("localMember") {
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                for slot in player_slots {
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        }
    }
    
    champion_ids
}

// Helper function to inject skins for multiple champions (used in Swift Play)
fn inject_skins_for_champions(app: &AppHandle, league_path: &str, champion_ids: &[i64]) {
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let cfg_file = config_dir.join("config.json");
    
    // Check if we have config with skin selections
    if let Ok(data) = std::fs::read_to_string(&cfg_file) {
        if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
            // Get all skins for the selected champions
            let mut skins_to_inject = Vec::new();
            
            for champ_id in champion_ids {
                let champ_id_u32 = *champ_id as u32;
                if let Some(skin) = config.skins.iter().find(|s| s.champion_id == champ_id_u32) {
                    
                    skins_to_inject.push(Skin {
                        champion_id: skin.champion_id,
                        skin_id: skin.skin_id,
                        chroma_id: skin.chroma_id,
                        fantome_path: skin.fantome.clone(),
                    });
                }
            }
            
            // If we found skins to inject, do it
            if !skins_to_inject.is_empty() {
                
                let champions_dir = app.path().app_data_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("champions");
                
                match crate::injection::inject_skins(
                    app,
                    league_path,
                    &skins_to_inject,
                    &champions_dir
                ) {
                    Ok(_) => {
                        let _ = app.emit("injection-status", "success");
                    },
                    Err(e) => {
                        let _ = app.emit("skin-injection-error", format!(
                            "Failed to inject Swift Play skins: {}", e
                        ));
                        let _ = app.emit("injection-status", "error");
                    }
                }
            }
        }
    }
}
// Extract Swift Play champion IDs from the lobby data directly
fn extract_swift_play_champions_from_lobby(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    if let Some(local_member) = json.get("localMember") {
        if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
            for slot in player_slots {
                if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                        champion_ids.push(champion_id);
                    }
                }
            }
        }
    }
    
    champion_ids
}
