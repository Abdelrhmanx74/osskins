use crate::commands::types::*;
use tauri::{Manager};
use std::fs;
use serde_json;

#[tauri::command]
pub async fn save_fantome_file(
    app: tauri::AppHandle,
    champion_name: String,
    skin_name: String,
    is_chroma: bool,
    chroma_id: Option<u32>,
    content: Vec<u8>,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    // Create champions directory if it doesn't exist
    let champions_dir = app_data_dir.join("champions");
    fs::create_dir_all(&champions_dir)
        .map_err(|e| format!("Failed to create champions directory: {}", e))?;
    
    // Create champion directory if it doesn't exist
    let champion_dir = champions_dir.join(&champion_name);
    fs::create_dir_all(&champion_dir)
        .map_err(|e| format!("Failed to create champion directory: {}", e))?;
    
    let fantome_file = if is_chroma {
        champion_dir.join(format!("{}_chroma_{}.fantome", skin_name, chroma_id.unwrap_or(0)))
    } else {
        champion_dir.join(format!("{}.fantome", skin_name))
    };
    
    // Ensure parent directory exists
    if let Some(parent) = fantome_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }
    
    fs::write(&fantome_file, content)
        .map_err(|e| format!("Failed to write fantome file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_champion_data(
    app: tauri::AppHandle,
    champion_id: u32,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champions_dir = app_data_dir.join("champions");
    if !champions_dir.exists() {
        return Ok("[]".to_string()); // Return empty array if no champions directory exists
    }

    // If champion_id is 0, return all champions
    if champion_id == 0 {
        let mut all_champions = Vec::new();
        for entry in fs::read_dir(champions_dir)
            .map_err(|e| format!("Failed to read champions directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                // Look for JSON files in the champion directory
                for champion_file in fs::read_dir(path)
                    .map_err(|e| format!("Failed to read champion directory: {}", e))? {
                    let champion_file = champion_file.map_err(|e| format!("Failed to read champion file: {}", e))?;
                    let file_path = champion_file.path();
                    if file_path.extension().and_then(|s| s.to_str()) == Some("json") {
                        let data = fs::read_to_string(&file_path)
                            .map_err(|e| format!("Failed to read champion file: {}", e))?;
                        all_champions.push(data);
                    }
                }
            }
        }
        return Ok(format!("[{}]", all_champions.join(",")));
    }

    // Otherwise, return data for specific champion
    // We need to search through all champion directories to find the one with matching ID
    for entry in fs::read_dir(champions_dir)
        .map_err(|e| format!("Failed to read champions directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let champion_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("Invalid champion directory name"))?;
            let champion_file = path.join(format!("{}.json", champion_name));
            if champion_file.exists() {
                return fs::read_to_string(champion_file)
                    .map_err(|e| format!("Failed to read champion data: {}", e));
            }
        }
    }

    Err(format!("Champion data not found for ID: {}", champion_id))
}

#[tauri::command]
pub async fn delete_champions_cache(app: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champions_dir = app_data_dir.join("champions");
    
    // If the directory exists, remove it and all its contents
    if champions_dir.exists() {
        fs::remove_dir_all(&champions_dir)
            .map_err(|e| format!("Failed to delete champions cache: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn save_selected_skins(
    app: tauri::AppHandle, 
    league_path: String, 
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
        "league_path": league_path,
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