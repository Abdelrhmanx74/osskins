use tauri::Manager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::injection::{Skin, inject_skins as inject_skins_impl};
use crate::injection;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSkinData {
    pub id: String,
    pub name: String,
    pub champion_id: u32,
    pub champion_name: String,
    pub file_path: String,
    pub created_at: u64,
    pub preview_image: Option<String>,
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
    if (!champions_dir.exists()) {
        return Ok("[]".to_string()); // Return empty array if no champions directory exists
    }

    // If champion_id is 0, return all champions
    if (champion_id == 0) {
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
    if (!champions_dir.exists()) {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerminalLog {
    pub message: String,
    pub log_type: String, // e.g. "lcu-watcher", "injection", "error", etc.
    pub timestamp: String,
}

fn emit_terminal_log(app: &AppHandle, message: &str, log_type: &str) {
    let log = TerminalLog {
        message: message.to_string(),
        log_type: log_type.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let _ = app.emit("terminal-log", log);
}

// Add helper function for cleaner log messages
fn format_json_summary(json: &serde_json::Value) -> String {
    let mut summary = String::new();
    
    if let Some(phase) = json.get("phase") {
        summary.push_str(&format!("phase: {}, ", phase.as_str().unwrap_or("unknown")));
    }
    
    if let Some(game_data) = json.get("gameData") {
        summary.push_str("gameData: {...}, ");
    }
    
    if let Some(actions) = json.get("actions") {
        summary.push_str(&format!("actions: [{} items], ", actions.as_array().map_or(0, |a| a.len())));
    }
    
    if summary.is_empty() {
        summary = "[Response summary unavailable]".to_string();
    }
    
    summary
}

// Helper function for delayed logging
fn delayed_log(app: &AppHandle, message: &str) {
    emit_terminal_log(app, message, "debug");
    thread::sleep(Duration::from_millis(100)); // Small delay for better readability
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
        
        // Track last seen selections to detect changes
        let mut last_selected_skins: std::collections::HashMap<u32, SkinData> = std::collections::HashMap::new();
        let mut last_skin_check_time = std::time::Instant::now();
        let mut last_champion_id: Option<u32> = None;
        
        loop {
            let mut sleep_duration = Duration::from_secs(5);
            
            let log_msg = format!("Monitoring directory: {}", league_path_clone);
            println!("{}", log_msg);
            emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");
            
            // Only check the configured League directory for lockfile
            let search_dirs = [PathBuf::from(&league_path_clone)];
            let mut port = None;
            let mut token = None;
            let mut found_any_lockfile = false;
            let mut lockfile_path = None;
            
            // Rest of the lockfile detection code remains the same
            for dir in &search_dirs {
                let log_msg = format!("Looking for lockfiles in: {}", dir.display());
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");
                
                // Check each possible lockfile name
                for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
                    let path = dir.join(name);
                    if path.exists() {
                        found_any_lockfile = true;
                        lockfile_path = Some(path.clone());
                        println!("Found lockfile: {}", path.display());
                        emit_terminal_log(&app_handle, &format!("Found lockfile: {}", path.display()), "lcu-watcher");
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        let parts: Vec<&str> = content.split(':').collect();
                        if parts.len() >= 5 {
                            port = Some(parts[2].to_string());
                            token = Some(parts[3].to_string());
                            found_any_lockfile = true;
                            break;
                        }
                    }
                }
                
                if port.is_some() && token.is_some() {
                    break;
                }
            }
            
            if (!found_any_lockfile) {
                // Handle no lockfile found cases...
                if was_in_game && (last_phase == "InProgress" || was_reconnecting) {
                    thread::sleep(Duration::from_secs(5));
                    continue;
                } else if was_in_game && last_phase == "None" {
                    if let Err(e) = crate::injection::cleanup_injection(&app_handle, &league_path_clone) {
                        println!("Error cleaning up injection: {}", e);
                        emit_terminal_log(&app_handle, &format!("Error cleaning up injection: {}", e), "error");
                    }
                    was_in_game = false;
                }
                
                let log_msg = format!("No valid lockfile found. Is League running? The lockfile should be at: {}", league_path_clone);
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg, "lcu-watcher");
                thread::sleep(Duration::from_secs(5));
                continue;
            }
            
            let port = port.unwrap();
            let token = token.unwrap();
            let lockfile_path = lockfile_path.unwrap();
            
            'lcu_connected: loop {
                if !lockfile_path.exists() {
                    break 'lcu_connected;
                }

                match reqwest::blocking::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build() 
                {
                    Ok(client) => {
                        let endpoints = [
                            "/lol-gameflow/v1/session",
                            "/lol-gameflow/v1/gameflow-phase",
                        ];
                        
                        let mut connected = false;
                        let mut phase_value: Option<String> = None;
                        
                        for endpoint in endpoints {
                            let url = format!("https://127.0.0.1:{}{}", port, endpoint);
                            let auth = base64::encode(format!("riot:{}", token));
                            
                            match client.get(&url)
                                .header("Authorization", format!("Basic {}", auth))
                                .send() 
                            {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        connected = true;
                                        
                                        match resp.json::<serde_json::Value>() {
                                            Ok(json) => {
                                                if endpoint == "/lol-gameflow/v1/gameflow-phase" {
                                                    if let Some(phase) = json.as_str() {
                                                        phase_value = Some(phase.to_string());
                                                        break;
                                                    }
                                                } else {
                                                    if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
                                                        phase_value = Some(phase.to_string());
                                                        break;
                                                    }
                                                }
                                            },
                                            Err(e) => println!("Failed to parse response from {}: {}", endpoint, e),
                                        }
                                    }
                                },
                                Err(e) => println!("Failed to connect to endpoint {}: {}", endpoint, e),
                            }
                        }
                        
                        if (!connected) {
                            thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                        
                        let phase = phase_value.unwrap_or_else(|| "None".to_string());
                        
                        if phase != last_phase {
                            println!("LCU status changed: {} -> {}", last_phase, phase);
                            
                            // If entering ChampSelect, preload assets to speed up injection later
                            if phase == "ChampSelect" {
                                let champions_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."))
                                    .join("champions");
                                
                                if !champions_dir.exists() {
                                    if let Err(e) = fs::create_dir_all(&champions_dir) {
                                        println!("Failed to create champions directory: {}", e);
                                    }
                                }
                                
                                let app_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."));
                                let overlay_dir = app_dir.join("overlay");
                                if overlay_dir.exists() {
                                    if let Err(e) = fs::remove_dir_all(&overlay_dir) {
                                        println!("Failed to clean overlay directory: {}", e);
                                    }
                                }
                            }
                        }
                        
                        if phase == "ChampSelect" {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_skin_check_time).as_secs() >= 1 {
                                last_skin_check_time = now;
                                
                                let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
                                let auth = base64::encode(format!("riot:{}", token));
                                
                                if let Ok(resp) = client.get(&session_url)
                                    .header("Authorization", format!("Basic {}", auth))
                                    .send() 
                                {
                                    if resp.status().is_success() {
                                        if let Ok(json) = resp.json::<serde_json::Value>() {
                                            if let Some(selected_champ_id) = get_selected_champion_id(&json) {
                                                let current_champion_id = selected_champ_id as u32;
                                                
                                                let champion_changed = last_champion_id != Some(current_champion_id) && current_champion_id > 0;
                                                if champion_changed {
                                                    last_champion_id = Some(current_champion_id);
                                                    
                                                    let config_dir = app_handle.path().app_data_dir()
                                                        .unwrap_or_else(|_| PathBuf::from("."))
                                                        .join("config");
                                                    let cfg_file = config_dir.join("config.json");
                                                    
                                                    if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                                                        if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                                                            if let Some(skin) = config.skins.iter().find(|s| s.champion_id == current_champion_id) {
                                                                
                                                                let skins = vec![Skin {
                                                                    champion_id: skin.champion_id,
                                                                    skin_id: skin.skin_id,
                                                                    chroma_id: skin.chroma_id,
                                                                    fantome_path: skin.fantome.clone(),
                                                                }];
                                                                
                                                                let champions_dir = app_handle.path().app_data_dir()
                                                                    .unwrap_or_else(|_| PathBuf::from("."))
                                                                    .join("champions");
                                                                
                                                                match crate::injection::inject_skins(
                                                                    &app_handle,
                                                                    &league_path_clone,
                                                                    &skins,
                                                                    &champions_dir
                                                                ) {
                                                                    Ok(_) => {
                                                                        let _ = app_handle.emit("injection-status", "success");
                                                                    },
                                                                    Err(e) => {
                                                                        let _ = app_handle.emit("skin-injection-error", format!(
                                                                            "Failed to inject pre-selected skin for champion {}: {}", current_champion_id, e
                                                                        ));
                                                                        let _ = app_handle.emit("injection-status", "error");
                                                                    }
                                                                }
                                                                
                                                                last_selected_skins.insert(current_champion_id, skin.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                last_champion_id = None;
                                            }
                                        }
                                    }
                                }
                                
                                let config_dir = app_handle.path().app_data_dir()
                                    .unwrap_or_else(|_| PathBuf::from("."))
                                    .join("config");
                                let cfg_file = config_dir.join("config.json");
                                
                                if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                                    if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                                        let mut skin_changes = false;
                                        
                                        for skin in &config.skins {
                                            let champ_id = skin.champion_id;
                                            
                                            if last_champion_id == Some(champ_id) {
                                                if !last_selected_skins.contains_key(&champ_id) ||
                                                   last_selected_skins.get(&champ_id).map_or(true, |old_skin| {
                                                       old_skin.skin_id != skin.skin_id || 
                                                       old_skin.chroma_id != skin.chroma_id || 
                                                       old_skin.fantome != skin.fantome
                                                   }) 
                                                {
                                                    
                                                    let skins = vec![Skin {
                                                        champion_id: skin.champion_id,
                                                        skin_id: skin.skin_id,
                                                        chroma_id: skin.chroma_id,
                                                        fantome_path: skin.fantome.clone(),
                                                    }];
                                                    
                                                    let champions_dir = app_handle.path().app_data_dir()
                                                        .unwrap_or_else(|_| PathBuf::from("."))
                                                        .join("champions");
                                                    
                                                    if phase != "ChampSelect" {
                                                        continue;
                                                    }
                                                    
                                                    match crate::injection::inject_skins(
                                                        &app_handle,
                                                        &league_path_clone,
                                                        &skins,
                                                        &champions_dir
                                                    ) {
                                                        Ok(_) => {
                                                            let _ = app_handle.emit("injection-status", "success");
                                                        },
                                                        Err(e) => {
                                                            let _ = app_handle.emit("skin-injection-error", format!(
                                                                "Failed to inject skin for champion {}: {}", champ_id, e
                                                            ));
                                                            let _ = app_handle.emit("injection-status", "error");
                                                        }
                                                    }
                                                    
                                                    last_selected_skins.insert(champ_id, skin.clone());
                                                    skin_changes = true;
                                                }
                                            }
                                        }
                                        
                                        if skin_changes {
                                            emit_terminal_log(&app_handle, "Updated skin selection tracking", "lcu-watcher");
                                        }
                                    }
                                }
                            }
                            
                            if phase != "ChampSelect" && phase != "None" && last_phase == "ChampSelect" {
                                let _ = crate::injection::cleanup_injection(&app_handle, &league_path_clone);
                            }
                            
                            sleep_duration = Duration::from_secs(1);
                        } else if phase == "InProgress" {
                            // Keep existing in-game phase behavior
                        }

                        // Handle Swift Play mode - detect Lobby -> Matchmaking transition
                        if last_phase == "Lobby" && phase == "Matchmaking" {
                            emit_terminal_log(&app_handle, "Detected transition from Lobby to Matchmaking, checking for Swift Play mode", "lcu-watcher");
                            
                            // Check current queue information
                            let queue_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", port);
                            let auth = base64::encode(format!("riot:{}", token));
                            
                            match client.get(&queue_url)
                                .header("Authorization", format!("Basic {}", auth))
                                .send()
                            {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        match resp.json::<serde_json::Value>() {
                                            Ok(json) => {
                                                // Log the full response structure for debugging
                                                emit_terminal_log(&app_handle, "[LCU Debug] Swift Play session structure:", "debug");
                                                // Print important paths in the JSON that might contain champion selections
                                                if let Some(game_data) = json.get("gameData") {
                                                    if let Some(queue) = game_data.get("queue") {
                                                        if let Some(queue_id) = queue.get("id").and_then(|id| id.as_i64()) {
                                                            emit_terminal_log(&app_handle, &format!("[LCU Debug] Queue ID: {}", queue_id), "debug");
                                                            
                                                            // Check if this is Swift Play queue (both ID 1700 and ID 480)
                                                            if queue_id == 1700 || queue_id == 480 {
                                                                emit_terminal_log(&app_handle, &format!("Confirmed Swift Play queue or compatible queue (ID: {})", queue_id), "lcu-watcher");
                                                                
                                                                // Log player selection data
                                                                if let Some(player_selections) = game_data.get("playerChampionSelections") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] playerChampionSelections: {}", player_selections), "debug");
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Debug] No playerChampionSelections found", "debug");
                                                                }
                                                                
                                                                if let Some(selected_champs) = game_data.get("selectedChampions") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] selectedChampions: {}", selected_champs), "debug");
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "[LCU Debug] No selectedChampions found", "debug");
                                                                }
                                                                
                                                                // Check for local player data
                                                                if let Some(local_player) = json.get("localPlayerSelection") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] localPlayerSelection: {}", local_player), "debug");
                                                                }
                                                                
                                                                // Check for team data
                                                                if let Some(my_team) = json.get("myTeam") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] myTeam: {}", my_team), "debug");
                                                                }
                                                                
                                                                // Check for role assignments
                                                                if let Some(roles) = json.get("roleAssignments") {
                                                                    emit_terminal_log(&app_handle, &format!("[LCU Debug] roleAssignments: {}", roles), "debug");
                                                                }
                                                                
                                                                // Get player champion selections for Swift Play
                                                                let swift_play_champion_ids = get_swift_play_champion_selections(&json);
                                                                
                                                                if !swift_play_champion_ids.is_empty() {
                                                                    emit_terminal_log(&app_handle, &format!(
                                                                        "Swift Play: Found {} champion selections: {:?}", 
                                                                        swift_play_champion_ids.len(), 
                                                                        swift_play_champion_ids
                                                                    ), "lcu-watcher");
                                                                    
                                                                    // Inject skins for all selected champions
                                                                    inject_skins_for_champions(&app_handle, &league_path_clone, &swift_play_champion_ids);
                                                                } else {
                                                                    emit_terminal_log(&app_handle, "Swift Play: No champion selections found in session data", "lcu-watcher");
                                                                    
                                                                    // Try checking additional endpoints to find Swift Play champions
                                                                    let swift_play_url = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
                                                                    match client.get(&swift_play_url)
                                                                        .header("Authorization", format!("Basic {}", auth))
                                                                        .send()
                                                                    {
                                                                        Ok(swift_resp) => {
                                                                            if swift_resp.status().is_success() {
                                                                                if let Ok(lobby_json) = swift_resp.json::<serde_json::Value>() {
                                                                                    emit_terminal_log(&app_handle, "[LCU Debug] Swift Play lobby data found", "debug");
                                                                                    
                                                                                    // Try to extract champion IDs from lobby data
                                                                                    let lobby_champion_ids = extract_swift_play_champions_from_lobby(&lobby_json);
                                                                                    
                                                                                    if !lobby_champion_ids.is_empty() {
                                                                                        emit_terminal_log(&app_handle, &format!(
                                                                                            "Swift Play: Found {} champion selections from lobby: {:?}", 
                                                                                            lobby_champion_ids.len(), 
                                                                                            lobby_champion_ids
                                                                                        ), "lcu-watcher");
                                                                                        
                                                                                        // Inject skins for all selected champions from lobby
                                                                                        inject_skins_for_champions(&app_handle, &league_path_clone, &lobby_champion_ids);
                                                                                    } else {
                                                                                        emit_terminal_log(&app_handle, "Swift Play: No champion selections found in lobby data", "lcu-watcher");
                                                                                        emit_terminal_log(&app_handle, &format!("[LCU Debug] Full lobby data: {}", 
                                                                                            serde_json::to_string_pretty(&lobby_json).unwrap_or_default()), "debug");
                                                                                    }
                                                                                }
                                                                            }
                                                                        },
                                                                        Err(e) => emit_terminal_log(&app_handle, &format!("[LCU Debug] Failed to get lobby data: {}", e), "debug"),
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                            Err(e) => println!("Failed to parse queue info: {}", e),
                                        }
                                    }
                                },
                                Err(e) => println!("Failed to get queue information: {}", e),
                            }
                        }
                        
                        last_phase = phase.to_string();
                        was_reconnecting = phase == "Reconnect";
                        was_in_game = phase == "InProgress" || was_reconnecting;
                    },
                    Err(e) => println!("Failed to build HTTP client: {}", e),
                }
                
                thread::sleep(sleep_duration);
            }
        }
    });
    
    println!("LCU status watcher thread started");
    Ok(())
}

// Helper function to get selected champion ID from session JSON
fn get_selected_champion_id(session_json: &serde_json::Value) -> Option<i64> {
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

// Helper function to get Swift Play champion selections from session JSON
fn get_swift_play_champion_selections(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    // Method 1: Look in gameData -> playerChampionSelections
    if let Some(game_data) = json.get("gameData") {
        if let Some(selections) = game_data.get("playerChampionSelections").and_then(|p| p.as_array()) {
            // Get local player's summoner ID first
            let local_summoner_id = json.get("localPlayerSelection")
                .and_then(|lp| lp.get("summonerId"))
                .and_then(|id| id.as_i64());
                
            if let Some(local_id) = local_summoner_id {
                for selection in selections {
                    // Check if this is the local player
                    if let Some(player_id) = selection.get("summonerId").and_then(|id| id.as_i64()) {
                        if player_id == local_id {
                            // Extract champion IDs
                            if let Some(champs) = selection.get("championIds").and_then(|ids| ids.as_array()) {
                                for champ in champs {
                                    if let Some(id) = champ.as_i64() {
                                        if id > 0 && !champion_ids.contains(&id) {
                                            champion_ids.push(id);
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
    
    // Method 2: Look in gameData -> selectedChampions
    if champion_ids.is_empty() {
        if let Some(game_data) = json.get("gameData") {
            if let Some(selected_champions) = game_data.get("selectedChampions").and_then(|sc| sc.as_array()) {
                for selection in selected_champions {
                    if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        }
    }
    
    // Method 3: Look in the player's team data
    if champion_ids.is_empty() {
        if let Some(team) = json.get("myTeam").and_then(|t| t.as_array()) {
            let player_name = json.get("playerName").and_then(|p| p.as_str()).unwrap_or("");
            
            for player in team {
                let is_local_player = player.get("summonerName")
                    .and_then(|n| n.as_str())
                    .map_or(false, |name| name == player_name);
                
                if is_local_player {
                    // Primary champion
                    if let Some(champion_id) = player.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    }
                    
                    // Secondary champion
                    if let Some(secondary_id) = player.get("secondaryChampionId").and_then(|id| id.as_i64()) {
                        if secondary_id > 0 && !champion_ids.contains(&secondary_id) {
                            champion_ids.push(secondary_id);
                        }
                    }
                }
            }
        }
    }
    
    // Try one more method for Swift Play
    if champion_ids.is_empty() {
        if let Some(roles) = json.get("roleAssignments").and_then(|r| r.as_array()) {
            for role in roles {
                if let Some(champion_id) = role.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                        champion_ids.push(champion_id);
                    }
                }
            }
        }
    }
    
    // Method 4: Check lobby data playerSlots for Swift Play
    if champion_ids.is_empty() {
        // Try to find champions in localMember.playerSlots (common in Swift Play)
        if let Some(local_member) = json.get("localMember") {
            if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                for slot in player_slots {
                    if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                        if champion_id > 0 && !champion_ids.contains(&champion_id) {
                            champion_ids.push(champion_id);
                        }
                    }
                }
            }
        }
    }
    
    champion_ids
}

// Helper function to inject skins for multiple champions (used in Swift Play)
fn inject_skins_for_champions(app: &AppHandle, league_path: &str, champion_ids: &[i64]) {
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

// Extract Swift Play champion IDs from the lobby data directly
fn extract_swift_play_champions_from_lobby(json: &serde_json::Value) -> Vec<i64> {
    let mut champion_ids = Vec::new();
    
    if let Some(local_member) = json.get("localMember") {
        if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
            for slot in player_slots {
                if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
                    if champion_id > 0 && !champion_ids.contains(&champion_id) {
                        champion_ids.push(champion_id);
                    }
                }
            }
        }
    }
    
    champion_ids
}

#[tauri::command]
pub async fn upload_custom_skin(
    app: tauri::AppHandle,
    championId: u32,
    skinName: String,
) -> Result<CustomSkinData, String> {
    println!("Uploading custom skin: {}", skinName);
    println!("For champion ID: {}", championId);
    
    // Open file dialog for the user to select a skin file
    #[cfg(target_os = "windows")]
    let file_path = {
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut command = Command::new("powershell");
        command.creation_flags(CREATE_NO_WINDOW);
        command.args([
            "-NoProfile",
            "-Command",
            r#"Add-Type -AssemblyName System.Windows.Forms; 
            $dialog = New-Object System.Windows.Forms.OpenFileDialog;
            $dialog.Filter = 'Skin files (*.fantome;*.wad;*.client;*.zip)|*.fantome;*.wad;*.client;*.zip';
            $dialog.Title = 'Select Custom Skin File';
            if($dialog.ShowDialog() -eq 'OK') { $dialog.FileName }"#,
        ]);
        
        let output = command
            .output()
            .map_err(|e| format!("Failed to execute file dialog command: {}", e))?;

        if !output.status.success() {
            return Err("File selection cancelled".to_string());
        }

        let path = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse selected path: {}", e))?
            .trim()
            .to_string();

        if path.is_empty() {
            return Err("No file selected".to_string());
        }
        
        path
    };
    
    #[cfg(not(target_os = "windows"))]
    let file_path = {
        return Err("File selection is only supported on Windows for now".to_string());
    };
    
    println!("Selected file: {}", file_path);
    
    // Get the app data directory
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    // Create custom skins directory if it doesn't exist
    let custom_skins_dir = app_data_dir.join("custom_skins");
    std::fs::create_dir_all(&custom_skins_dir)
        .map_err(|e| format!("Failed to create custom skins directory: {}", e))?;
        
    // Get champion name (for organization)
    let champion_name = if let Ok(champion_data) = get_champion_name(&app, championId).await {
        champion_data
    } else {
        format!("champion_{}", championId) // Fallback if name not found
    };
    
    // Create directory for this champion's custom skins
    let champion_dir = custom_skins_dir.join(&champion_name);
    std::fs::create_dir_all(&champion_dir)
        .map_err(|e| format!("Failed to create champion directory: {}", e))?;
        
    // Generate a unique ID for this skin
    let skin_id = format!("custom_{}_{}", championId, chrono::Utc::now().timestamp());
    
    // Copy the file to the custom skins directory with a new name
    let source_path = std::path::Path::new(&file_path);
    let file_ext = source_path.extension()
        .map(|ext| ext.to_string_lossy().to_string())
        .unwrap_or_else(|| "fantome".to_string());
    
    // Create filename: champion_name_skinid.extension
    let dest_filename = format!("{}_{}.{}", champion_name, skin_id, file_ext);
    let dest_path = champion_dir.join(&dest_filename);
    
    // Copy the file
    std::fs::copy(source_path, &dest_path)
        .map_err(|e| format!("Failed to copy skin file: {}", e))?;
        
    // Create metadata for the custom skin
    let custom_skin = CustomSkinData {
        id: skin_id,
        name: skinName,
        champion_id: championId,
        champion_name,
        file_path: dest_path.to_string_lossy().to_string(),
        created_at: chrono::Utc::now().timestamp() as u64,
        preview_image: None, // We'll leave preview generation for a future enhancement
    };
    
    // Save metadata about this custom skin
    save_custom_skin(&app, &custom_skin).await?;
    
    Ok(custom_skin)
}

#[tauri::command]
pub async fn get_custom_skins(
    app: tauri::AppHandle
) -> Result<Vec<CustomSkinData>, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    let file = config_dir.join("custom_skins.json");
    
    if !file.exists() {
        return Ok(Vec::new());
    }
    
    let data = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read custom skins data: {}", e))?;
        
    let custom_skins: Vec<CustomSkinData> = serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse custom skins data: {}", e))?;
        
    Ok(custom_skins)
}

#[tauri::command]
pub async fn delete_custom_skin(
    app: tauri::AppHandle,
    skin_id: String
) -> Result<(), String> {
    // Get all custom skins
    let custom_skins = get_custom_skins(app.clone()).await?;
    
    // Find the skin to delete
    let skin_to_delete = custom_skins.iter().find(|skin| skin.id == skin_id)
        .ok_or_else(|| format!("Custom skin with ID {} not found", skin_id))?;
    
    // Delete the skin file
    let file_path = std::path::Path::new(&skin_to_delete.file_path);
    if file_path.exists() {
        std::fs::remove_file(file_path)
            .map_err(|e| format!("Failed to delete skin file: {}", e))?;
    }
    
    // Update the custom skins list
    let updated_skins: Vec<CustomSkinData> = custom_skins.into_iter()
        .filter(|skin| skin.id != skin_id)
        .collect();
        
    // Save the updated list
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
        
    let file = config_dir.join("custom_skins.json");
    let data = serde_json::to_string_pretty(&updated_skins)
        .map_err(|e| format!("Failed to serialize custom skins: {}", e))?;
        
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write custom_skins.json: {}", e))?;
        
    Ok(())
}

// Helper functions

async fn get_champion_name(app: &tauri::AppHandle, champion_id: u32) -> Result<String, String> {
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
                                        if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
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

async fn save_custom_skin(app: &tauri::AppHandle, custom_skin: &CustomSkinData) -> Result<(), String> {
    // Get all existing custom skins
    let mut custom_skins = get_custom_skins(app.clone()).await.unwrap_or_default();
    
    // Add the new skin
    custom_skins.push(custom_skin.clone());
    
    // Save to file
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
        
    let file = config_dir.join("custom_skins.json");
    let data = serde_json::to_string_pretty(&custom_skins)
        .map_err(|e| format!("Failed to serialize custom skins: {}", e))?;
        
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write custom_skins.json: {}", e))?;
    
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
    if !overlay_cache_dir.exists() {
        std::fs::create_dir_all(&overlay_cache_dir)
            .map_err(|e| format!("Failed to create overlay cache directory: {}", e))?;
    }
    
    // Initialize the global file index to cache champion data
    if let Ok(index) = injection::get_global_index(app_handle) {
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
            if let Ok(mut injector) = injection::SkinInjector::new(&app_handle_clone, &league_path) {
                let _ = injector.initialize_cache();
                println!("Successfully preloaded injection resources");
            }
        } else {
            println!("League path not found, skipping preload");
        }
    });
    
    Ok(())
}

// Helper function to get league path from config
fn get_league_path_from_config(app_handle: &AppHandle) -> Option<String> {
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