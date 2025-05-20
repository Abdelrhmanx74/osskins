use crate::injection::{Skin, inject_skins as inject_skins_impl, get_global_index, SkinInjector};
use crate::commands::types::*;
use tauri::{AppHandle, Manager, Emitter};
use std::path::{Path, PathBuf};
use std::fs;
use serde_json;
use super::lcu_communication::start_lcu_watcher;
use crate::commands::league_detection::save_league_path;

#[tauri::command]
pub async fn inject_skins(
    app: tauri::AppHandle,
    request: SkinInjectionRequest,
) -> Result<(), String> {
    println!("Starting skin injection process");
    println!("League path: {}", request.league_path);
    println!("Number of skins to inject: {}", request.skins.len());
    
    // Get the app data directory (where champion data is stored)
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    // Get the path to the champions directory where fantome files are stored
    let fantome_files_dir = app_data_dir.join("champions");
    println!("Fantome files directory: {}", fantome_files_dir.display());
    
    // Emit injection started event to update UI
    let _ = app.emit("injection-status", "injecting");
    
    // Call the native Rust implementation of skin injection using our new SkinInjector
    let result = inject_skins_impl(
        &app,
        &request.league_path,
        &request.skins,
        &fantome_files_dir,
    );
    
    // Handle result with proper error propagation to frontend
    match result {
        Ok(_) => {
            println!("Skin injection completed successfully");
            let _ = app.emit("injection-status", "success");
            Ok(())
        },
        Err(err) => {
            println!("Skin injection failed: {}", err);
            let _ = app.emit("injection-status", "error");
            let _ = app.emit("skin-injection-error", format!("Injection failed: {}", err));
            Err(format!("Injection failed: {}", err))
        }
    }
}


#[tauri::command]
pub async fn inject_game_skins(
    app_handle: AppHandle,
    game_path: String,
    skins: Vec<SkinData>, 
    fantome_files_dir: String
) -> Result<String, String> {
    println!("Starting skin injection process");
    println!("League path: {}", game_path);
    println!("Number of skins to inject: {}", skins.len());
    println!("Fantome files directory: {}", fantome_files_dir);

    // Emit injection started event
    let _ = app_handle.emit("injection-status", true);

    // Validate game path exists
    if !Path::new(&game_path).exists() {
        let _ = app_handle.emit("injection-status", false);
        return Err(format!("League of Legends directory not found: {}", game_path));
    }
    
    // Validate fantome directory exists
    let base_path = Path::new(&fantome_files_dir);
    if !base_path.exists() {
        // Create the directory if it doesn't exist
        println!("Creating fantome files directory: {}", base_path.display());
        fs::create_dir_all(base_path)
            .map_err(|e| {
                let _ = app_handle.emit("injection-status", false);
                format!("Failed to create fantome directory: {}", e)
            })?;
    }

    // Save the league path for future use
    save_league_path(app_handle.clone(), game_path.clone()).await?;

    // Convert SkinData to the internal Skin type
    let internal_skins: Vec<Skin> = skins.iter().map(|s| {
        Skin {
            champion_id: s.champion_id,
            skin_id: s.skin_id,
            chroma_id: s.chroma_id,
            fantome_path: s.fantome.clone(),
        }
    }).collect();

    // Call the injection function
    let result = match inject_skins_impl(
        &app_handle,
        &game_path,
        &internal_skins,
        base_path
    ) {
        Ok(_) => {
            println!("Skin injection completed successfully");
            Ok("Skin injection completed successfully".to_string())
        },
        Err(e) => {
            println!("Skin injection failed: {}", e);
            Err(format!("Skin injection failed: {}", e))
        },
    };

    // Always emit injection ended event, regardless of success/failure
    let _ = app_handle.emit("injection-status", false);
    
    result
}

// Helper function to inject skins for multiple champions (used in Swift Play)
pub fn inject_skins_for_champions(app: &AppHandle, league_path: &str, champion_ids: &[i64]) {
    println!("[Swift Play Injection] Attempting to inject skins for champions: {:?}", champion_ids);
    
    // Filter out invalid champion IDs (0 or negative)
    let valid_champion_ids: Vec<i64> = champion_ids.iter()
        .filter(|&&id| id > 0)
        .cloned()
        .collect();
        
    if valid_champion_ids.is_empty() {
        println!("[Swift Play Injection] No valid champion IDs to inject");
        return;
    }
    
    println!("[Swift Play Injection] Valid champion IDs: {:?}", valid_champion_ids);
    
    let config_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
    let cfg_file = config_dir.join("config.json");
    
    println!("[Swift Play Injection] Looking for config file: {}", cfg_file.display());
    
    // Check if we have config with skin selections
    if let Ok(data) = std::fs::read_to_string(&cfg_file) {
        println!("[Swift Play Injection] Found config file, parsing");
        if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
            println!("[Swift Play Injection] Config has {} skin selections", config.skins.len());
            
            // Get all skins for the selected champions
            let mut skins_to_inject = Vec::new();
            let mut champions_without_skins = Vec::new();
            
            for champ_id in &valid_champion_ids {
                let champ_id_u32 = *champ_id as u32;
                println!("[Swift Play Injection] Looking for skin for champion ID: {}", champ_id_u32);
                
                if let Some(skin) = config.skins.iter().find(|s| s.champion_id == champ_id_u32) {
                    println!("[Swift Play Injection] Found skin selection for champion {}: skin_id={}, chroma_id={:?}", 
                             champ_id_u32, skin.skin_id, skin.chroma_id);
                    
                    skins_to_inject.push(Skin {
                        champion_id: skin.champion_id,
                        skin_id: skin.skin_id,
                        chroma_id: skin.chroma_id,
                        fantome_path: skin.fantome.clone(),
                    });
                } else {
                    println!("[Swift Play Injection] No skin selection found for champion {}", champ_id_u32);
                    champions_without_skins.push(*champ_id);
                }
            }
            
            // Log information about champions without skins
            if !champions_without_skins.is_empty() {
                println!("Champions without selected skins: {:?}", champions_without_skins);
            }
            
            // If we found skins to inject, do it
            if !skins_to_inject.is_empty() {
                println!("Injecting skins for {} champions", skins_to_inject.len());
                
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
                        println!("Successfully injected skins");
                    },
                    Err(e) => {
                        let _ = app.emit("skin-injection-error", format!(
                            "Failed to inject skins: {}", e
                        ));
                        let _ = app.emit("injection-status", "error");
                        println!("Error injecting skins: {}", e);
                    }
                }
            } else if !champion_ids.is_empty() {
                // Let user know we found champions but no skins were configured
                println!("Found {} champions but no skins are configured for them", champion_ids.len());
            }
        } else {
            println!("Failed to parse config file for skin selections");
        }
    } else {
        println!("No config file found with skin selections");
    }
}

// Get the selected champions from any game mode (Normal, ARAM, Swift Play, Brawl, etc.)
pub fn get_selected_champions_universal(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    println!("!!! FUNCTION CALLED: get_selected_champions_universal !!!");
    println!("Universal champion detection: checking for champions in session data");
    println!("JSON structure keys at root level: {:?}", json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
    
    // Get game mode info for logging purposes
    let game_mode = if let Some(game_data) = json.get("gameData") {
        if let Some(queue) = game_data.get("queue") {
            if let Some(mode) = queue.get("gameMode").and_then(|m| m.as_str()) {
                Some(mode)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    
    println!("Detected game mode from JSON: {:?}", game_mode);
    
    // IMPORTANT: Check for Swift Play matchmaking phase - special case that needs auto-confirmation
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
    
    // Determine if this is Swift Play mode
    let is_swift_play = queue_id == 480 || queue_id == 1700 || 
                      game_mode.map_or(false, |mode| mode.to_uppercase().contains("SWIFT") || 
                                                       mode.to_uppercase().contains("ARENA"));
                      
    // Check if we're in matchmaking phase                  
    let is_matchmaking = json.get("state")
        .and_then(|s| s.as_str())
        .map(|s| s == "MATCHMAKING" || s == "GAMESTARTING" || s == "INPROGRESS")
        .unwrap_or(false);
        
    let swift_play_matchmaking = is_swift_play && is_matchmaking;
    
    // Special Swift Play matchmaking auto-confirmation override
    if swift_play_matchmaking {
        println!("[Swift Play AUTO-CONFIRM] Detected Swift Play in matchmaking phase");
        
        // In Swift Play matchmaking, we should auto-confirm all champions in playerSlots
        if let Some(local_member) = json.get("localMember") {
            println!("[Swift Play AUTO-CONFIRM] Found localMember in matchmaking");
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                println!("[Swift Play AUTO-CONFIRM] Found {} player slots", player_slots.len());
                
                // Auto-confirm all champions in player slots
                for (i, slot) in player_slots.iter().enumerate() {
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 {
                            println!("[Swift Play AUTO-CONFIRM] Auto-confirming champion {} in slot {}", champion_id, i);
                            champion_ids.push(champion_id);
                        }
                    }
                }
                
                // If we found champions, return them immediately
                if !champion_ids.is_empty() {
                    println!("[Swift Play AUTO-CONFIRM] Returning {} auto-confirmed champions: {:?}", 
                            champion_ids.len(), champion_ids);
                    return champion_ids;
                }
            } else {
                // Also try checking gameData.playerChampionSelections for Swift Play matchmaking
                if let Some(game_data) = json.get("gameData") {
                    if let Some(selections) = game_data.get("playerChampionSelections").and_then(|s| s.as_array()) {
                        println!("[Swift Play AUTO-CONFIRM] Found playerChampionSelections: {} entries", selections.len());
                        
                        for (i, selection) in selections.iter().enumerate() {
                            if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                                if champion_id > 0 {
                                    println!("[Swift Play AUTO-CONFIRM] Auto-confirming champion {} from selection {}", champion_id, i);
                                    champion_ids.push(champion_id);
                                }
                            }
                        }
                        
                        // If we found champions this way, return them immediately
                        if !champion_ids.is_empty() {
                            println!("[Swift Play AUTO-CONFIRM] Returning {} auto-confirmed champions from selections: {:?}", 
                                    champion_ids.len(), champion_ids);
                            return champion_ids;
                        }
                    }
                }
                
                println!("[Swift Play AUTO-CONFIRM] No playerSlots found in localMember");
            }
        } else {
            println!("[Swift Play AUTO-CONFIRM] No localMember found");
        }
    }
    
    // Check if we're in any matchmaking phase - this applies to all game modes  
    let in_matchmaking = json.get("state")
        .and_then(|s| s.as_str())
        .map(|s| s == "MATCHMAKING" || s == "GAMESTARTING" || s == "INPROGRESS")
        .unwrap_or(false);
    
    // If we're in matchmaking but not Swift Play, still auto-confirm champions
    if in_matchmaking && !swift_play_matchmaking {
        println!("[Matchmaking AUTO-CONFIRM] Detected game in matchmaking phase");
        
        // In matchmaking, we should auto-confirm all champions in playerSlots for any game mode
        if let Some(local_member) = json.get("localMember") {
            println!("[Matchmaking AUTO-CONFIRM] Found localMember in matchmaking");
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                println!("[Matchmaking AUTO-CONFIRM] Found {} player slots", player_slots.len());
                
                // Auto-confirm all champions in player slots
                for (i, slot) in player_slots.iter().enumerate() {
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 {
                            println!("[Matchmaking AUTO-CONFIRM] Auto-confirming champion {} in slot {}", champion_id, i);
                            champion_ids.push(champion_id);
                        }
                    }
                }
                
                // If we found champions, return them immediately
                if !champion_ids.is_empty() {
                    println!("[Matchmaking AUTO-CONFIRM] Returning {} auto-confirmed champions: {:?}", 
                            champion_ids.len(), champion_ids);
                    return champion_ids;
                }
            }
            
            // Also check gameData for any game mode
            if let Some(game_data) = json.get("gameData") {
                if let Some(selections) = game_data.get("playerChampionSelections").and_then(|s| s.as_array()) {
                    println!("[Matchmaking AUTO-CONFIRM] Found playerChampionSelections: {} entries", selections.len());
                    
                    for (i, selection) in selections.iter().enumerate() {
                        if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                            if champion_id > 0 {
                                println!("[Matchmaking AUTO-CONFIRM] Auto-confirming champion {} from selection {}", champion_id, i);
                                champion_ids.push(champion_id);
                            }
                        }
                    }
                    
                    // If we found champions this way, return them immediately
                    if !champion_ids.is_empty() {
                        println!("[Matchmaking AUTO-CONFIRM] Returning {} auto-confirmed champions from selections: {:?}", 
                                champion_ids.len(), champion_ids);
                        return champion_ids;
                    }
                }
            }
        }
    }
    
    // Determine if this is a multi-champion mode by analyzing properties
    let is_multi_champion_mode = is_multi_champion_mode(json, game_mode);
    println!("[Mode Detection] Multi-champion mode: {}", is_multi_champion_mode);
    
    // If single-champion mode (like BRAWL), only return the local player's champion
    if !is_multi_champion_mode {
        // Try localMember.playerSlots first (most reliable)
        if let Some(local_member) = json.get("localMember") {
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                if let Some(slot) = player_slots.first() {
                    // Check if this slot is locked/selected
                    let selection_status = slot.get("selectionStatus")
                        .and_then(|status| status.as_str())
                        .unwrap_or("UNKNOWN");
                        
                    let is_ready = local_member.get("ready").and_then(|r| r.as_bool()).unwrap_or(false);
                    let is_slot_locked = selection_status == "SELECTED" || 
                                         selection_status == "LOCKED" ||
                                         is_ready;
                                         
                    println!("[Champion Detection] First slot selection status: {}, ready: {}", 
                             selection_status, is_ready);
                
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && is_slot_locked {
                            println!("[Champion Detection] Single-champion mode - using locked champion: {}", champion_id);
                            return vec![champion_id];
                        } else if champion_id > 0 {
                            println!("[Champion Detection] Found champion {} but it's not locked yet", champion_id);
                        }
                    }
                }
            }
            
            // Fallback: direct championId on localMember, but only if player is ready
            let is_ready = local_member.get("ready").and_then(|r| r.as_bool()).unwrap_or(false);
            if is_ready {
                if let Some(champion_id) = local_member.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 {
                        println!("[Champion Detection] Single-champion mode - using confirmed champion: {}", champion_id);
                        return vec![champion_id];
                    }
                }
            }
        }
        
        // Fallback: try myTeam for local player
        if let Some(my_team) = json.get("myTeam").and_then(|t| t.as_array()) {
            if let Some(local_cell_id) = json.get("localPlayerCellId").and_then(|id| id.as_i64()) {
                for player in my_team {
                    if let Some(cell_id) = player.get("cellId").and_then(|id| id.as_i64()) {
                        if cell_id == local_cell_id {
                            if let Some(champion_id) = player.get("championId").and_then(|id| id.as_i64()) {
                                if champion_id > 0 {
                                    println!("[Champion Detection] Single-champion mode - using only: {}", champion_id);
                                    return vec![champion_id];
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // For multi-champion modes or if no champion was found in single-champion mode logic,
    // continue with universal detection logic:
    
    // Method 1: Standard champion selection via localPlayerCellId and actions
    if let Some(champion_id) = get_selected_champion_id(json) {
        println!("Found champion using standard detection: {}", champion_id);
        champion_ids.push(champion_id);
    }
    
    // Method 2: Check gameData -> playerChampionSelections
    if let Some(game_data) = json.get("gameData") {
        println!("Found gameData node with keys: {:?}", game_data.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        
        // Check for queue info to identify game mode
        let game_mode = game_data.get("queue")
            .and_then(|q| q.get("gameMode"))
            .and_then(|m| m.as_str())
            .unwrap_or("UNKNOWN");
            
        let queue_id = game_data.get("queue")
            .and_then(|q| q.get("id"))
            .and_then(|id| id.as_i64())
            .unwrap_or(0);
            
        println!("Detected game mode: {}, Queue ID: {}", game_mode, queue_id);
        
        // Check if this is Brawl mode by queue ID
        let is_brawl_mode = queue_id == 1300; // 1300 is the queue ID for Brawl
        if is_brawl_mode {
            println!("[Mode Detection] Detected Brawl mode by queue ID: {}", queue_id);
        }
        
        // Check for player champion selections
        if let Some(selections) = game_data.get("playerChampionSelections") {
            println!("Found playerChampionSelections node: {:?}", selections);
            if let Some(selections_array) = selections.as_array() {
            println!("playerChampionSelections is an array with {} items", selections_array.len());
            
            // First try to identify the local player's selection
            let local_player_id = json.get("localPlayerSelection")
                .and_then(|lp| lp.get("summonerId"))
                .and_then(|id| id.as_i64());
                
            // For Brawl mode, we need to be extra careful to only detect the local player's champion
            let is_brawl_mode = queue_id == 1300;
            
            // If it's a single champion mode like Brawl or we didn't detect the mode yet, make sure to check local player
            let check_local_player = is_brawl_mode || !is_multi_champion_mode || local_player_id.is_some();
                
            for (i, selection) in selections_array.iter().enumerate() {
                // Check if this is the local player's selection
                let is_local_player = if check_local_player {
                    if let Some(local_id) = local_player_id {
                        let matches = selection.get("summonerId")
                            .and_then(|id| id.as_i64())
                            .map_or(false, |id| id == local_id);
                            
                        println!("[Champion Detection] Selection {} belongs to local player: {}", i, matches);
                        matches
                    } else {
                        // For Brawl mode, be especially careful
                        if is_brawl_mode {
                            println!("[Champion Detection] Brawl mode - cannot identify local player, skipping selection");
                            false
                        } else {
                            // If we can't identify the local player, only include this in multi-champion modes
                            println!("[Champion Detection] Cannot identify local player, using multi-champion mode check: {}", is_multi_champion_mode);
                            is_multi_champion_mode
                        }
                    }
                } else {
                    // In multi-champion modes without local player ID info, include all champions
                    println!("[Champion Detection] Multi-champion mode without local player ID, including all champions");
                    true
                };
                
                println!("Checking selection {}, is local player: {}", i, is_local_player);
                
                if is_local_player {
                    if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                        println!("Selection {} has championId: {}", i, champion_id);
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            println!("Found champion in playerChampionSelections: {}", champion_id);
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        } else {
            println!("playerChampionSelections is not an array: {:?}", selections);
        }
    } else {
        println!("No playerChampionSelections found in gameData");
    }
} else {
    println!("No gameData node found in JSON");
}
    
    // Method 3: Check local member player slots
    println!("Checking localMember playerSlots");
    if let Some(local_member) = json.get("localMember") {
        println!("Found localMember: {:?}", local_member);
        if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
            println!("Found playerSlots array with {} items", player_slots.len());
            
            // Only process the first slot for single-champion modes
            if !is_multi_champion_mode && player_slots.len() > 0 {
                println!("[Champion Detection] Single-champion mode: only processing first player slot");
                if let Some(champion_id) = player_slots[0].get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                        println!("Adding champion ID from first slot: {}", champion_id);
                        champion_ids.push(champion_id);
                    }
                }
            } else {            // Process all slots for multi-champion modes
            for (i, slot) in player_slots.iter().enumerate() {
                println!("Checking slot {}: {:?}", i, slot);
                
                // Check if this slot is properly selected/locked
                let selection_status = slot.get("selectionStatus")
                    .and_then(|status| status.as_str())
                    .unwrap_or("UNKNOWN");
                    
                let is_ready = json.get("localMember")
                    .and_then(|m| m.get("ready"))
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);
                
                // IMPORTANT: Check for Swift Play matchmaking phase - special case that needs auto-confirmation
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
                
                // Determine if this is Swift Play mode
                let is_swift_play = queue_id == 480 || queue_id == 1700 || 
                                  game_mode.map_or(false, |mode| mode.to_uppercase().contains("SWIFT") || 
                                                               mode.to_uppercase().contains("ARENA"));
                
                // Check if we're in matchmaking phase                  
                let is_matchmaking = json.get("state")
                    .and_then(|s| s.as_str())
                    .map(|s| s == "MATCHMAKING" || s == "GAMESTARTING" || s == "INPROGRESS")
                    .unwrap_or(false);
                
                let swift_play_matchmaking = is_swift_play && is_matchmaking;
                
                // Auto-confirm in Swift Play matchmaking
                let is_swift_play_matchmaking = is_swift_play && is_matchmaking;
                    
                let is_slot_locked = selection_status == "SELECTED" || 
                                     selection_status == "LOCKED" ||
                                     is_ready ||
                                     is_swift_play_matchmaking;
                
                println!("Slot {} selection status: {}, ready: {}, locked: {}, swift_play_matchmaking: {}", 
                         i, selection_status, is_ready, is_slot_locked, is_swift_play_matchmaking);
                
                if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                    println!("Slot {} has championId: {}", i, champion_id);
                    if champion_id > 0 && !champion_ids.contains(&champion_id) && is_slot_locked {
                        println!("Adding champion ID: {} (locked)", champion_id);
                        champion_ids.push(champion_id);
                    } else if champion_id > 0 && !is_slot_locked {
                        println!("Skipping champion ID: {} (not locked)", champion_id);
                    }
                }
            }
            }
        } else {
            println!("[Swift Play Method 3] No playerSlots found in localMember");
        }
    } else {
        println!("[Swift Play Method 3] No localMember found in JSON");
    }
    
    // Method 4: Check all members player slots (Multi-player Swift Play lobby)
    if let Some(members) = json.get("members") {
        println!("Found 'members' node in JSON: {:?}", members);
        if let Some(members_array) = members.as_array() {
            println!("Members is an array with {} items", members_array.len());
            
            // For single-champion modes, we need to identify the local player
            let local_puuid = if !is_multi_champion_mode {
                // Try to get the puuid of the local player
                json.get("localPlayerPuuid")
                    .and_then(|id| id.as_str())
                    .or_else(|| {
                        // Fallback to localPlayer->puuid if available
                        json.get("localPlayer")
                            .and_then(|lp| lp.get("puuid"))
                            .and_then(|id| id.as_str())
                    })
            } else {
                None
            };
            
            for (i, member) in members_array.iter().enumerate() {
                // For single-champion modes, check if this is the local player
                let is_local_player = if !is_multi_champion_mode {
                    if let Some(local_id) = local_puuid {
                        member.get("puuid")
                            .and_then(|id| id.as_str())
                            .map_or(false, |id| id == local_id)
                    } else {
                        // If we can't identify local player, only include in multi-champion modes
                        is_multi_champion_mode
                    }
                } else {
                    // In multi-champion modes, include all members
                    true
                };
                
                println!("Checking member {}: keys = {:?}, is local player: {}", 
                         i, member.as_object().map(|o| o.keys().collect::<Vec<_>>()), is_local_player);
                
                if is_local_player {
                    // Check player slots first
                    if let Some(player_slots) = member.get("playerSlots") {
                        println!("Member {} has playerSlots: {:?}", i, player_slots);
                        if let Some(slots_array) = player_slots.as_array() {
                            println!("PlayerSlots is an array with {} items", slots_array.len());
                            for (j, slot) in slots_array.iter().enumerate() {
                                println!("Checking slot {}: {:?}", j, slot);
                                if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                                    println!("Slot {} has championId: {}", j, champion_id);
                                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                                        println!("Found champion in member playerSlots: {}", champion_id);
                                        champion_ids.push(champion_id);
                                    }
                                }
                            }
                        }
                    }
                    
                    // Direct champion ID check (used in some Swift Play modes)
                    if let Some(champion_id) = member.get("championId").and_then(|id| id.as_i64()) {
                        println!("Member {} has direct championId: {}", i, champion_id);
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            println!("Found champion directly in members: {}", champion_id);
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        } else {
            println!("'members' is not an array: {:?}", members);
        }
    } else {
        println!("No 'members' node found in JSON");
    }
    
    // Method 5: Special handling for ARAM - look at myChampions and bench champions
    if let Some(my_selection) = json.get("myTeam").and_then(|t| t.as_array()) {
        // Get local player cell ID first
        if let Some(local_cell_id) = json.get("localPlayerCellId").and_then(|id| id.as_i64()) {
            // Find the player with matching cellId
            for player in my_selection {
                if let Some(cell_id) = player.get("cellId").and_then(|id| id.as_i64()) {
                    if cell_id == local_cell_id {
                        // This is the local player, get their champion
                        if let Some(champion_id) = player.get("championId").and_then(|id| id.as_i64()) {
                            if champion_id > 0 && !champion_ids.contains(&champion_id) {
                                println!("[ARAM Detection] Found champion ID: {}", champion_id);
                                champion_ids.push(champion_id);
                                
                                // For single-champion modes, only return this one champion
                                if !is_multi_champion_mode {
                                    println!("[Single Champion Mode] Found local player champion: {}", champion_id);
                                    return vec![champion_id]; // Return only the local player's champion
                                }
                                // Continue checking other methods for multi-champion modes
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Method 6: Check bench champions in ARAM (after reroll)
    // Only check bench if we haven't found the local player's champion
    if champion_ids.is_empty() {
        if let Some(bench) = json.get("benchChampions").and_then(|b| b.as_array()) {
            for bench_champ in bench {
                if let Some(champion_id) = bench_champ.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                        champion_ids.push(champion_id);
                    }
                }
            }
        }
    }
    
    // Method 7: Check benchEnabled to confirm if this is ARAM
    if let Some(bench_enabled) = json.get("benchEnabled").and_then(|b| b.as_bool()) {
    }
    
    // Method 8: Handle any potential new game modes by checking any champion ID anywhere in the JSON
    // This is a catch-all for any new game modes Riot might add
    if champion_ids.is_empty() {
        if let Some(champion_id) = find_any_champion_id_in_json(json) {
            if champion_id > 0 {
                println!("Found champion using fallback method: {}", champion_id);
                champion_ids.push(champion_id);
            }
        }
    }
    
    if champion_ids.is_empty() {
        // Output a condensed version of the JSON
        if let Ok(json_str) = serde_json::to_string(json) {
            if json_str.len() > 1000 {
                println!("[JSON Summary] JSON is {} characters long, truncating for log...", json_str.len());
                // Output just a structure summary rather than the full content
                println!("[JSON Structure Summary] Root keys: {:?}", json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                
                // Output specific sections we're interested in
                if let Some(game_data) = json.get("gameData") {
                    println!("[JSON Structure Summary] gameData keys: {:?}", 
                             game_data.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                }
                
                if let Some(my_team) = json.get("myTeam") {
                    println!("[JSON Structure Summary] myTeam: {:?}", my_team);
                }
            } else {
                println!("[Full JSON] {}", json_str);
            }
        }
    } else {
        println!("[Champion Detection] Successfully detected champions: {:?}", champion_ids);
    }
    
    champion_ids
}

pub fn get_selected_champion_id(session_json: &serde_json::Value) -> Option<i64> {
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

// Helper function to get champion ID from name
#[allow(dead_code)]
fn get_champion_id_by_name(app: &AppHandle, champion_name: &str) -> Option<u32> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(_) => return None,
    };
    
    let champions_dir = app_data_dir.join("champions");
    if !champions_dir.exists() {
        return None;
    }
    
    // Normalize the champion name for comparison
    let normalized_name = champion_name.to_lowercase().replace(" ", "").replace("'", "");
    
    // Search through champion JSON files
    if let Ok(entries) = fs::read_dir(champions_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                let champ_dir_name = entry.file_name().to_string_lossy().to_lowercase();
                
                // Check if directory name matches
                if champ_dir_name == normalized_name {
                    // Found a potential match, check the JSON file
                    let json_path = entry.path().join(format!("{}.json", champ_dir_name));
                    
                    if let Ok(content) = fs::read_to_string(json_path) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                            // Extract champion ID from JSON
                            return data.get("id").and_then(|v| v.as_u64()).map(|id| id as u32);
                        }
                    }
                }
            }
        }
    }
    
    None
}

#[tauri::command]
pub async fn start_auto_inject(app: AppHandle, league_path: String) -> Result<(), String> {
    println!("Starting auto-inject for path: {}", league_path);
    
    
    // Start the LCU watcher in a separate thread
    match start_lcu_watcher(app.clone(), league_path.clone()) {
        Ok(_) => {
            
            // Let's try to directly access the config file to verify it exists
            if let Ok(app_data_dir) = app.path().app_data_dir() {
                let config_dir = app_data_dir.join("config");
                let cfg_file = config_dir.join("config.json");
                
                if cfg_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cfg_file) {
                    } else {
                    }
                } else {
                }
            } else {
            }
            
            Ok(())
        },
        Err(e) => {
            Err(format!("Failed to start LCU watcher: {}", e))
        }
    }
}

#[allow(dead_code)]
// Preload resources function to improve first-time injection speed
pub fn preload_resources(app_handle: &tauri::AppHandle) -> Result<(), String> {
    // Inform user that resources are loading
    println!("Preloading resources for faster first injection...");
    
    // Get app data directory
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    // Create essential directories if they don't exist
    let overlay_cache_dir = app_data_dir.join("overlay_cache");
    if !overlay_cache_dir.exists() {
        std::fs::create_dir_all(&overlay_cache_dir)
            .map_err(|e| format!("Failed to create overlay cache directory: {}", e))?;
    }
    
    // Initialize the global file index to cache champion data
    if let Ok(index) = get_global_index(app_handle) {
        let _index_guard = index.lock().unwrap();
        // Index is now initialized in background
    }
    
    // Clone the app_handle before moving it into the thread
    let app_handle_clone = app_handle.clone();
    
    // Pre-build empty overlay templates in the background
    std::thread::spawn(move || {
        // This runs in a separate thread to not block UI
        if let Some(league_path) = get_league_path_from_config(&app_handle_clone) {
            // Try to create a temporary injector (cache logic removed)
            if let Ok(_injector) = SkinInjector::new(&app_handle_clone, &league_path) {
                println!("Successfully preloaded injection resources");
            }
        } else {
            println!("League path not found, skipping preload");
        }
    });
    
    Ok(())
}

#[allow(dead_code)]
pub fn get_league_path_from_config(app_handle: &AppHandle) -> Option<String> {
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let config_file = app_data_dir.join("config").join("config.json");
        if config_file.exists() {
            if let Ok(contents) = std::fs::read_to_string(&config_file) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(path) = json.get("leaguePath").and_then(|v| v.as_str()) {
                        if !path.trim().is_empty() {
                            return Some(path.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

// Helper function to recursively search for any champion ID in a JSON structure
// This is a last-resort fallback for any new or unexpected game mode formats
fn find_any_champion_id_in_json(json: &serde_json::Value) -> Option<i64> {
    println!("[Fallback Detection] Searching for any champion ID in JSON");
    let mut found_id = None;
    let max_depth = 5; // Limit recursion depth to avoid stack overflow
    let _max_depth = max_depth;
    
    fn search_recursive(json: &serde_json::Value, depth: i32, path: &str) -> Option<(i64, String)> {
        if depth > 5 {
            return None; // Prevent stack overflow with deep recursion
        }
        
        match json {
            serde_json::Value::Object(obj) => {
                // First check if this object itself has a championId field
                if let Some(champion_id) = obj.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 {
                        let new_path = format!("{}->championId", path);
                        println!("[Fallback Detection] Found championId={} at path {}", champion_id, new_path);
                        return Some((champion_id, new_path));
                    }
                }
                
                // Then recursively check all fields
                for (key, value) in obj {
                    // Skip certain fields to avoid deep recursion on large objects
                    if key == "actions" || key == "background" || key == "assets" || key == "conversations" {
                        continue;
                    }
                    
                    let new_path = format!("{}->{}", path, key);
                    if let Some((id, found_path)) = search_recursive(value, depth + 1, &new_path) {
                        return Some((id, found_path));
                    }
                }
                None
            },
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    if let Some((id, found_path)) = search_recursive(item, depth + 1, &new_path) {
                        return Some((id, found_path));
                    }
                }
                None
            },
            _ => None,
        }
    }
    
    if let Some((id, path)) = search_recursive(json, 0, "root") {
        println!("[Fallback Detection] Found champion ID {} at path '{}'", id, path);
        found_id = Some(id);
    } else {
        println!("[Fallback Detection] No champion IDs found in JSON");
    }
    
    found_id
}

// Helper function to determine if a mode supports multiple champions per player
pub fn is_multi_champion_mode(json: &serde_json::Value, game_mode: Option<&str>) -> bool {
    // First check for specific queue IDs - these override other checks
    if let Some(game_data) = json.get("gameData") {
        if let Some(queue_id) = game_data.get("queue")
            .and_then(|q| q.get("id"))
            .and_then(|id| id.as_i64())
        {
            // Brawl mode (1300, 2300) is definitely single-champion
            if queue_id == 1300 || queue_id == 2300 {
                println!("[Mode Detection] Detected Brawl mode (Queue ID {}) - single-champion mode", queue_id);
                return false;
            }
            
            // Swift Play modes (480, 1700) are multi-champion
            if queue_id == 1700 || queue_id == 480 {
                println!("[Mode Detection] Detected Swift Play mode (Queue ID {}) - multi-champion mode", queue_id);
                return true;
            }
            
            println!("[Mode Detection] Queue ID: {}", queue_id);
        }
    }
    
    // Check in gameConfig section as well - some API responses have queue ID here
    if let Some(game_config) = json.get("gameConfig") {
        if let Some(queue_id) = game_config.get("queueId").and_then(|id| id.as_i64()) {
            // Brawl mode (1300, 2300) is definitely single-champion
            if queue_id == 1300 || queue_id == 2300 {
                println!("[Mode Detection] Detected Brawl mode (Queue ID {}) in gameConfig - single-champion mode", queue_id);
                return false;
            }
            
            // Swift Play modes (480, 1700) are multi-champion
            if queue_id == 1700 || queue_id == 480 {
                println!("[Mode Detection] Detected Swift Play mode (Queue ID {}) in gameConfig - multi-champion mode", queue_id);
                return true;
            }
        }
    }
    
    // Check for game mode string matching
    if let Some(mode) = game_mode {
        if mode.to_uppercase().contains("SWIFT") || mode.to_uppercase().contains("ARENA") {
            println!("[Mode Detection] Detected multi-champion mode by name: {}", mode);
            return true;
        }
        
        if mode.to_uppercase().contains("BRAWL") {
            println!("[Mode Detection] Detected single-champion 'BRAWL' mode by name");
            return false;
        }
    }
    
    // Check for multiple player slots in localMember - a strong indicator of multi-champion
    if let Some(local_member) = json.get("localMember") {
        if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
            println!("[Mode Detection] Found {} player slots", player_slots.len());
            if player_slots.len() > 1 {
                return true;
            }
        }
    }
    
    // Check for showQuickPlaySlotSelection flag which indicates multi-champion selection
    if let Some(game_config) = json.get("gameConfig") {
        if let Some(quick_play) = game_config.get("showQuickPlaySlotSelection").and_then(|v| v.as_bool()) {
            println!("[Mode Detection] showQuickPlaySlotSelection: {}", quick_play);
            return quick_play;
        }
    }
    
    // Default: Single champion mode is safer (won't duplicate injections)
    println!("[Mode Detection] Defaulting to single-champion mode (safest option)");
    false
}