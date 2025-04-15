use tauri::Manager;
use tauri::AppHandle;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::injection::{Skin, inject_skins as inject_skins_impl};

#[derive(Debug, Serialize, Deserialize)]
pub struct DataUpdateProgress {
    pub current_champion: String,
    pub total_champions: usize,
    pub processed_champions: usize,
    pub status: String,
    pub progress: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataUpdateResult {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub updated_champions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinInjectionRequest {
    pub league_path: String,
    pub skins: Vec<Skin>,
}

// Add a new structure to match the JSON data for skins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinData {
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome: Option<String>, // Add fantome path from the JSON
}

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
pub async fn select_league_directory() -> Result<String, String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; $dialog = New-Object System.Windows.Forms.OpenFileDialog; $dialog.Filter = 'League of Legends|League of Legends.exe'; $dialog.Title = 'Select League of Legends.exe'; $dialog.ShowDialog() | Out-Null; $dialog.FileName",
        ])
        .output()
        .map_err(|e| format!("Failed to execute powershell command: {}", e))?;

    if !output.status.success() {
        return Err("Failed to select file".to_string());
    }

    let path = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse file path: {}", e))?
        .trim()
        .to_string();

    if path.is_empty() {
        return Err("No file selected".to_string());
    }

    // Get the directory path from the file path
    let game_path = PathBuf::from(&path);
    if !game_path.exists() {
        return Err("Selected file does not exist".to_string());
    }

    // Return the directory path
    Ok(game_path.parent()
        .ok_or_else(|| "Failed to get parent directory".to_string())?
        .to_str()
        .ok_or_else(|| "Failed to convert path to string".to_string())?
        .to_string())
}

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
    
    // Call the native Rust implementation of skin injection using our new SkinInjector
    inject_skins_impl(
        &app,
        &request.league_path,
        &request.skins,
        &fantome_files_dir,
    )?;
    
    println!("Skin injection completed");
    Ok(())
}

// The ensure_mod_tools command is no longer needed since we're not using external tools
#[tauri::command]
pub async fn ensure_mod_tools(_app: tauri::AppHandle) -> Result<(), String> {
    // This function now does nothing since we don't need external tools anymore
    Ok(())
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
    
    // Verify the path still exists and contains League of Legends.exe
    let game_path = Path::new(&path);
    if !game_path.exists() || !game_path.join("League of Legends.exe").exists() {
        return Ok(String::new()); // Return empty if path is no longer valid
    }
    
    println!("Loaded League path: {}", path);
    Ok(path)
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

    // Validate game path exists
    if !Path::new(&game_path).exists() {
        return Err(format!("League of Legends directory not found: {}", game_path));
    }
    
    // Validate fantome directory exists
    let base_path = Path::new(&fantome_files_dir);
    if !base_path.exists() {
        // Create the directory if it doesn't exist
        println!("Creating fantome files directory: {}", base_path.display());
        fs::create_dir_all(base_path)
            .map_err(|e| format!("Failed to create fantome directory: {}", e))?;
    }

    // Save the league path for future use
    save_league_path(app_handle.clone(), game_path.clone()).await?;

    // Convert SkinData to the internal Skin type, preserving the fantome path
    let internal_skins: Vec<Skin> = skins.iter().map(|s| {
        let skin = Skin {
            champion_id: s.champion_id,
            skin_id: s.skin_id,
            chroma_id: s.chroma_id,
            fantome_path: s.fantome.clone(),
        };
        println!("Skin to inject: champion_id={}, skin_id={}, chroma_id={:?}, fantome_path={:?}",
            skin.champion_id, skin.skin_id, skin.chroma_id, skin.fantome_path);
        skin
    }).collect();
    
    // Call the injection function directly instead of using the async version
    match inject_skins_impl(
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
    }
}