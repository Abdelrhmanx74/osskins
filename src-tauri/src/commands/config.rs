use tauri::{AppHandle, Manager};
use std::fs;
use std::path::{Path};
use serde_json;
use crate::commands::types::{SavedConfig, SkinData, CustomSkinData, ThemePreferences, PartyModeConfig};

// Configuration management commands

// Debug command to check what's in config
#[tauri::command]
pub async fn debug_config(app: tauri::AppHandle) -> Result<String, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    let file = config_dir.join("config.json");
    
    if !file.exists() {
        return Ok("Config file does not exist".to_string());
    }
    
    let content = fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    Ok(content)
}

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
    
    if !config_file.exists() {
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
    league_path: String, 
    skins: Vec<SkinData>, 
    favorites: Vec<u32>,
    theme: Option<ThemePreferences>,
    selected_misc_items: Option<std::collections::HashMap<String, Vec<String>>>
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let file = config_dir.join("config.json");

    // Read existing config if it exists
    let mut config: serde_json::Value = if file.exists() {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?
    } else {
        serde_json::json!({})
    };

    // Update only the relevant fields
    config["league_path"] = serde_json::to_value(league_path).unwrap();
    config["skins"] = serde_json::to_value(skins).unwrap();
    config["favorites"] = serde_json::to_value(favorites).unwrap();
    config["theme"] = serde_json::to_value(theme).unwrap();
    config["selected_misc_items"] = serde_json::to_value(selected_misc_items.unwrap_or_default()).unwrap();
    // Preserve auto update and last commit fields if present
    if config.get("auto_update_data").is_none() {
        config["auto_update_data"] = serde_json::json!(true);
    }

    // Write back the merged config
    let data = serde_json::to_string_pretty(&config)
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
        return Ok(SavedConfig { 
            league_path: None, 
            skins: Vec::new(),
            custom_skins: Vec::new(),
            favorites: Vec::new(), 
            theme: None,
            party_mode: PartyModeConfig::default(),
            selected_misc_items: std::collections::HashMap::new(),
            auto_update_data: true,
            last_data_commit: None,
        });
    }
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;
    let mut cfg: SavedConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config.json: {}", e))?;

    // Backfill defaults
    if cfg.last_data_commit.is_none() {
        cfg.last_data_commit = None;
    }
    Ok(cfg)
}

// Unified skin selection command - handles both official and custom skins with mutual exclusion
#[tauri::command]
pub async fn select_skin_for_champion(
    app: tauri::AppHandle,
    champion_id: u32,
    skin_data: Option<SkinData>,
    custom_skin_data: Option<CustomSkinData>
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let file = config_dir.join("config.json");

    // Load existing config
    let mut config = if file.exists() {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;
        serde_json::from_str::<SavedConfig>(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?
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

    // Remove any existing selections for this champion (both official and custom)
    config.skins.retain(|s| s.champion_id != champion_id);
    config.custom_skins.retain(|s| s.champion_id != champion_id);

    // Add the new selection
    if let Some(skin) = skin_data {
        if skin.champion_id == champion_id {
            config.skins.push(skin);
        }
    }
    
    if let Some(custom_skin) = custom_skin_data {
        if custom_skin.champion_id == champion_id {
            config.custom_skins.push(custom_skin);
        }
    }

    // Save the updated config
    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write config.json: {}", e))?;

    Ok(())
}

// Command to remove skin selection for a champion
#[tauri::command]
pub async fn remove_skin_for_champion(
    app: tauri::AppHandle,
    champion_id: u32
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    let file = config_dir.join("config.json");

    if !file.exists() {
        return Ok(()); // Nothing to remove
    }

    // Load existing config
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;
    let mut config: SavedConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config.json: {}", e))?;

    // Remove selections for this champion
    config.skins.retain(|s| s.champion_id != champion_id);
    config.custom_skins.retain(|s| s.champion_id != champion_id);

    // Save the updated config
    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write config.json: {}", e))?;

    Ok(())
}

// Command to add/update a custom skin
#[tauri::command]
pub async fn save_custom_skin(
    app: tauri::AppHandle,
    custom_skin: CustomSkinData
) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let file = config_dir.join("config.json");

    // Load existing config
    let mut config = if file.exists() {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;
        serde_json::from_str::<SavedConfig>(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?
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

    // Remove existing custom skin with same ID if it exists
    config.custom_skins.retain(|s| s.id != custom_skin.id);
    
    // Add the new/updated custom skin
    config.custom_skins.push(custom_skin);

    // Save the updated config
    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write config.json: {}", e))?;

    Ok(())
}

// Command to get all custom skins
#[tauri::command]
pub async fn get_all_custom_skins(app: tauri::AppHandle) -> Result<Vec<CustomSkinData>, String> {
    let config = load_config(app).await?;
    Ok(config.custom_skins)
}

// Helper function to get league path from config
#[allow(dead_code)]
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

// Command to set auto_update_data in config.json
#[tauri::command]
pub async fn set_auto_update_data(app: tauri::AppHandle, value: bool) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let file = config_dir.join("config.json");

    let mut cfg: serde_json::Value = if file.exists() {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read config.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?
    } else {
        serde_json::json!({})
    };

    cfg["auto_update_data"] = serde_json::json!(value);

    let data = serde_json::to_string_pretty(&cfg)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write config.json: {}", e))?;

    Ok(())
}