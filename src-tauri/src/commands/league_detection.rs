use crate::commands::types::*;
use tauri::{AppHandle, Manager};
use std::path::Path;
use std::process::Command;
use std::fs;
use serde_json;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub async fn select_league_directory() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut command = Command::new("powershell");
    
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW); // CREATE_NO_WINDOW flag

    command
        .args([
            "-NoProfile",
            "-Command",
            r#"Add-Type -AssemblyName System.Windows.Forms; 
            $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; 
            $dialog.Description = 'Select League of Legends Installation Directory'; 
            if($dialog.ShowDialog() -eq 'OK') { $dialog.SelectedPath }"#,
        ]);
    
    let output = command
        .output()
        .map_err(|e| format!("Failed to execute powershell command: {}", e))?;

    if !output.status.success() {
        return Err("Directory selection cancelled".to_string());
    }

    let path = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse selected path: {}", e))?
        .trim()
        .to_string();

    if path.is_empty() {
        return Err("No directory selected".to_string());
    }

    // Validate that this appears to be a League of Legends directory
    // Check for either the Game\League of Legends.exe or LeagueClient.exe
    let selected_dir = Path::new(&path);
    let game_exe_path = selected_dir.join("Game").join("League of Legends.exe");
    let client_exe_path = selected_dir.join("LeagueClient.exe");
    
    if !client_exe_path.exists() && !game_exe_path.exists() {
        return Err("Selected directory does not appear to be a valid League of Legends installation".to_string());
    }

    // Always return the root League directory path
    Ok(path)
}

#[tauri::command]
pub async fn auto_detect_league() -> Result<String, String> {
    // Common League of Legends installation paths on Windows
    let common_paths = [
        r"C:\Riot Games\League of Legends",
        r"C:\Program Files\Riot Games\ League of Legends",
        r"C:\Program Files (x86)\Riot Games\League of Legends",
    ];

    for path in common_paths.iter() {
        let client_path = Path::new(path).join("LeagueClient.exe");
        if client_path.exists() {
            return Ok(path.to_string());
        }
    }

    // Try to find through registry as fallback
    let mut command = Command::new("powershell");
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW flag

    command
        .args([
            "-NoProfile",
            "-Command",
            r#"Get-ItemProperty -Path 'HKLM:\SOFTWARE\WOW6432Node\Riot Games, Inc\League of Legends' -Name 'Location' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Location"#,
        ]);

    if let Ok(output) = command.output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let path = path.trim();
                if !path.is_empty() {
                    let path = Path::new(path);
                    if path.join("LeagueClient.exe").exists() {
                        return Ok(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    Err("League of Legends installation not found".to_string())
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
