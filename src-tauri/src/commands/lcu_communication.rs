use crate::commands::types::*;
use tauri::{AppHandle};
use std::fs;
use reqwest;
use base64::Engine;
use std::sync::OnceLock;
use std::time::Duration;
use serde_json;
use std::path::PathBuf;
use crate::commands::skin_management::{get_selected_champions_universal, inject_skins_for_champions, is_multi_champion_mode};

#[tauri::command]
pub fn start_lcu_watcher(app: AppHandle, league_path: String) -> Result<(), String> {
    println!("Starting LCU status watcher for path: {}", league_path);
    let app_handle = app.clone();
    let league_path_clone = league_path.clone();
    std::thread::spawn(move || {
        let mut last_phase = String::new();
        let _was_in_game = false;
        let _was_reconnecting = false;
        let _last_skin_check_time = std::time::Instant::now();
        let mut _sleep_duration = Duration::from_secs(5);
        let mut last_champion_ids: Vec<i64> = Vec::new();
        let client = get_lcu_client();
        let mut last_lockfile_found = false;
        loop {
            _sleep_duration = Duration::from_secs(5);
            // Only print if lockfile state changes
            let mut found_any_lockfile = false;
            let mut _lockfile_path = None;
            let mut port = None;
            let mut token = None;
            for dir in [PathBuf::from(&league_path_clone)].iter() {
                // Only print if lockfile state changes
                for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
                    let path = dir.join(name);
                    if path.exists() {
                        found_any_lockfile = true;
                        _lockfile_path = Some(path.clone());
                        if !last_lockfile_found {
                            println!("Found lockfile: {}", path.display());
                        }
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
            if found_any_lockfile != last_lockfile_found {
                if found_any_lockfile {
                    println!("Lockfile detected in directory: {}", league_path_clone);
                } else {
                    println!("Lockfile lost in directory: {}", league_path_clone);
                }
                last_lockfile_found = found_any_lockfile;
            }
            if !found_any_lockfile || port.is_none() || token.is_none() {
                println!("[LCU] No valid lockfile found or missing port/token. Skipping this cycle.");
                std::thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
            let port = port.as_ref().unwrap();
            let token = token.as_ref().unwrap();
            // Phase tracking: get current phase from /lol-gameflow/v1/gameflow-phase
            let phase_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/gameflow-phase", port);
            let auth = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", token));
            let mut phase = String::from("None");
            match client.get(&phase_url)
                .header("Authorization", format!("Basic {}", auth))
                .send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(phase_str) = resp.text() {
                            phase = phase_str.trim_matches('"').to_string();
                        }
                    }
                },
                Err(e) => println!("[LCU API Debug] Failed to fetch gameflow-phase: {}", e),
            }
            if !last_phase.is_empty() && last_phase != phase {
                println!("[LCU Phase] Transition: {} -> {}", last_phase, phase);
            }
            // Detect Swift Play mode more accurately with queue IDs
            let is_swift_play = {
                // Check if any endpoint returns a Swift Play mode
                let url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", port);
                let mut swift_play = false;
                match client.get(&url)
                    .header("Authorization", format!("Basic {}", auth))
                    .send() {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(json) = resp.json::<serde_json::Value>() {
                                // First try to detect by queue ID (most reliable)
                                let queue_id = json.get("gameData")
                                    .and_then(|d| d.get("queue"))
                                    .and_then(|q| q.get("id"))
                                    .and_then(|id| id.as_i64())
                                    .unwrap_or_else(|| {
                                        json.get("gameConfig")
                                            .and_then(|c| c.get("queueId"))
                                            .and_then(|id| id.as_i64())
                                            .unwrap_or(0)
                                    });
                                
                                // Swift Play modes (480, 1700) 
                                if queue_id == 1700 || queue_id == 480 {
                                    println!("[Swift Play Detection] Detected Swift Play by queue ID: {}", queue_id);
                                    swift_play = true;
                                } else {
                                    let mode = detect_game_mode(&json);
                                    swift_play = mode.to_uppercase().contains("SWIFT") || 
                                                mode.to_uppercase().contains("ARENA");
                                    if swift_play {
                                        println!("[Swift Play Detection] Detected Swift Play by mode name: {}", mode);
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => {}
                }
                swift_play
            };
            let mut champion_ids_found = false;
            let mut champion_ids: Vec<i64> = Vec::new();
            if is_swift_play {
                // For Swift Play, we need to consider multiple conditions for injection:
                // 1. Phase transitions indicate selection finalization
                // 2. Being in Matchmaking or related phases with champions already means they're locked in
                // 3. We must inject continuously during Matchmaking phase to catch all champion selections
                let is_transition_to_inject = (last_phase == "Lobby" && phase == "Matchmaking") ||
                                             (last_phase == "Matchmaking" && phase == "InProgress") ||
                                             (last_phase == "ChampSelect" && phase == "InProgress") ||
                                             (last_phase == "None" && phase == "InProgress") ||
                                             // Also inject when directly entering InProgress from any state
                                             (phase == "InProgress" && last_phase != "InProgress") || 
                                             // Also inject during Preparing phase (pre-game)
                                             (phase == "Preparing" && last_phase != "Preparing") ||
                                             // CRITICAL: Important to also inject during Matchmaking without transition
                                             // This addresses the case where the champion is selected but not being injected
                                             (phase == "Matchmaking") ||
                                             // Make sure we're continuously injecting during Swift Play phases
                                             (is_swift_play && (phase == "Matchmaking" || phase == "Preparing" || phase == "InProgress"));
                
                println!("[Swift Play] Phase transition check: {} -> {} = {} (including matchmaking auto-inject)", 
                        last_phase, phase, is_transition_to_inject);
                
                if is_transition_to_inject {
                    // Use the lobby endpoint for Swift Play champion selection
                    let url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
                    println!("[Swift Play] Fetching Swift Play lobby data from: {}", url);
                    
                    match client.get(&url)
                        .header("Authorization", format!("Basic {}", auth))
                        .send() {
                        Ok(resp) => {
                            println!("[Swift Play] Got response status: {}", resp.status());
                            if resp.status().is_success() {
                                if let Ok(json) = resp.json::<serde_json::Value>() {
                                    let mode = detect_game_mode(&json);
                                    println!("[Swift Play] Phase transition: {} -> {}", last_phase, phase);
                                    println!("[Swift Play] Detected game mode: {}", mode);
                                    
                                    // Extract confirmed champions, not just hovered ones
                                    let ids = extract_lobby_champions(&json, &mode);
                                    
                                    if !ids.is_empty() {
                                        champion_ids = ids;
                                        champion_ids_found = true;
                                        println!("[Swift Play] Found locked-in champions: {:?}", champion_ids);
                                    } else {
                                        println!("[Swift Play] No locked-in champions found");
                                        
                                        // Detailed logging of available champion data
                                        if let Some(local_member) = json.get("localMember") {
                                            if let Some(player_slots) = local_member.get("playerSlots") {
                                                println!("[Swift Play Debug] Player slots available: {}", 
                                                         serde_json::to_string_pretty(player_slots).unwrap_or_default());
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => println!("[Swift Play] Failed to fetch lobby data: {}", e),
                    }
                }
            } else {
                // For other modes, use the normal detection logic but with special handling for matchmaking
                let is_matchmaking_phase = phase == "Matchmaking" || phase == "Preparing";
                
                // For any matchmaking phase, we should always get and debug data to help troubleshoot
                if is_matchmaking_phase {
                    println!("[Matchmaking] Detected matchmaking phase in non-Swift Play mode: {}", phase);
                    
                    // Use the lobby endpoint first for matchmaking phase
                    let url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
                    println!("[Matchmaking] Fetching lobby data from: {}", url);
                    
                    match client.get(&url)
                        .header("Authorization", format!("Basic {}", auth))
                        .send() {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                if let Ok(json) = resp.json::<serde_json::Value>() {
                                    // Extract confirmed champions
                                    let mode = detect_game_mode(&json);
                                    let ids = extract_lobby_champions(&json, &mode);
                                    
                                    if !ids.is_empty() {
                                        champion_ids = ids;
                                        champion_ids_found = true;
                                        println!("[Matchmaking] Found champions: {:?}", champion_ids);
                                    } else {
                                        println!("[Matchmaking] No champions found in lobby data");
                                    }
                                }
                            }
                        },
                        Err(e) => println!("[Matchmaking] Failed to fetch lobby data: {}", e),
                    }
                }
                
                // If we didn't find champions yet, try the standard endpoints
                if !champion_ids_found {
                    let endpoints = [
                        "/lol-champ-select/v1/session",
                        "/lol-gameflow/v1/session",
                        "/lol-lobby/v2/lobby",
                    ];
                    for endpoint in endpoints.iter() {
                        let url = format!("https://127.0.0.1:{}{}", port, endpoint);
                        match client.get(&url)
                            .header("Authorization", format!("Basic {}", auth))
                            .send() {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    if let Ok(json) = resp.json::<serde_json::Value>() {
                                        let ids = get_selected_champions_universal(&json);
                                        if !ids.is_empty() {
                                            champion_ids = ids;
                                            champion_ids_found = true;
                                            break;
                                        }
                                    }
                                }
                            },
                            Err(e) => println!("[LCU API Debug] Failed to fetch {}: {}", endpoint, e),
                        }
                    }
                }
            }
            // If we found champion IDs, check if they're locked in before injecting
            if champion_ids_found && !champion_ids.is_empty() {
                // Check if this is a champion that's locked in, not just hovered
                let is_champion_locked = if is_swift_play {
                    // For Swift Play, champions are always considered locked in several phases
                    if phase == "Matchmaking" || phase == "InProgress" || phase == "GameStart" || phase == "Preparing" {
                        println!("[Swift Play] Champions are AUTO-LOCKED in phase: {}", phase);
                        true
                    } else {
                        // For other phases, we need to check more specifically
                        println!("[Swift Play] Phase {} doesn't guarantee locked champions", phase);
                        
                        // For Swift Play, we need to check the lobby data for confirmation
                        let is_swift_play_confirmed = match client.get(&format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port))
                            .header("Authorization", format!("Basic {}", auth))
                            .send()
                            .ok()
                            .and_then(|resp| resp.status().is_success().then(|| resp.json::<serde_json::Value>().ok()).flatten())
                        {
                            Some(json) => {
                                // Check if member is ready or if matchmaking has started
                                let is_ready = json.get("localMember")
                                    .and_then(|m| m.get("ready"))
                                    .and_then(|r| r.as_bool())
                                    .unwrap_or(false);
                                    
                                let state = json.get("state")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("");
                                    
                                println!("[Swift Play] Lobby check - ready: {}, state: {}", is_ready, state);
                                
                                // Consider ready or any active matchmaking state as confirmation
                                let state_confirmed = is_ready || 
                                                     state == "MATCHMAKING" || 
                                                     state == "GAMESTARTING" || 
                                                     state == "PREPARING";
                                
                                // Also check if we have valid champion IDs in player slots
                                let has_champion_slots = json.get("localMember")
                                    .and_then(|m| m.get("playerSlots"))
                                    .and_then(|slots| slots.as_array())
                                    .map(|arr| !arr.is_empty() && arr.iter().any(|slot| 
                                        slot.get("championId")
                                        .and_then(|id| id.as_i64())
                                        .map_or(false, |id| id > 0)
                                    ))
                                    .unwrap_or(false);
                                    
                                if has_champion_slots {
                                    println!("[Swift Play] Found valid champion IDs in player slots");
                                    // Important: In Swift Play, having champion IDs in slots is already a confirmation
                                    // Return true immediately when we have slots with champions
                                    return true;
                                }
                                
                                state_confirmed || has_champion_slots
                            },
                            None => false
                        };
                        
                        is_swift_play_confirmed
                    }
                } else if phase == "ChampSelect" {
                    // In champion select, verify it's a locked pick, not just a hover
                    let champ_select_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
                    println!("[Champion Lock Check] Checking if champion is locked in ChampSelect phase");
                    
                    let locked_picks = client.get(&champ_select_url)
                        .header("Authorization", format!("Basic {}", auth))
                        .send()
                        .ok()
                        .and_then(|resp| {
                            if resp.status().is_success() {
                                resp.json::<serde_json::Value>().ok()
                            } else {
                                None
                            }
                        })
                        .and_then(|json| {
                            // Check if there are completed actions for the local player
                            let mut found_locked = false;
                            if let Some(actions) = json.get("actions").and_then(|a| a.as_array()) {
                                let local_cell_id = json.get("localPlayerCellId").and_then(|id| id.as_i64());
                                println!("[ChampSelect] Local player cell ID: {:?}", local_cell_id);
                                
                                for (i, action_group) in actions.iter().enumerate() {
                                    if let Some(action_list) = action_group.as_array() {
                                        for (j, action) in action_list.iter().enumerate() {
                                            let is_completed = action.get("completed").and_then(|c| c.as_bool()).unwrap_or(false);
                                            let actor_cell_id = action.get("actorCellId").and_then(|id| id.as_i64());
                                            let is_local_player = actor_cell_id == local_cell_id;
                                            let action_type = action.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                            
                                            println!("[ChampSelect] Action {}.{}: type={}, completed={}, local={}", 
                                                    i, j, action_type, is_completed, is_local_player);
                                            
                                            if is_completed && is_local_player && action_type == "pick" {
                                                println!("[ChampSelect] Found locked pick for local player");
                                                found_locked = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            Some(found_locked)
                        })
                        .unwrap_or(false);
                    
                    println!("[Champion Lock Check] Is champion locked in ChampSelect: {}", locked_picks);
                    locked_picks
                } else {
                    // For game phases like InProgress, Matchmaking, etc. champions are definitely locked
                    println!("[Champion Lock Check] Phase {} indicates champions are locked", phase);
                    phase == "InProgress" || phase == "GameStart" || phase == "Matchmaking" || phase == "Preparing"
                };
                
                // Only inject if champions are locked in AND the champions have changed since last injection
                if is_champion_locked && champion_ids != last_champion_ids {
                    // Filter out any invalid/zero champion IDs as an extra precaution
                    let valid_champion_ids: Vec<i64> = champion_ids.iter()
                        .filter(|&&id| id > 0)
                        .cloned()
                        .collect();
                        
                    // Double check we actually have valid champions
                    if !valid_champion_ids.is_empty() {
                        println!("[LCU Detection] INJECTING skins for champion IDs: {:?}", valid_champion_ids);
                        println!("[LCU Detection] Game mode: {} | Phase: {} | Swift Play: {} | Brawl: {}", 
                                 detect_game_mode(&serde_json::json!({})), phase, is_swift_play, !is_swift_play);
                        
                        // Log the injection parameters in detail for debugging
                        println!("[INJECTION EVENT] ===================================");
                        println!("[INJECTION EVENT] Champion IDs: {:?}", valid_champion_ids);
                        println!("[INJECTION EVENT] Game phase: {}", phase);
                        println!("[INJECTION EVENT] Previous phase: {}", last_phase);
                        println!("[INJECTION EVENT] Swift Play mode: {}", is_swift_play);
                        println!("[INJECTION EVENT] ===================================");
                        
                        // Get and analyze the current game session one more time before injection
                        match client.get(&format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port))
                            .header("Authorization", format!("Basic {}", auth))
                            .send()
                            .ok()
                            .and_then(|resp| resp.status().is_success().then(|| resp.json::<serde_json::Value>().ok()).flatten()) {
                            Some(json) => {
                                if is_swift_play {
                                    // Run detailed Swift Play debugging
                                    // debug_swift_play_session(&json, &phase);
                                } 
                                
                                // For all game modes, debug matchmaking data if in matchmaking phase
                                if phase == "Matchmaking" {
                                    // debug_matchmaking_session(&json, &phase);
                                }
                            },
                            None => {}
                        }
                        
                        // Log the actual injection
                        inject_skins_for_champions(&app_handle, &league_path_clone, &valid_champion_ids);
                        last_champion_ids = valid_champion_ids.clone();
                    } else {
                        println!("[LCU Detection] No valid champion IDs found after filtering: {:?}", champion_ids);
                    }
                } else if !is_champion_locked {
                    println!("[LCU Detection] Champions found but NOT LOCKED IN: {:?}, phase: {}", 
                             champion_ids, phase);
                             
                    // Special safety check: if we're in matchmaking phase but champions aren't locked,
                    // force the injection anyway to handle edge cases like blind pick where client might not show lock
                    if phase == "Matchmaking" && !champion_ids.is_empty() && 
                       champion_ids != last_champion_ids && champion_ids.iter().all(|&id| id > 0) {
                        println!("[Matchmaking SAFETY OVERRIDE] Force injecting champions in matchmaking phase: {:?}", champion_ids);
                        
                        // Log the injection parameters in detail for debugging
                        println!("[INJECTION EVENT] ===================================");
                        println!("[INJECTION EVENT] SAFETY OVERRIDE - Champion IDs: {:?}", champion_ids);
                        println!("[INJECTION EVENT] Game phase: {}", phase);
                        println!("[INJECTION EVENT] Previous phase: {}", last_phase);
                        println!("[INJECTION EVENT] ===================================");
                        
                        // Log the actual injection
                        inject_skins_for_champions(&app_handle, &league_path_clone, &champion_ids);
                        last_champion_ids = champion_ids.clone();
                    }
                } else if champion_ids == last_champion_ids {
                    println!("[LCU Detection] Champions unchanged from last injection: {:?}", champion_ids);
                }
            } else if champion_ids_found {
                println!("[LCU Detection] No valid champions found to inject");
            } else {
                // Only log if there is an actual error (not just no champion IDs found)
                // The check for found_any_lockfile, port, and token must be done before unwrap, so we skip logging here
                // Otherwise, do not log repeated 'no champion IDs' messages
            }
            // Use a shorter polling interval during any matchmaking phase for faster champion detection
            let polling_interval = if phase == "Matchmaking" {
                if is_swift_play {
                    println!("[Swift Play] Using faster polling in matchmaking phase");
                } else {
                    println!("[Matchmaking] Using faster polling in matchmaking phase");
                }
                std::time::Duration::from_millis(1000) // 1 second polling during matchmaking
            } else {
                std::time::Duration::from_secs(2) // 2 seconds for normal polling
            };
            
            last_phase = phase;
            std::thread::sleep(polling_interval);
        }
    });                       

    println!("LCU status watcher thread started");
    Ok(())
}

// New command to get the friends list from LCU
#[tauri::command]
pub fn get_lcu_friends(app: AppHandle, league_path: String) -> Result<Vec<Friend>, String> {
    let _app = app;
    // Find the lockfile to get auth details
    let lockfile_path = find_lockfile(&league_path)?;
    let (port, token) = get_auth_from_lockfile(&lockfile_path)?;
    
    let client = get_lcu_client();
    let url = format!("https://127.0.0.1:{}/lol-chat/v1/friends", port);
    let auth = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", token));
    
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
                        
                        println!("Found {} friends in LCU", valid_friends.len());
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
        let _app = app;
        // Find the lockfile to get auth details
        let lockfile_path = find_lockfile(&league_path)?;
        let (port, token) = get_auth_from_lockfile(&lockfile_path)?;
        
        let client = get_lcu_client();
        let url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages", port, friend_id);
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", token));
        
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
                    println!("Message sent to friend {}", friend_id);
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
        let _app = app;
        // Find the lockfile to get auth details
        let lockfile_path = find_lockfile(&league_path)?;
        let (port, token) = get_auth_from_lockfile(&lockfile_path)?;
        
        let client = get_lcu_client();
        
        // First, get the summoner ID for the local player to form the conversation ID
        let summoner_url = format!("https://127.0.0.1:{}/lol-summoner/v1/current-summoner", port);
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", token));
        
        println!("Requesting current summoner data...");
        let my_summoner = match client.get(&summoner_url)
            .header("Authorization", format!("Basic {}", auth))
            .send() 
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>() {
                        Ok(data) => data,
                        Err(e) => {
                            println!("Failed to parse summoner data: {}", e);
                            return Err(format!("Failed to parse summoner data: {}", e));
                        }
                    }
                } else {
                    let error_msg = format!("LCU API returned error: {} when fetching summoner data", resp.status());
                    println!("{}", error_msg);
                    return Err(error_msg);
                }
            },
            Err(e) => {
                let error_msg = format!("Failed to connect to LCU API for summoner data: {}", e);
                println!("{}", error_msg);
                return Err(error_msg);
            }
        };
        
        // Get the local summoner's ID (puuid)
        let my_puuid = match my_summoner.get("puuid") {
            Some(id) => id.as_str().unwrap_or(""),
            None => {
                println!("Failed to get current summoner's puuid");
                return Err("Failed to get current summoner's puuid".to_string());
            }
        };
        
        if my_puuid.is_empty() {
            println!("Invalid summoner puuid (empty string)");
            return Err("Invalid summoner puuid".to_string());
        }
        
        println!("Local summoner PUUID: {}", my_puuid);
        println!("Friend ID with suffix: {}", friend_id);
        
        // Clean the friend ID by removing the server suffix (e.g., @eu1.pvp.net)
        let clean_friend_id = if friend_id.contains('@') {
            friend_id.split('@').next().unwrap_or(&friend_id).to_string()
        } else {
            friend_id.clone()
        };
        
        println!("Friend ID after cleaning: {}", clean_friend_id);
        
        // Form the conversation ID from summoner IDs
        // The conversation ID is formed by sorting the puuids and joining with underscore
        let mut ids = vec![my_puuid.to_string(), clean_friend_id];
        ids.sort();
        let conversation_id = ids.join("_");
        
        println!("Using conversation_id: {}", conversation_id);
        
        // Now use the conversation ID to get messages
        let url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages", port, conversation_id);
        println!("Requesting messages from URL: {}", url);
        
        match client.get(&url)
            .header("Authorization", format!("Basic {}", auth))
            .send() 
        {
            Ok(resp) => {
                println!("LCU API response status: {}", resp.status());
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>() {
                        Ok(messages) => {
                            let msg_count = messages.as_array().map_or(0, |arr| arr.len());
                            println!("Retrieved {} messages from conversation", msg_count);
                            Ok(messages)
                        },
                        Err(e) => {
                            let error_msg = format!("Failed to parse messages data: {}", e);
                            println!("{}", error_msg);
                            Err(error_msg)
                        }
                    }
                } else {
                    // If 404 or other error, return an empty array instead of error
                    if resp.status() == 404 {
                        println!("Conversation not found with ID: {}", conversation_id);
                        println!("This could be normal for new conversations or if these users have never chatted. Returning empty array.");
                        Ok(serde_json::json!([]))
                    } else {
                        let error_msg = format!("LCU API returned error: {} when fetching messages", resp.status());
                        println!("{}", error_msg);
                        Err(error_msg)
                    }
                }
            },
            Err(e) => {
                let error_msg = format!("Failed to connect to LCU API for messages: {}", e);
                println!("{}", error_msg);
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

// Helper to extract game mode from LCU session/lobby JSON
fn detect_game_mode(json: &serde_json::Value) -> String {
    // Try to get queue ID first since it's the most reliable indicator
    let queue_id = json.get("gameData")
        .and_then(|d| d.get("queue"))
        .and_then(|q| q.get("id"))
        .and_then(|id| id.as_i64())
        .unwrap_or_else(|| {
            // Try alternate location for queue ID
            json.get("queue")
                .and_then(|q| q.get("id"))
                .and_then(|id| id.as_i64())
                .unwrap_or_else(|| {
                    // Try in gameConfig
                    json.get("gameConfig")
                        .and_then(|c| c.get("queueId"))
                        .and_then(|id| id.as_i64())
                        .unwrap_or(0)
                })
        });
    
    // Use queue ID to identify specific modes
    if queue_id > 0 {
        match queue_id {
            480 | 1700 => return "SWIFT_PLAY".to_string(),
            1300 | 2300 => return "BRAWL".to_string(),
            // Add more queue IDs as needed
            _ => {
                println!("[Game Mode Detection] Unknown queue ID: {}", queue_id);
                // Continue to other detection methods
            }
        }
    }
    
    // Try to get game mode from gameData.queue.gameMode
    if let Some(game_data) = json.get("gameData") {
        if let Some(game_mode) = game_data.get("queue")
            .and_then(|q| q.get("gameMode"))
            .and_then(|m| m.as_str())
        {
            return game_mode.to_string();
        }
    }
    
    // Try to get game mode from map.gameMode
    if let Some(map) = json.get("map") {
        if let Some(game_mode) = map.get("gameMode").and_then(|m| m.as_str()) {
            return game_mode.to_string();
        }
    }
    
    // Fall back to phase
    if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
        return phase.to_string();
    }
    
    "UNKNOWN".to_string()
}

// Helper to extract champion IDs from lobby JSON for all modes
fn extract_lobby_champions(json: &serde_json::Value, mode: &str) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    // Determine if this is a multi-champion mode by analyzing properties
    let is_multi_champion_mode = is_multi_champion_mode(json, Some(mode));
    println!("[Mode Detection] Multi-champion mode detected: {}", is_multi_champion_mode);
    
    // Extract the queue ID to explicitly detect Swift Play (480, 1700) vs Brawl (1300, 2300)
    let queue_id = json.get("gameData")
        .and_then(|d| d.get("queue"))
        .and_then(|q| q.get("id"))
        .and_then(|id| id.as_i64())
        .unwrap_or_else(|| {
            json.get("gameConfig")
                .and_then(|c| c.get("queueId"))
                .and_then(|id| id.as_i64())
                .unwrap_or(0)
        });
        
    println!("[Queue Detection] Queue ID: {}", queue_id);
    
    // Explicitly check if this is Brawl mode - more specific queue ID detection
    let is_brawl_mode = queue_id == 1300 || queue_id == 2300 || 
                       mode.to_uppercase().contains("BRAWL");
                       
    // Check if this is Swift Play mode - more specific queue ID detection
    let is_swift_play = queue_id == 1700 || queue_id == 480 || 
                      mode.to_uppercase().contains("SWIFT") || 
                      mode.to_uppercase().contains("ARENA");
                       
    println!("[Mode Detection] Is Brawl mode: {}, Is Swift Play: {}", is_brawl_mode, is_swift_play);
    
    // SPECIAL CASE: In Matchmaking phase, always consider champions as locked
    let state = json.get("state").and_then(|s| s.as_str()).unwrap_or("");
    let in_matchmaking = state == "MATCHMAKING" || state == "GAMESTARTING" || state == "PREPARING";
    
    // Auto-confirmation flags for different game modes
    let swift_play_auto_confirm = is_swift_play && (in_matchmaking || state == "INPROGRESS");
    // Also auto-confirm for other modes in matchmaking
    let matchmaking_auto_confirm = in_matchmaking || state == "INPROGRESS";
    
    if swift_play_auto_confirm || matchmaking_auto_confirm {
        // For Swift Play, we use special logging
        if is_swift_play {
            println!("[Swift Play Override] AUTO-CONFIRMING all champions in {} phase", state);
        } else {
            println!("[Matchmaking Override] AUTO-CONFIRMING all champions in {} phase", state);
        }
        
        // Directly extract champions from playerSlots when in any matchmaking phase
        if let Some(local_member) = json.get("localMember") {
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                println!("[Swift Play Override] Found {} player slots in {}", player_slots.len(), state);
                
                for (i, slot) in player_slots.iter().enumerate() {
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            println!("[Swift Play Override] Auto-adding champion {} from slot {}", champion_id, i);
                            champion_ids.push(champion_id);
                        }
                    }
                }
                
                // If we found champions with the override, return them immediately
                if !champion_ids.is_empty() {
                    println!("[Swift Play Override] Returning {} auto-confirmed champions: {:?}", 
                             champion_ids.len(), champion_ids);
                    return champion_ids;
                }
            }
            
            // Also try direct championId in Swift Play auto-confirm mode
            if champion_ids.is_empty() {
                if let Some(champion_id) = local_member.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 {
                        println!("[Swift Play Override] Found champion directly on localMember: {}", champion_id);
                        champion_ids.push(champion_id);
                        return champion_ids;
                    }
                }
            }
        }
        
        // Another fallback for Swift Play: check gameData.playerChampionSelections
        if champion_ids.is_empty() {
            if let Some(game_data) = json.get("gameData") {
                if let Some(selections) = game_data.get("playerChampionSelections").and_then(|s| s.as_array()) {
                    println!("[Swift Play Override] Checking playerChampionSelections: {} entries", selections.len());
                    
                    for (i, selection) in selections.iter().enumerate() {
                        if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                            if champion_id > 0 && !champion_ids.contains(&champion_id) {
                                println!("[Swift Play Override] Auto-confirming champion {} from selection {}", champion_id, i);
                                champion_ids.push(champion_id);
                            }
                        }
                    }
                    
                    if !champion_ids.is_empty() {
                        println!("[Swift Play Override] Returning {} champions from gameData: {:?}", 
                                champion_ids.len(), champion_ids);
                        return champion_ids;
                    }
                }
            }
        }
    }
    
    // Look for confirmation that selections are locked/confirmed - more stringent criteria
    let is_confirmed_selection = 
        // The ready flag is a strong indicator in Swift Play matchmaking
        json.get("localMember")
            .and_then(|m| m.get("ready"))
            .and_then(|r| r.as_bool())
            .unwrap_or(false) ||
        // State being matchmaking or further along is another strong indicator
        json.get("state")
            .and_then(|state| state.as_str())
            .map(|s| s == "MATCHMAKING" || s == "GAMESTARTING" || s == "INPROGRESS")
            .unwrap_or(false) ||
        // Check if game phase indicates we're past champion selection
        json.get("gameData")
            .and_then(|d| d.get("gameState"))
            .and_then(|s| s.as_str())
            .map(|s| s == "IN_PROGRESS" || s == "PRE_GAME")
            .unwrap_or(false);
    
    println!("[Champion Detection] Is confirmed selection: {}", is_confirmed_selection);
    
    // Auto-confirm flags for different game modes
    let auto_confirm_swift_play = is_swift_play && (
        // Check if we're in the matchmaking phase
        json.get("state")
            .and_then(|state| state.as_str())
            .map(|s| s == "MATCHMAKING" || s == "GAMESTARTING" || s == "INPROGRESS")
            .unwrap_or(false)
    );
    
    // Auto-confirm for any game mode in matchmaking phase
    let auto_confirm_matchmaking = json.get("state")
        .and_then(|state| state.as_str())
        .map(|s| s == "MATCHMAKING" || s == "GAMESTARTING" || s == "INPROGRESS")
        .unwrap_or(false);
    
    if auto_confirm_swift_play {
        println!("[Swift Play] Auto-confirming champion selections in matchmaking phase");
    } else if auto_confirm_matchmaking {
        println!("[Matchmaking] Auto-confirming champion selections in matchmaking phase");
    }
    
    // First, always try to find the local player's champion (primary champion)
    if let Some(local_member) = json.get("localMember") {
        // Try in playerSlots first (most reliable)
        if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
            println!("[Champion Detection] Found {} player slots", player_slots.len());
            
            // For Brawl mode, we should only consider the first champion
            let slots_to_check = if is_brawl_mode {
                println!("[Brawl Mode] Only considering first champion slot");
                std::cmp::min(1, player_slots.len())
            } else {
                if is_swift_play {
                    println!("[Swift Play Mode] Considering all {} champion slots", player_slots.len());
                } else {
                    println!("[Standard Mode] Considering all champion slots");
                }
                player_slots.len()
            };
            
            for i in 0..slots_to_check {
                if let Some(slot) = player_slots.get(i) {
                    println!("[Slot Analysis] Analyzing slot {}: {:?}", i, slot);
                    
                    // Check if this is a locked selection using multiple indicators - be more stringent
                    let selection_status = slot.get("selectionStatus")
                        .and_then(|status| status.as_str())
                        .unwrap_or("UNKNOWN");
                        
                    // Get the champion ID first to use it in our checks
                    let champion_id = slot.get("championId").and_then(|id| id.as_i64()).unwrap_or(0);
                    
                    // In any matchmaking phase, ANY champion in a slot should be considered locked
                    let is_slot_locked = selection_status == "SELECTED" || 
                                        selection_status == "LOCKED" ||
                                        is_confirmed_selection || // Fall back to global confirmation
                                        auto_confirm_swift_play || // Auto-confirm in Swift Play matchmaking
                                        matchmaking_auto_confirm || // Auto-confirm in any matchmaking phase
                                        (is_swift_play && champion_id > 0) || // In Swift Play, any non-zero champion is valid
                                        (in_matchmaking && champion_id > 0); // In any matchmaking phase, any non-zero champion is valid
                    
                    // We already extracted champion_id above, so just use it directly
                    println!("[Champion Analysis] Slot {} has champion ID: {}, selection status: {}, locked: {}", 
                            i, champion_id, selection_status, is_slot_locked);
                    
                    if champion_id > 0 && is_slot_locked {
                        println!("[Lobby Champion] Adding champion in slot {} ({}), locked: {}", i, champion_id, is_slot_locked);
                        
                        // Only add if not already in the list
                        if !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    } else if champion_id > 0 {
                        println!("[Lobby Champion] Skipping unlocked champion in slot {}: {}", i, champion_id);
                    }
                }
            }
        }
        
        // Fallback: try direct championId on localMember if we're in a confirmed state
        if champion_ids.is_empty() && is_confirmed_selection {
            if let Some(champion_id) = local_member.get("championId").and_then(|id| id.as_i64()) {
                if champion_id > 0 {
                    println!("[Lobby Champion] Found confirmed champion on localMember: {}", champion_id);
                    champion_ids.push(champion_id);
                }
            }
        }
    }
    
    // For multi-champion modes like Swift Play, add all additional champions
    if is_multi_champion_mode {
        // Get additional champions from local member slots (beyond the first)
        if let Some(local_member) = json.get("localMember") {
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                for slot in player_slots.iter().skip(1) {
                    // Check if this additional slot is locked
                    let is_slot_locked = slot.get("selectionStatus")
                        .and_then(|status| status.as_str())
                        .map(|s| s == "SELECTED" || s == "LOCKED")
                        .unwrap_or(is_confirmed_selection) || // Fall back to global confirmation
                        (is_swift_play && is_multi_champion_mode); // In Swift Play, slots are always valid
                    
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) && (is_slot_locked || is_confirmed_selection) {
                            println!("[Lobby Champion] Found additional confirmed champion: {}", champion_id);
                            champion_ids.push(champion_id);
                        } else if champion_id > 0 {
                            println!("[Lobby Champion] Found additional unlocked champion: {} (not adding)", champion_id);
                        }
                    }
                }
            }
        }
    }
    champion_ids
}