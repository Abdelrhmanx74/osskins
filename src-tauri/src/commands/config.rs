use tauri::{AppHandle, Manager};
use std::fs;
use std::path::{Path, PathBuf};
use serde_json;
use crate::commands::types::{SavedConfig, SkinData, ThemePreferences};

// Configuration management commands

// Add functions to save and load game path
#[tauri::command]
pub async fn save_league_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    println!("Saving League path: {}", path);
    
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    // Create config directory if it doesn't exist
    let config_dir = app_data_dir.join("config");
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    
    // Save path to config file
    let config_file = config_dir.join("league_path.txt");
    fs::write(&config_file, &path)
        .map_err(|e| format!("Failed to write league path: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn load_league_path(app: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let config_file = app_data_dir.join("config").join("league_path.txt");
    
    if (!config_file.exists()) {
        return Ok(String::new()); // Return empty string if no saved path
    }
    
    let path = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read league path: {}", e))?;
    
    // Verify the path still exists and contains either:
    // - Game/League of Legends.exe (game executable)
    // - LeagueClient.exe (client executable)
    let game_path = Path::new(&path);
    let game_exe_path = game_path.join("Game").join("League of Legends.exe");
    let client_exe_path = game_path.join("LeagueClient.exe");
    
    if !game_path.exists() || (!game_exe_path.exists() && !client_exe_path.exists()) {
        return Ok(String::new()); // Return empty if path is no longer valid
    }
    
    println!("Loaded League path: {}", path);
    Ok(path)
}

// Existing save_selected_skins now also takes league_path and writes combined config.json
#[tauri::command]
pub async fn save_selected_skins(
    app: tauri::AppHandle, 
    leaguePath: String, 
    skins: Vec<SkinData>, 
    favorites: Vec<u32>,
    theme: Option<ThemePreferences>
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let file = config_dir.join("config.json");
    // build combined JSON
    let config_json = serde_json::json!({
        "league_path": leaguePath,
        "skins": skins,
        "favorites": favorites,
        "theme": theme
    });
    let data = serde_json::to_string_pretty(&config_json)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write config.json: {}", e))?;
    Ok(())
}


// New command to load config.json (league path + skins)
#[tauri::command]
pub async fn load_config(app: tauri::AppHandle) -> Result<SavedConfig, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    let file = config_dir.join("config.json");
    if !file.exists() {
        return Ok(SavedConfig { league_path: None, skins: Vec::new(), favorites: Vec::new(), theme: None });
    }
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;
    let cfg: SavedConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config.json: {}", e))?;
    Ok(cfg)
}

// Helper function to get league path from config
pub fn get_league_path_from_config(app_handle: &AppHandle) -> Option<String> {
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let config_file = app_data_dir.join("config").join("config.json");
        if config_file.exists() {
            if let Ok(content) = fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str::<SavedConfig>(&content) {
                    return config.league_path;
                }
            }
        }
        
        // Try the legacy league_path.txt file as fallback
        let legacy_path_file = app_data_dir.join("config").join("league_path.txt");
        if legacy_path_file.exists() {
            if let Ok(path) = fs::read_to_string(&legacy_path_file) {
                if !path.trim().is_empty() {
                    return Some(path.trim().to_string());
                }
            }
        }
    }
    None
}