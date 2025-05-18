use crate::injection::{Skin, inject_skins as inject_skins_impl, cleanup_injection, get_global_index, SkinInjector};
use super::types::*;
use tauri::{AppHandle, Manager, Emitter};
use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::time::Duration;
use serde_json;
use crate::save_league_path;
use crate::start_lcu_watcher;

// TODO: Move these functions from commands.rs:

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
pub async fn start_auto_inject(app: AppHandle, leaguePath: String) -> Result<(), String> {
    println!("Starting auto-inject for path: {}", leaguePath);
    
    // Start the LCU watcher in a separate thread
    start_lcu_watcher(app, leaguePath)?;
    
    Ok(())
}

// Preload resources function to improve first-time injection speed
pub fn preload_resources(app_handle: &tauri::AppHandle) -> Result<(), String> {
    // Inform user that resources are loading
    println!("Preloading resources for faster first injection...");
    
    // Get app data directory
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    // Create essential directories if they don't exist
    let overlay_cache_dir = app_data_dir.join("overlay_cache");
    if (!overlay_cache_dir.exists()) {
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
            // Try to create a temporary injector that will initialize cache
            if let Ok(mut injector) = SkinInjector::new(&app_handle_clone, &league_path) {
                let _ = injector.initialize_cache();
                println!("Successfully preloaded injection resources");
            }
        } else {
            println!("League path not found, skipping preload");
        }
    });
    
    Ok(())
}

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