use tauri::Manager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::injection::{Skin, inject_skins as inject_skins_impl};
use serde_json;
use std::{thread, time::Duration};
use base64;
use tauri::{AppHandle, Emitter};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::OnceLock;

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
    if (!champions_dir.exists()) {
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
        r"C:\Program Files\Riot Games\League of Legends",
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

// The ensure_mod_tools command is no longer needed since we're not using external tools anymore
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

// Existing save_selected_skins now also takes league_path and writes combined config.json
#[derive(Debug, Serialize, Deserialize)]
pub struct ThemePreferences {
    pub tone: Option<String>,
    pub isDark: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedConfig {
    pub league_path: Option<String>,
    pub skins: Vec<SkinData>,
    pub favorites: Vec<u32>,
    #[serde(default)]
    pub theme: Option<ThemePreferences>,
}

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

fn emit_terminal_log(app: &AppHandle, message: &str) {
    let _ = app.emit("terminal-log", message);
}

#[tauri::command]
pub fn start_lcu_watcher(app: AppHandle, leaguePath: String) -> Result<(), String> {
    println!("Starting LCU status watcher for path: {}", leaguePath);
    let app_handle = app.clone();
    let league_path_clone = leaguePath.clone();
    
    thread::spawn(move || {
        let mut last_phase = String::new();
        let mut was_in_game = false;
        let mut was_reconnecting = false;
        let _ = app_handle.emit("lcu-status", "None".to_string());
        
        loop {
            // Set a default sleep duration at the start of each loop
            let mut sleep_duration = Duration::from_secs(5);
            
            let log_msg = format!("[LCU Watcher] Monitoring directory: {}", league_path_clone);
            println!("{}", log_msg);
            emit_terminal_log(&app_handle, &log_msg);
            
            // Only check the configured League directory for lockfile
            let search_dirs = [PathBuf::from(&league_path_clone)];
            let mut port = None;
            let mut token = None;
            let mut found_any_lockfile = false;
            let mut lockfile_path = None;
            for dir in &search_dirs {
                let log_msg = format!("[LCU Watcher] Looking for lockfiles in: {}", dir.display());
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg);
                
                // Check each possible lockfile name
                for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
                    let path = dir.join(name);
                    if path.exists() {
                        found_any_lockfile = true;
                        lockfile_path = Some(path.clone());
                        println!("[LCU Watcher] Found lockfile: {}", path.display());
                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Found lockfile: {}", path.display()));
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        let parts: Vec<&str> = content.split(':').collect();
                        if parts.len() >= 5 {
                            port = Some(parts[2].to_string());
                            token = Some(parts[3].to_string());
                            println!("[LCU Watcher] Found valid lockfile at: {}", path.display());
                            emit_terminal_log(&app_handle, &format!("[LCU Watcher] Found valid lockfile at: {}", path.display()));
                            found_any_lockfile = true;
                            break;
                        } else {
                            println!("[LCU Watcher] Invalid lockfile format at: {}", path.display());
                            emit_terminal_log(&app_handle, &format!("[LCU Watcher] Invalid lockfile format at: {}", path.display()));
                        }
                    }
                }
                
                if port.is_some() && token.is_some() {
                    break; // Stop checking directories if we found a valid lockfile
                }
            }
            
            if !found_any_lockfile {
                // Client is closed, but we were in game - maintain that state
                if was_in_game && (last_phase == "InProgress" || was_reconnecting) {
                    // Do not stop the injection as the player might be reconnecting
                    println!("[LCU Watcher] Client closed during active game! Maintaining state for reconnection.");
                    emit_terminal_log(&app_handle, &"[LCU Watcher] Client closed during active game! Maintaining state for reconnection.");
                    thread::sleep(Duration::from_secs(5));
                    continue;
                } else if was_in_game && last_phase == "None" {
                    // Game actually ended, clean up the injection
                    println!("[LCU Watcher] Game ended, cleaning up skin injection.");
                    emit_terminal_log(&app_handle, &"[LCU Watcher] Game ended, cleaning up skin injection.");
                    if let Err(e) = crate::injection::cleanup_injection(&app_handle, &league_path_clone) {
                        println!("[LCU Watcher] Error cleaning up injection: {}", e);
                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Error cleaning up injection: {}", e));
                    }
                    was_in_game = false;
                }
                
                // Only print a single message if no lockfile is found, and sleep
                let log_msg = format!("[LCU Watcher] No valid lockfile found. Is League running? The lockfile should be at: {}", league_path_clone);
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
            
            let port = port.unwrap();
            let token = token.unwrap();
            let lockfile_path = lockfile_path.unwrap();
            println!("[LCU Watcher] Successfully connected to LCU on port: {}", port);
            emit_terminal_log(&app_handle, &format!("[LCU Watcher] Successfully connected to LCU on port: {}", port));
            
            // Now that we have a valid lockfile and connection, enter a new loop
            'lcu_connected: loop {
                // Before each iteration, check if the lockfile still exists
                if !lockfile_path.exists() {
                    println!("[LCU Watcher] Lockfile disappeared, returning to outer loop to recheck.");
                    emit_terminal_log(&app_handle, "[LCU Watcher] Lockfile disappeared, returning to outer loop to recheck.");
                    break 'lcu_connected;
                }

                // build client with error handling
                match reqwest::blocking::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build() 
                {
                    Ok(client) => {
                        // Try different LCU API endpoints if one fails
                        let endpoints = [
                            "/lol-gameflow/v1/session",
                            "/lol-gameflow/v1/gameflow-phase",
                        ];
                        
                        let mut connected = false;
                        let mut phase_value: Option<String> = None;
                        
                        for endpoint in endpoints {
                            let url = format!("https://127.0.0.1:{}{}", port, endpoint);
                            let log_msg = format!("[LCU Watcher] Trying endpoint: {}", url);
                            println!("{}", log_msg);
                            emit_terminal_log(&app_handle, &log_msg);
                            
                            // Define auth here using the token
                            let auth = base64::encode(format!("riot:{}", token));
                            
                            match client.get(&url)
                                .header("Authorization", format!("Basic {}", auth))
                                .send() 
                            {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        connected = true;
                                        
                                        // Handle different response formats based on the endpoint
                                        match resp.json::<serde_json::Value>() {
                                            Ok(json) => {
                                                // Different endpoints return phase in different formats
                                                if endpoint == "/lol-gameflow/v1/gameflow-phase" {
                                                    // The /gameflow-phase endpoint directly returns the phase as a string
                                                    if let Some(phase) = json.as_str() {
                                                        phase_value = Some(phase.to_string());
                                                        let log_msg = format!("[LCU Watcher] Found phase via gameflow-phase: {}", phase);
                                                        println!("{}", log_msg);
                                                        emit_terminal_log(&app_handle, &log_msg);
                                                        break;
                                                    }
                                                } else {
                                                    // The /session endpoint returns phase as a field
                                                    if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
                                                        phase_value = Some(phase.to_string());
                                                        let log_msg = format!("[LCU Watcher] Found phase via session: {}", phase);
                                                        println!("{}", log_msg);
                                                        emit_terminal_log(&app_handle, &log_msg);
                                                        break;
                                                    }
                                                }
                                            },
                                            Err(e) => println!("[LCU Watcher] Failed to parse response from {}: {}", endpoint, e),
                                        }
                                    } else {
                                        println!("[LCU Watcher] Endpoint {} returned status: {}", endpoint, resp.status());
                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Endpoint {} returned status: {}", endpoint, resp.status()));
                                    }
                                },
                                Err(e) => println!("[LCU Watcher] Failed to connect to endpoint {}: {}", endpoint, e),
                            }
                        }
                        
                        if (!connected) {
                            println!("[LCU Watcher] Could not connect to any LCU API endpoint. Retrying in 5 seconds...");
                            emit_terminal_log(&app_handle, &"[LCU Watcher] Could not connect to any LCU API endpoint. Retrying in 5 seconds...");
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                        
                        let phase = phase_value.unwrap_or_else(|| "None".to_string());
                        
                        if (phase != last_phase) {
                            println!("[LCU Watcher] LCU status changed: {} -> {}", last_phase, phase);
                            emit_terminal_log(&app_handle, &format!("[LCU Watcher] LCU status changed: {} -> {}", last_phase, phase));
                            
                            // If entering ChampSelect, preload assets to speed up injection later
                            if phase == "ChampSelect" {
                                emit_terminal_log(&app_handle, "[LCU Watcher] Champion select started, preparing for skin injection");
                                
                                // Preload by ensuring the champions directory exists and overlay directory is clean
                                let champions_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."))
                                    .join("champions");
                                
                                if !champions_dir.exists() {
                                    if let Err(e) = fs::create_dir_all(&champions_dir) {
                                        println!("[LCU Watcher] Failed to create champions directory: {}", e);
                                    }
                                }
                                
                                // Clean up any existing overlay for faster injection later
                                let app_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."));
                                let overlay_dir = app_dir.join("overlay");
                                if overlay_dir.exists() {
                                    if let Err(e) = fs::remove_dir_all(&overlay_dir) {
                                        println!("[LCU Watcher] Failed to clean overlay directory: {}", e);
                                    }
                                }
                            }
                        }
                        
                        // Create a variable to track our sleep duration - we'll use shorter intervals during champion select
                        let mut sleep_duration = Duration::from_secs(5);
                        
                        if (phase == "ChampSelect") {
                            // Use shorter polling interval during champion select (1.5 seconds instead of 5)
                            // But use adaptive polling - if we've seen the same champion multiple times, we can slow down
                            static mut STABLE_CHAMPION_COUNT: i32 = 0;
                            sleep_duration = if unsafe { STABLE_CHAMPION_COUNT > 3 } {
                                Duration::from_millis(2500) // Slower polling if champion selection is stable
                            } else {
                                Duration::from_millis(1200) // Faster polling when actively selecting
                            };
                            
                            // Get the current session to check selected champion
                            let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
                            let auth = base64::encode(format!("riot:{}", token));
                            
                            // Try to use the persistent HTTP client if available
                            let client = get_lcu_client(); // Use our optimized HTTP client
                            
                            match client.get(&session_url)
                                .header("Authorization", format!("Basic {}", auth))
                                .send() 
                            {
                                Ok(resp) => {
                                    if (resp.status().is_success()) {
                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Session API returned status: {}", resp.status()));
                                        
                                        match resp.json::<serde_json::Value>() {
                                            Ok(json) => {
                                                // Track if we already injected skins in this champion select session
                                                // to avoid doing it repeatedly
                                                static mut LAST_INJECTED_CHAMPION: i64 = -1;
                                                let mut previous_injected_champion;
                                                unsafe { previous_injected_champion = LAST_INJECTED_CHAMPION; }
                                                
                                                // Debug log to see what's in the session
                                                emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Local player cell ID exists: {}", json.get("localPlayerCellId").is_some()));
                                                
                                                // Get the local player's cell ID
                                                if let Some(local_player_cell_id) = json.get("localPlayerCellId").and_then(|v| v.as_i64()) {
                                                    emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Local player cell ID: {}", local_player_cell_id));
                                                    
                                                    // First look for locked in champions - highest priority
                                                    let mut selected_champion_id = 0;
                                                    let mut is_locked_in = false;
                                                    
                                                    if let Some(actions) = json.get("actions").and_then(|v| v.as_array()) {
                                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Found {} action groups", actions.len()));
                                                        
                                                        for (i, action_group) in actions.iter().enumerate() {
                                                            if let Some(actions) = action_group.as_array() {
                                                                emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Action group {} has {} actions", i, actions.len()));
                                                                
                                                                for (j, action) in actions.iter().enumerate() {
                                                                    if let Some(actor_cell_id) = action.get("actorCellId").and_then(|v| v.as_i64()) {
                                                                        if actor_cell_id == local_player_cell_id {
                                                                            emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Found action {} for local player", j));
                                                                            
                                                                            // Check if locked in
                                                                            let completed = action.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
                                                                            let champion_id = action.get("championId").and_then(|v| v.as_i64()).unwrap_or(0);
                                                                            
                                                                            emit_terminal_log(&app_handle, &format!(
                                                                                "[LCU Watcher Debug] Action details - completed: {}, champion_id: {}", 
                                                                                completed, champion_id
                                                                            ));
                                                                            
                                                                            if completed {
                                                                                if champion_id > 0 {
                                                                                    is_locked_in = true;
                                                                                    selected_champion_id = champion_id;
                                                                                    emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Champion {} is locked in", champion_id));
                                                                                    break;
                                                                                }
                                                                            }
                                                                            
                                                                            // Even if not locked in, capture the selected champion
                                                                            // so we can pre-inject skins when champion is just selected
                                                                            if !is_locked_in {
                                                                                if champion_id > 0 {
                                                                                    selected_champion_id = champion_id;
                                                                                    emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Champion {} is selected but not locked", champion_id));
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        emit_terminal_log(&app_handle, "[LCU Watcher Debug] No actions found in session");
                                                    }
                                                    
                                                    // If no champion found in actions, check in my selection directly
                                                    if selected_champion_id == 0 {
                                                        emit_terminal_log(&app_handle, "[LCU Watcher Debug] Checking myTeam for champion selection");
                                                        
                                                        if let Some(my_team) = json.get("myTeam").and_then(|v| v.as_array()) {
                                                            emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] myTeam has {} players", my_team.len()));
                                                            
                                                            for (i, player) in my_team.iter().enumerate() {
                                                                if let Some(cell_id) = player.get("cellId").and_then(|v| v.as_i64()) {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Player {} has cellId {}", i, cell_id));
                                                                    
                                                                    if cell_id == local_player_cell_id {
                                                                        if let Some(champion_id) = player.get("championId").and_then(|v| v.as_i64()) {
                                                                            emit_terminal_log(&app_handle, &format!("[LCU Watcher Debug] Found champion {} for local player in myTeam", champion_id));
                                                                            
                                                                            if champion_id > 0 {
                                                                                selected_champion_id = champion_id;
                                                                                // Assuming that if champion is in myTeam, it's locked in
                                                                                is_locked_in = true;
                                                                                break;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            emit_terminal_log(&app_handle, "[LCU Watcher Debug] No myTeam found in session");
                                                        }
                                                    }
                                                    
                                                    // Enhanced champion shuffle detection system
                                                    static mut LAST_SEEN_CHAMPION: i64 = -1;
                                                    static mut CHAMPION_SELECTION_COUNTER: i32 = 0;
                                                    static mut LAST_CHAMPION_SELECTION_TIME: u128 = 0;
                                                    static mut CHAMPIONS_VISITED: i32 = 0;
                                                    
                                                    // Get current time for timing-based shuffle detection
                                                    let now = std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap_or_default()
                                                        .as_millis();
                                                    
                                                    // Detect champion shuffling and stability
                                                    let (is_selection_stable, is_shuffling) = unsafe {
                                                        // Check if champion changed
                                                        if LAST_SEEN_CHAMPION != selected_champion_id {
                                                            // Calculate time since last champion change
                                                            let time_since_last_change = if LAST_CHAMPION_SELECTION_TIME > 0 {
                                                                now - LAST_CHAMPION_SELECTION_TIME
                                                            } else {
                                                                0
                                                            };
                                                            
                                                            emit_terminal_log(&app_handle, &format!(
                                                                "[LCU Watcher Debug] Champion selection changed from {} to {} (after {} ms)", 
                                                                LAST_SEEN_CHAMPION, selected_champion_id, time_since_last_change
                                                            ));
                                                            
                                                            // Track rapid champion changes as "shuffling"
                                                            let is_rapid_change = time_since_last_change < 1500; // Less than 1.5 seconds
                                                            if is_rapid_change && selected_champion_id > 0 {
                                                                CHAMPIONS_VISITED += 1;
                                                                emit_terminal_log(&app_handle, &format!(
                                                                    "[LCU Watcher Debug] Rapid champion change detected! Champions visited: {}", 
                                                                    CHAMPIONS_VISITED
                                                                ));
                                                            } else {
                                                                // Reset the shuffle counter if there's a longer pause
                                                                CHAMPIONS_VISITED = 0;
                                                            }
                                                            
                                                            // Update tracking variables
                                                            LAST_SEEN_CHAMPION = selected_champion_id;
                                                            LAST_CHAMPION_SELECTION_TIME = now;
                                                            CHAMPION_SELECTION_COUNTER = 0;
                                                            
                                                            (false, CHAMPIONS_VISITED > 2) // Not stable, and shuffling if >2 champions visited
                                                        } else {
                                                            // Same champion as last check
                                                            let time_since_selection = now - LAST_CHAMPION_SELECTION_TIME;
                                                            CHAMPION_SELECTION_COUNTER += 1;
                                                            
                                                            emit_terminal_log(&app_handle, &format!(
                                                                "[LCU Watcher Debug] Champion {} stable for {} checks ({} ms)", 
                                                                selected_champion_id, CHAMPION_SELECTION_COUNTER, time_since_selection
                                                            ));
                                                            
                                                            // Consider selection stable based on checks and time
                                                            let enough_checks = CHAMPION_SELECTION_COUNTER >= 2;
                                                            let enough_time = time_since_selection > 1500; // 1.5 seconds
                                                            
                                                            // If user was shuffling, require more stability evidence
                                                            let stability_threshold = if CHAMPIONS_VISITED > 2 {
                                                                enough_checks && enough_time
                                                            } else {
                                                                enough_checks || enough_time
                                                            };
                                                            
                                                            (stability_threshold, CHAMPIONS_VISITED > 2)
                                                        }
                                                    };
                                                    
                                                    // More intelligent injection criteria
                                                    let should_inject = selected_champion_id > 0 && (
                                                        is_locked_in || // Either locked in (highest priority)
                                                        (is_selection_stable && !is_shuffling) // Or stable selection (when not actively shuffling)
                                                    );
                                                    
                                                    // Reset LAST_INJECTED_CHAMPION when champion changes
                                                    // This allows reinjection when user changes champions
                                                    if unsafe { LAST_INJECTED_CHAMPION > 0 && LAST_INJECTED_CHAMPION != selected_champion_id && selected_champion_id > 0 } {
                                                        emit_terminal_log(&app_handle, &format!(
                                                            "[LCU Watcher] Champion selection changed from {} to {}, will reinject if needed", 
                                                            unsafe { LAST_INJECTED_CHAMPION }, selected_champion_id
                                                        ));
                                                        unsafe { LAST_INJECTED_CHAMPION = -1; }
                                                    }
                                                    
                                                    // Only proceed with injection if we have a champion ready for injection and haven't already injected for this champion
                                                    if should_inject && unsafe { LAST_INJECTED_CHAMPION != selected_champion_id } {
                                                        // Mark that we've processed this champion
                                                        unsafe { LAST_INJECTED_CHAMPION = selected_champion_id };
                                                        
                                                        println!("[LCU Watcher] Champion {} locked in", selected_champion_id);
                                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Champion {} locked in", selected_champion_id));
                                                        
                                                        // Load saved skins from config.json
                                                        let config_dir = app_handle.path().app_data_dir()
                                                            .unwrap_or_else(|_| PathBuf::from("."))
                                                            .join("config");
                                                        let cfg_file = config_dir.join("config.json");
                                                        
                                                        match std::fs::read_to_string(&cfg_file) {
                                                            Ok(data) => {
                                                                match serde_json::from_str::<SavedConfig>(&data) {
                                                                    Ok(config) => {
                                                                        // Find the skin for the selected champion
                                                                        if let Some(skin) = config.skins.iter().find(|s| s.champion_id == selected_champion_id as u32) {
                                                                            // Prepare skin injection
                                                                            println!("[LCU Watcher] Injecting skin for champion {}: skin_id={}", 
                                                                                selected_champion_id, skin.skin_id);
                                                                            emit_terminal_log(&app_handle, &format!("[LCU Watcher] Injecting skin for champion {}: skin_id={}", 
                                                                                selected_champion_id, skin.skin_id));
                                                                            
                                                                            // Prepare the skin for injection
                                                                            let skins = vec![Skin {
                                                                                champion_id: skin.champion_id,
                                                                                skin_id: skin.skin_id,
                                                                                chroma_id: skin.chroma_id,
                                                                                fantome_path: skin.fantome.clone(),
                                                                            }];
                                                                            
                                                                            // Get the champions directory for fantome files
                                                                            let champions_dir = app_handle.path().app_data_dir()
                                                                                .unwrap_or_else(|_| PathBuf::from("."))
                                                                                .join("champions");
                                                                            
                                                                            // Inject the skin with a retry mechanism
                                                                            let max_injection_retries = 2;
                                                                            let mut injection_attempts = 0;
                                                                            let mut injection_success = false;
                                                                            
                                                                            while injection_attempts < max_injection_retries && !injection_success {
                                                                                if injection_attempts > 0 {
                                                                                    emit_terminal_log(&app_handle, &format!("[LCU Watcher] Retry {} of {} for skin injection", 
                                                                                        injection_attempts, max_injection_retries));
                                                                                    thread::sleep(Duration::from_millis(800)); // Short delay between retries
                                                                                }
                                                                                
                                                                                match crate::injection::inject_skins(
                                                                                    &app_handle,
                                                                                    &league_path_clone,
                                                                                    &skins,
                                                                                    &champions_dir
                                                                                ) {
                                                                                    Ok(_) => {
                                                                                        println!("[LCU Watcher] Successfully injected skin for champion {}", selected_champion_id);
                                                                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Successfully injected skin for champion {}", selected_champion_id));
                                                                                        injection_success = true;
                                                                                        break;
                                                                                    },
                                                                                    Err(e) => {
                                                                                        println!("[LCU Watcher] Injection attempt {} failed: {}", injection_attempts + 1, e);
                                                                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Injection attempt {} failed: {}", injection_attempts + 1, e));
                                                                                        injection_attempts += 1;
                                                                                    },
                                                                                }
                                                                            }
                                                                            
                                                                            // Check if all attempts failed
                                                                            if !injection_success {
                                                                                let error_msg = format!("Failed to inject skin for champion {} after {} attempts", selected_champion_id, max_injection_retries);
                                                                                println!("[LCU Watcher] {}", error_msg);
                                                                                emit_terminal_log(&app_handle, &format!("[LCU Watcher] {}", error_msg));
                                                                                let _ = app_handle.emit("skin-injection-error", error_msg);
                                                                            }
                                                                        } else {
                                                                            println!("[LCU Watcher] No skin configured for champion {}", selected_champion_id);
                                                                            emit_terminal_log(&app_handle, &format!("[LCU Watcher] No skin configured for champion {}", selected_champion_id));
                                                                        }
                                                                    },
                                                                    Err(e) => println!("[LCU Watcher] Failed to parse config.json: {}", e),
                                                                }
                                                            },
                                                            Err(e) => println!("[LCU Watcher] Failed to read config.json: {}", e),
                                                        }
                                                    } else if selected_champion_id > 0 && !is_locked_in {
                                                        // Just log that a champion is selected but not locked in yet
                                                        println!("[LCU Watcher] Champion {} selected but not locked in", selected_champion_id);
                                                        emit_terminal_log(&app_handle, &format!("[LCU Watcher] Champion {} selected but not locked in", selected_champion_id));
                                                    }
                                                }
                                            },
                                            Err(e) => println!("[LCU Watcher] Failed to parse session data: {}", e),
                                        }
                                    }
                                },
                                Err(e) => println!("[LCU Watcher] Failed to get session data: {}", e),
                            }
                        } else {
                            // Reset the last injected champion ID when we leave champion select
                            unsafe {
                                static mut LAST_INJECTED_CHAMPION: i64 = -1;
                                LAST_INJECTED_CHAMPION = -1;
                            }
                        }
                        
                        last_phase = phase.to_string();
                    },
                    Err(e) => println!("Failed to build HTTP client: {}", e),
                }
                
                // Sleep for the appropriate duration before checking again
                thread::sleep(sleep_duration);
            }
        }
    });
    
    println!("LCU status watcher thread started");
    Ok(())
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
pub async fn start_auto_inject(app: AppHandle, leaguePath: String) -> Result<(), String> {
    println!("Starting auto-inject for path: {}", leaguePath);
    
    // Start the LCU watcher in a separate thread
    start_lcu_watcher(app, leaguePath)?;
    
    Ok(())
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