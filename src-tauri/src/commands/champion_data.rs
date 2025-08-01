use tauri::{AppHandle, Manager};
use std::fs;
use crate::commands::types::{DataUpdateResult};

// Champion data management commands

#[tauri::command]
pub async fn check_data_updates(app: tauri::AppHandle) -> Result<DataUpdateResult, String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champions_dir = app_data_dir.join("champions");
    if !champions_dir.exists() {
        return Ok(DataUpdateResult {
            success: true,
            error: None,
            updated_champions: vec!["all".to_string()],
        });
    }

    // TODO: Implement actual update checking logic
    // For now, we'll just return that no updates are needed
    Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: Vec::new(),
    })
}

#[tauri::command]
pub async fn update_champion_data(
    app: tauri::AppHandle,
    champion_name: String,
    data: String,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champion_dir = app_data_dir.join("champions").join(&champion_name);
    fs::create_dir_all(&champion_dir)
        .map_err(|e| format!("Failed to create champion directory: {}", e))?;

    let champion_file = champion_dir.join(format!("{}.json", champion_name));
    fs::write(champion_file, data)
        .map_err(|e| format!("Failed to write champion data: {}", e))?;

    Ok(())
}

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
pub async fn check_champions_data(app: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champions_dir = app_data_dir.join("champions");
    if !champions_dir.exists() {
        return Ok(false);
    }

    // Check if there are any champion directories with JSON files
    let has_data = fs::read_dir(champions_dir)
        .map_err(|e| format!("Failed to read champions directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .any(|champion_dir| {
            fs::read_dir(champion_dir.path())
                .ok()
                .map_or(false, |mut entries| {
                    entries.any(|entry| {
                        entry.map_or(false, |e| {
                            e.path().extension().and_then(|s| s.to_str()) == Some("json")
                        })
                    })
                })
        });

    Ok(has_data)
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


// Helper functions
pub async fn get_champion_name(app: &tauri::AppHandle, champion_id: u32) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        
    let champions_dir = app_data_dir.join("champions");
    
    // Look through champion directories to find the one with matching ID
    if champions_dir.exists() {
        for entry in std::fs::read_dir(champions_dir).map_err(|e| e.to_string())? {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let json_file = path.join(format!("{}.json", entry.file_name().to_string_lossy()));
                    
                    if json_file.exists() {
                        if let Ok(content) = std::fs::read_to_string(&json_file) {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(id) = data.get("id").and_then(|v| v.as_u64()) {
                                    if id as u32 == champion_id {
                                        if let Some(_name) = data.get("name").and_then(|v| v.as_str()) {
                                            // Use champion directory name instead of display name for consistency
                                            return Ok(entry.file_name().to_string_lossy().to_string());
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
    
    // Fallback
    Ok(format!("champion_{}", champion_id))
}

// Helper function to get champion ID from name
#[allow(dead_code)]
pub fn get_champion_id_by_name(app: &AppHandle, champion_name: &str) -> Option<u32> {
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
