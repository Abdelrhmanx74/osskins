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
            if (path.is_dir()) {
                // Look for JSON files in the champion directory
                for champion_file in fs::read_dir(path)
                    .map_err(|e| format!("Failed to read champion directory: {}", e))? {
                    let champion_file = champion_file.map_err(|e| format!("Failed to read champion file: {}", e))?;
                    let file_path = champion_file.path();
                    if (file_path.extension().and_then(|s| s.to_str()) == Some("json")) {
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
        if (path.is_dir()) {
            let champion_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("Invalid champion directory name"))?;
            let champion_file = path.join(format!("{}.json", champion_name));
            if (champion_file.exists()) {
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
    // Open a folder picker for the Riot Client installation directory
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; $dialog.Description = 'Select Riot Client installation directory (contains lockfile)'; $dialog.ShowNewFolderButton = $false; $dialog.RootFolder = 'MyComputer'; $dialog.ShowDialog() | Out-Null; $dialog.SelectedPath",
        ])
        .output()
        .map_err(|e| format!("Failed to execute powershell command: {}", e))?;

    if !output.status.success() {
        return Err("Folder selection cancelled".to_string());
    }

    let path = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse selected path: {}", e))?
        .trim()
        .to_string();

    if path.is_empty() {
        return Err("No folder selected".to_string());
    }

    Ok(path)
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
    
    // Verify the path still exists and contains League of Legends.exe
    let game_path = Path::new(&path);
    if (!game_path.exists() || !game_path.join("League of Legends.exe").exists()) {
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
    if (!Path::new(&game_path).exists()) {
        return Err(format!("League of Legends directory not found: {}", game_path));
    }
    
    // Validate fantome directory exists
    let base_path = Path::new(&fantome_files_dir);
    if (!base_path.exists()) {
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

// Existing save_selected_skins now also takes league_path and writes combined config.json
#[tauri::command]
pub async fn save_selected_skins(app: tauri::AppHandle, leaguePath: String, skins: Vec<SkinData>) -> Result<(), String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let file = config_dir.join("config.json");
    // build combined JSON
    let config_json = serde_json::json!({"league_path": leaguePath, "skins": skins});
    let data = serde_json::to_string_pretty(&config_json)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&file, data)
        .map_err(|e| format!("Failed to write config.json: {}", e))?;
    Ok(())
}

// New command to load config.json (league path + skins)
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedConfig {
    pub league_path: Option<String>,
    pub skins: Vec<SkinData>,
}

#[tauri::command]
pub async fn load_config(app: tauri::AppHandle) -> Result<SavedConfig, String> {
    let config_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("config");
    let file = config_dir.join("config.json");
    if !file.exists() {
        return Ok(SavedConfig { league_path: None, skins: Vec::new() });
    }
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;
    let cfg: SavedConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config.json: {}", e))?;
    Ok(cfg)
}

// In start_auto_inject: emit status change events
#[tauri::command]
pub async fn start_auto_inject(app: tauri::AppHandle, leaguePath: String) -> Result<(), String> {
    println!("Starting LCU status watcher for path: {}", leaguePath);
    let app_handle = app.clone();
    let league_path_clone = leaguePath.clone();
    
    thread::spawn(move || {
        let mut last_phase = String::new();
        // Set an initial status to make the dot visible
        let _ = app_handle.emit("lcu-status", "None".to_string());
        
        loop {
            println!("[LCU Watcher] Monitoring directory: {}", league_path_clone);
            
            // Check both current directory and parent directory for lockfile
            let search_dirs = [
                PathBuf::from(&league_path_clone),
                PathBuf::from(&league_path_clone).parent().unwrap_or(&PathBuf::from(&league_path_clone)).to_path_buf()
            ];
            
            let mut port = None;
            let mut token = None;
            
            // Try each directory, looking for lockfiles
            for dir in &search_dirs {
                println!("[LCU Watcher] Looking for lockfiles in: {}", dir.display());
                
                // Check each possible lockfile name
                for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
                    let path = dir.join(name);
                    println!("[LCU Watcher] Checking for lockfile: {} (exists={})", path.display(), path.exists());
                    
                    if let Ok(content) = fs::read_to_string(&path) {
                        let parts: Vec<&str> = content.split(':').collect();
                        if parts.len() >= 5 {
                            port = Some(parts[2].to_string());
                            token = Some(parts[3].to_string());
                            println!("[LCU Watcher] Found valid lockfile at: {}", path.display());
                            break;
                        } else {
                            println!("[LCU Watcher] Invalid lockfile format at: {}", path.display());
                        }
                    }
                }
                
                if port.is_some() && token.is_some() {
                    break; // Stop checking directories if we found a valid lockfile
                }
            }
            
            if port.is_none() || token.is_none() {
                println!("[LCU Watcher] No valid lockfile found. Is League running? The lockfile should be at: D:\\Mana\\Riot Games\\League of Legends\\lockfile");
                thread::sleep(Duration::from_secs(5));
                continue;
            }
            
            let port = port.unwrap();
            let token = token.unwrap();
            println!("[LCU Watcher] Successfully connected to LCU on port: {}", port);
            
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
                        println!("[LCU Watcher] Trying endpoint: {}", url);
                        
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
                                                    println!("[LCU Watcher] Found phase via gameflow-phase: {}", phase);
                                                    break;
                                                }
                                            } else {
                                                // The /session endpoint returns phase as a field
                                                if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
                                                    phase_value = Some(phase.to_string());
                                                    println!("[LCU Watcher] Found phase via session: {}", phase);
                                                    break;
                                                }
                                            }
                                        },
                                        Err(e) => println!("[LCU Watcher] Failed to parse response from {}: {}", endpoint, e),
                                    }
                                } else {
                                    println!("[LCU Watcher] Endpoint {} returned status: {}", endpoint, resp.status());
                                }
                            },
                            Err(e) => println!("[LCU Watcher] Failed to connect to endpoint {}: {}", endpoint, e),
                        }
                    }
                    
                    if !connected {
                        println!("[LCU Watcher] Could not connect to any LCU API endpoint. Retrying in 5 seconds...");
                        thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                    
                    let phase = phase_value.unwrap_or_else(|| "None".to_string());
                    
                    if phase != last_phase {
                        println!("[LCU Watcher] LCU status changed: {} -> {}", last_phase, phase);
                        // emit status event to frontend
                        let _ = app_handle.emit("lcu-status", phase.to_string());
                    }
                    
                    if phase == "ChampSelect" {
                        // Get the current session to check selected champion
                        let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
                        let auth = base64::encode(format!("riot:{}", token));
                        
                        match client.get(&session_url)
                            .header("Authorization", format!("Basic {}", auth))
                            .send() 
                        {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    match resp.json::<serde_json::Value>() {
                                        Ok(json) => {
                                            // Get the local player's cell ID
                                            if let Some(local_player_cell_id) = json.get("localPlayerCellId").and_then(|v| v.as_i64()) {
                                                // First check if the champion is locked in via actions
                                                let mut is_locked_in = false;
                                                let mut selected_champion_id = 0;
                                                
                                                if let Some(actions) = json.get("actions").and_then(|v| v.as_array()) {
                                                    for action_group in actions {
                                                        if let Some(actions) = action_group.as_array() {
                                                            for action in actions {
                                                                if let Some(actor_cell_id) = action.get("actorCellId").and_then(|v| v.as_i64()) {
                                                                    if actor_cell_id == local_player_cell_id {
                                                                        if let Some(completed) = action.get("completed").and_then(|v| v.as_bool()) {
                                                                            if completed {
                                                                                if let Some(champion_id) = action.get("championId").and_then(|v| v.as_i64()) {
                                                                                    if champion_id > 0 {
                                                                                        is_locked_in = true;
                                                                                        selected_champion_id = champion_id;
                                                                                        break;
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
                                                
                                                // Only proceed with injection if the champion is locked in
                                                if is_locked_in {
                                                    println!("[LCU Watcher] Champion {} locked in", selected_champion_id);
                                                    
                                                    // load saved skins from config.json
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
                                                                        println!("[LCU Watcher] Injecting skin for champion {}: skin_id={}", 
                                                                            selected_champion_id, skin.skin_id);
                                                                        
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
                                                                        
                                                                        // Inject the skin
                                                                        match inject_skins_impl(
                                                                            &app_handle,
                                                                            &league_path_clone,
                                                                            &skins,
                                                                            &champions_dir
                                                                        ) {
                                                                            Ok(_) => println!("[LCU Watcher] Successfully injected skin for champion {}", selected_champion_id),
                                                                            Err(e) => println!("[LCU Watcher] Failed to inject skin: {}", e),
                                                                        }
                                                                    } else {
                                                                        println!("[LCU Watcher] No skin configured for champion {}", selected_champion_id);
                                                                    }
                                                                },
                                                                Err(e) => println!("[LCU Watcher] Failed to parse config.json: {}", e),
                                                            }
                                                        },
                                                        Err(e) => println!("[LCU Watcher] Failed to read config.json: {}", e),
                                                    }
                                                }
                                            }
                                        },
                                        Err(e) => println!("[LCU Watcher] Failed to parse session data: {}", e),
                                    }
                                }
                            },
                            Err(e) => println!("[LCU Watcher] Failed to get session data: {}", e),
                        }
                    }
                    last_phase = phase.to_string();
                },
                Err(e) => println!("Failed to build HTTP client: {}", e),
            }
            
            // Sleep for a bit before checking again
            thread::sleep(Duration::from_secs(5));
        }
    });
    
    println!("LCU status watcher thread started");
    Ok(())
}