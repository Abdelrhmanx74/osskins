use tauri::{AppHandle, Emitter, Manager};
use std::path::Path;
use std::fs;
use crate::injection::{Skin, MiscItem, inject_skins as inject_skins_impl, inject_skins_and_misc};
use crate::commands::types::{SkinInjectionRequest, SkinData};
use crate::commands::config::{save_league_path};
use crate::commands::lcu_watcher::start_lcu_watcher;

// Skin injection related commands

#[tauri::command]
pub fn inject_skins(
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
        }
    };

    // Always emit injection ended event, regardless of success/failure
    let _ = app_handle.emit("injection-status", false);
    
    result
}

// Enhanced injection command that supports both skins and misc items
#[tauri::command]
pub async fn inject_skins_with_misc(
    app_handle: AppHandle,
    game_path: String,
    skins: Vec<SkinData>,
    misc_items: Vec<MiscItem>,
    fantome_files_dir: String
) -> Result<String, String> {
    println!("Starting enhanced skin injection process");
    println!("League path: {}", game_path);
    println!("Number of skins to inject: {}", skins.len());
    println!("Number of misc items to inject: {}", misc_items.len());

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

    // Call the enhanced injection function
    let result = match inject_skins_and_misc(
        &app_handle,
        &game_path,
        &internal_skins,
        &misc_items,
        base_path
    ) {
        Ok(_) => {
            println!("Enhanced skin injection completed successfully");
            Ok("Enhanced skin injection completed successfully".to_string())
        },
        Err(e) => {
            println!("Enhanced skin injection failed: {}", e);
            Err(format!("Enhanced skin injection failed: {}", e))
        }
    };

    // Always emit injection ended event, regardless of success/failure
    let _ = app_handle.emit("injection-status", false);
    
    result
}

// The ensure_mod_tools command is no longer needed since we're not using external tools anymore
#[tauri::command]
pub async fn ensure_mod_tools(_app: tauri::AppHandle) -> Result<(), String> {
    // This function now does nothing since we don't need external tools anymore
    Ok(())
}

#[tauri::command]
pub async fn start_auto_inject(app: AppHandle, league_path: String) -> Result<(), String> {
    println!("Starting auto-inject for path: {}", league_path);
    
    // Start the LCU watcher in a separate thread
    start_lcu_watcher(app, league_path)?;
    
    Ok(())
}

// Preload resources function to improve first-time injection speed
#[allow(dead_code)]
pub fn preload_resources(_app_handle: &tauri::AppHandle) -> Result<(), String> {
    // Preloading is disabled - no caching or fallback logic
    println!("Preloading disabled - using direct file access only");
    Ok(())
}
