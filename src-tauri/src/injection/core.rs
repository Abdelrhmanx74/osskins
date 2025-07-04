use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::env;
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;
use zip::ZipArchive;
use memmap2::MmapOptions;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use crate::injection::error::{InjectionError, ModState, Skin};
use crate::injection::file_index::get_global_index;
use crate::injection::fantome::copy_default_overlay;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Main skin injector class - simplified without profiles
pub struct SkinInjector {
    pub(crate) state: ModState,
    pub(crate) app_dir: PathBuf,
    pub(crate) root_path: PathBuf,  // Store the root League directory path
    pub(crate) game_path: PathBuf,  // Store the Game subdirectory path
    pub(crate) status: String,
    pub(crate) log_file: Option<File>,
    pub(crate) mod_tools_path: Option<PathBuf>, // Add mod_tools path
    pub(crate) champion_names: HashMap<u32, String>, // Add cache for champion names
    pub(crate) app_handle: Option<AppHandle>,
}

impl SkinInjector {
    pub fn new(app_handle: &AppHandle, root_path: &str) -> Result<Self, InjectionError> {
        // Get the app directory
        let app_dir = app_handle.path().app_data_dir()
            .map_err(|e| InjectionError::IoError(io::Error::new(io::ErrorKind::NotFound, format!("{}", e))))?;
        
        // Store both root and game paths
        let root_path = PathBuf::from(root_path);
        let game_path = root_path.join("Game");
        
        // Validate game path
        if !game_path.join("League of Legends.exe").exists() {
            return Err(InjectionError::InvalidGamePath("Game\\League of Legends.exe not found".into()));
        }
        
        // Create directories needed
        fs::create_dir_all(app_dir.join("mods"))?;
        fs::create_dir_all(app_dir.join("temp"))?;
        
        // Create log file
        let log_path = app_dir.join("log.txt");
        let log_file = File::create(&log_path)?;

        // Initialize empty champion names cache
        let champion_names = HashMap::new();

        // Look for mod-tools executable in multiple locations
        let mut mod_tools_path = None;
        
        // Check in resource directory and bundled cslol-tools subfolder
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            // Direct resource root
            let direct = resource_dir.join("mod-tools.exe");
            if direct.exists() {
                mod_tools_path = Some(direct.clone());
            }
            // Bundled under cslol-tools folder
            let sub = resource_dir.join("cslol-tools").join("mod-tools.exe");
            if sub.exists() {
                mod_tools_path = Some(sub.clone());
            }
        }
        
        // Check next to the app executable
        if mod_tools_path.is_none() {
            if let Ok(app_local_dir) = app_handle.path().app_local_data_dir() {
                let candidate = app_local_dir.join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate);
                }
            }
        }
        
        // Check in CSLOL directory
        if mod_tools_path.is_none() {
            if let Ok(app_local_dir) = app_handle.path().app_local_data_dir() {
                // Try looking in cslol-tools subdirectory
                let candidate = app_local_dir.join("cslol-tools").join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate);
                }
                
                // Try looking in the original CSLOL Manager directory
                let candidate = app_local_dir.join("..").join("cslol-manager-2024-10-27-401067d-prerelease").join("cslol-tools").join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate.canonicalize().unwrap_or(candidate));
                }
            }
        }
        
        // Fallback: look relative to current executable location
        if mod_tools_path.is_none() {
            if let Ok(exe_path) = env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    // Common bundled structure: cslol-tools/*
                    let cand1 = exe_dir.join("cslol-tools").join("mod-tools.exe");
                    if cand1.exists() { mod_tools_path = Some(cand1.clone()); }
                    // Next to exe in resources folder
                    let cand2 = exe_dir.join("resources").join("cslol-tools").join("mod-tools.exe");
                    if cand2.exists() { mod_tools_path = Some(cand2.clone()); }
                    // Directly in exe directory
                    let cand3 = exe_dir.join("mod-tools.exe");
                    if cand3.exists() { mod_tools_path = Some(cand3.clone()); }
                }
            }
        }
        
        Ok(Self {
            state: ModState::Uninitialized,
            app_dir,
            root_path,
            game_path,
            status: String::new(),
            log_file: Some(log_file),
            mod_tools_path,
            champion_names,
            app_handle: Some(app_handle.clone()),
        })
    }
    
    // Initialize the injector - sets up necessary state
    pub(crate) fn initialize(&mut self) -> Result<(), InjectionError> {
        self.log("Initializing skin injector...");
        
        // Set state to Idle (ready to inject)
        self.set_state(ModState::Idle);
        
        // Load champion names if available
        if let Err(e) = self.load_champion_names() {
            self.log(&format!("Warning: Could not load champion names: {}", e));
        }
        
        self.log("Skin injector initialized successfully");
        Ok(())
    }
    
    // Load champion names from CDN
    fn load_champion_names(&mut self) -> Result<(), InjectionError> {
        // Moved to champion_data.rs module
        // This is a placeholder - actual implementation is in the champion_data module
        Ok(())
    }
    
    pub(crate) fn log(&mut self, message: &str) {
        // Add emoji based on message content
        let emoji_message = if message.contains("Initializing") {
            format!("🔄 {}", message)
        } else if message.contains("State changed to") {
            if message.contains("Busy") {
                format!("⏳ {}", message)
            } else if message.contains("Idle") {
                format!("💤 {}", message)
            } else if message.contains("Running") {
                format!("▶️ {}", message)
            } else {
                format!("ℹ️ {}", message)
            }
        } else if message.contains("Starting skin injection") {
            format!("🚀 {}", message)
        } else if message.contains("Stopping skin injection") {
            format!("🛑 {}", message)
        } else if message.contains("Skin injection stopped") {
            format!("⏹️ {}", message)
        } else if message.contains("Cleaning up") {
            format!("🧹 {}", message)
        } else if message.contains("Processing skin") {
            format!("⚙️ {}", message)
        } else if message.contains("Found fantome file") {
            format!("📂 {}", message)
        } else if message.contains("Processing fantome file") {
            format!("📦 {}", message)
        } else if message.contains("Extracting fantome file") {
            format!("📤 {}", message)
        } else if message.contains("Creating mod") {
            format!("🛠️ {}", message)
        } else if message.contains("valid") {
            format!("✅ {}", message)
        } else if message.contains("Copying mod") {
            format!("📋 {}", message)
        } else if message.contains("already has EnableMods=1") {
            format!("✨ {}", message)
        } else if message.contains("Using mod-tools") {
            format!("🔧 {}", message)
        } else if message.contains("overlay") && !message.contains("failed") {
            format!("🔮 {}", message)
        } else if message.contains("succeeded") || message.contains("successfully") {
            format!("✅ {}", message)
        } else if message.contains("failed") || message.contains("error") || message.contains("Error") {
            format!("❌ {}", message)
        } else {
            format!("ℹ️ {}", message)
        };

        // Write to log file and print
        if let Some(log_file) = &mut self.log_file {
            let _ = writeln!(log_file, "{}", emoji_message);
            let _ = log_file.flush();
        }
        println!("{}", emoji_message);
        
        // Emit the log to the frontend
        if let Some(app) = &self.app_handle {
            let _ = app.emit("terminal-log", &emoji_message);
        }
        
        self.status = message.to_string();
    }
    
    pub(crate) fn set_state(&mut self, new_state: ModState) {
        if self.state != new_state {
            self.state = new_state;
            self.log(&format!("State changed to: {:?}", new_state));
        }
    }

    // Replace the hardcoded get_champion_name with a function that uses JSON data
    pub(crate) fn get_champion_name(&mut self, champion_id: u32) -> Option<String> {
        // Check cache first
        if let Some(name) = self.champion_names.get(&champion_id) {
            return Some(name.clone());
        }

        // If not in cache, look up in the champions directory
        let champions_dir = self.app_dir.join("champions");
        if !champions_dir.exists() {
            return None;
        }

        // Look through all champion directories
        if let Ok(entries) = fs::read_dir(&champions_dir) {
            for entry in entries.filter_map(Result::ok) {
                if !entry.path().is_dir() {
                    continue;
                }

                let champion_file = entry.path().join(format!("{}.json", 
                    entry.file_name().to_string_lossy()));

                if let Ok(content) = fs::read_to_string(&champion_file) {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Check if this JSON contains the champion ID we're looking for
                        if let Some(id) = data.get("id").and_then(|v| v.as_u64()) {
                            if id as u32 == champion_id {
                                // Found the champion, get their name
                                if let Some(name) = data.get("name")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_lowercase().replace(" ", "")) 
                                {
                                    // Cache it for future lookups
                                    self.champion_names.insert(champion_id, name.clone());
                                    return Some(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
    
    // Functions split across modular files:
    // - fantome.rs: extract_fantome, extract_fantome_mmap, find_fantome_for_skin, create_mod_from_extracted, process_fantome_file
    // - mod_tools.rs: copy_mod_to_game, run_overlay
    // - game_config.rs: enable_mods_in_game_cfg
    
    pub fn inject_skins(&mut self, skins: &[Skin], fantome_files_dir: &Path) -> Result<(), InjectionError> {
        // Emit start event to frontend
        if let Some(app) = &self.app_handle {
            let _ = app.emit("injection-status", "injecting");
        }

        // First, ensure that we clean up any existing running processes
        // We do this even if we're not in Running state to avoid issues with orphaned processes
        self.cleanup()?;
        
        // Now we can properly initialize for a new injection
        self.set_state(ModState::Busy);
        self.log("Starting skin injection process...");
        
        // First, clean up the game's mods directory
        let game_mods_dir = self.game_path.join("mods");
        if game_mods_dir.exists() {
            self.log("Cleaning up existing mods in game directory");
            fs::remove_dir_all(&game_mods_dir)?;
        }
        fs::create_dir_all(&game_mods_dir)?;
        
        // Process each skin
        for (i, skin) in skins.iter().enumerate() {
            self.log(&format!("Processing skin {}/{}: champion_id={}, skin_id={}, chroma_id={:?}", 
                i + 1, skins.len(), skin.champion_id, skin.skin_id, skin.chroma_id));
                
            // Find the fantome file
            let fantome_path = self.find_fantome_for_skin(skin, fantome_files_dir)?;
            if let Some(fantome_path) = fantome_path {
                self.log(&format!("Found fantome file: {}", fantome_path.display()));
                
                // Process the fantome file to create a proper mod structure
                let mod_dir = self.process_fantome_file(&fantome_path)?;
                
                // Copy the processed mod to the game
                if self.is_valid_mod_dir(&mod_dir) {
                    self.log("Mod structure is valid, copying to game directory");
                    self.copy_mod_to_game(&mod_dir)?;
                } else {
                    // If processing failed, fall back to direct copy
                    self.log("WARNING: Processing failed, falling back to direct copy");
                    let file_name = match fantome_path.file_name() {
                        Some(f) => f,
                        None => {
                            self.log("ERROR: Could not get file name for fallback copy");
                            continue;
                        }
                    };
                    let game_fantome_path = game_mods_dir.join(file_name);
                    if let Err(e) = fs::copy(&fantome_path, &game_fantome_path) {
                        self.log(&format!("ERROR: Failed to copy fallback fantome file: {}", e));
                    }
                }
            } else {
                let msg = format!(
                    "No fantome file found for skin: champion_id={}, skin_id={}, chroma_id={:?}",
                    skin.champion_id, skin.skin_id, skin.chroma_id
                );
                self.log(&format!("ERROR: {}", msg));
                self.set_state(ModState::Idle);
                return Err(InjectionError::MissingFantomeFile(msg));
            }
        }
        
        // Enable mods in Game.cfg
        self.enable_mods_in_game_cfg()?;
        
        // Get the list of mod names we've installed
        let mut mod_names = Vec::new();
        for entry in fs::read_dir(&game_mods_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("META").join("info.json").exists() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        mod_names.push(name_str.to_string());
                    } else {
                        self.log("WARNING: Could not convert mod directory name to string");
                    }
                } else {
                    self.log("WARNING: Could not get mod directory file name");
                }
            }
        }
        
        // Start the overlay process - THIS is the key part that makes skins actually show in-game!
        if let Err(e) = self.run_overlay() {
            self.log(&format!("WARNING: Failed to start overlay process: {}. Trying fallback injection method...", e));
            
            // Try our fallback direct WAD injection method
            match self.inject_skin_direct_fallback(&mod_names) {
                Ok(_) => {
                    self.log("✅ Fallback injection successful! Skins will work but the game may show integrity warnings.");
                    
                    // Set state to running so the cleanup function will be called later
                    self.set_state(ModState::Running);
                    
                    // All steps complete successfully with fallback, emit success event
                    if let Some(app) = &self.app_handle {
                        let _ = app.emit("injection-status", "completed");
                    }
                    
                    return Ok(());
                },
                Err(fallback_err) => {
                    // Both methods failed, return the original error
                    self.log(&format!("❌ Fallback injection also failed: {}", fallback_err));
                    self.set_state(ModState::Idle);
                    return Err(InjectionError::ProcessError(format!(
                        "Both overlay and fallback injection methods failed. Original error: {}",
                        e
                    )));
                }
            }
        }
        
        self.log("Skin injection completed successfully");
        // Note: We don't set state to Idle because we're now in Running state with the overlay active
        // After all steps complete successfully, emit end event
        if let Some(app) = &self.app_handle {
            let _ = app.emit("injection-status", "completed");
        }
        Ok(())
    }

    // Add a cleanup method to stop the injection
    pub fn cleanup(&mut self) -> Result<(), InjectionError> {
        self.log("Stopping skin injection process...");
        
        // Find and kill the mod-tools processes - more aggressive approach
        #[cfg(target_os = "windows")]
        {
            // First try normal taskkill
            let mut command = std::process::Command::new("taskkill");
            command.args(["/F", "/IM", "mod-tools.exe"]);
            
            #[cfg(target_os = "windows")]
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            #[cfg(target_os = "windows")]
            command.creation_flags(CREATE_NO_WINDOW);
            
            let _ = command.output();
            
            // Then check if any processes are still running with wmic (more reliable)
            let mut check_command = std::process::Command::new("wmic");
            check_command.args(["process", "where", "name='mod-tools.exe'", "get", "processid"]);
            #[cfg(target_os = "windows")]
            check_command.creation_flags(CREATE_NO_WINDOW);
            
            // If we find any processes still running, use taskkill with /PID for each one
            if let Ok(output) = check_command.output() {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        let line = line.trim();
                        if line != "ProcessId" && !line.is_empty() && line.chars().all(|c| c.is_digit(10)) {
                            // Found a PID, kill it specifically
                            let mut kill_pid = std::process::Command::new("taskkill");
                            kill_pid.args(["/F", "/PID", line]);
                            #[cfg(target_os = "windows")]
                            kill_pid.creation_flags(CREATE_NO_WINDOW);
                            let _ = kill_pid.output();
                        }
                    }
                }
            }
        }
        
        // Clean up the overlay directory
        let overlay_dir = self.app_dir.join("overlay");
        if overlay_dir.exists() {
            // Try multiple times if needed - sometimes Windows file locks take time to release
            for _ in 0..3 {
                match fs::remove_dir_all(&overlay_dir) {
                    Ok(_) => break,
                    Err(_) => {
                        // Sleep briefly to allow file locks to clear
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
        
        // Reset the state regardless of previous state to ensure cleanup
        self.set_state(ModState::Idle);
        self.log("Skin injection stopped");
        
        Ok(())
    }

    // Cleanup mod-tools processes specifically
    pub(crate) fn cleanup_mod_tools_processes(&mut self) -> Result<(), InjectionError> {
        self.log("Cleaning up mod-tools processes...");
        
        #[cfg(target_os = "windows")]
        {
            // Kill any running mod-tools processes
            let mut command = std::process::Command::new("taskkill");
            command.args(["/F", "/IM", "mod-tools.exe"]);
            command.creation_flags(CREATE_NO_WINDOW);
            
            match command.output() {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if !stderr.contains("not found") && !stderr.contains("No tasks are running") {
                            self.log(&format!("Warning: Failed to kill mod-tools processes: {}", stderr));
                        }
                    } else {
                        self.log("Successfully terminated mod-tools processes");
                    }
                }
                Err(e) => {
                    self.log(&format!("Warning: Could not run taskkill: {}", e));
                }
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // For non-Windows systems, use pkill
            let mut command = std::process::Command::new("pkill");
            command.args(["-f", "mod-tools"]);
            let _ = command.output();
        }
        
        Ok(())
    }

    // Add this method above the new method or other impl methods
    
    // Initialize cache for faster first-time injection
    // Direct WAD injection fallback for when mod-tools overlay fails
    fn inject_skin_direct_fallback(&mut self, mod_names: &[String]) -> Result<(), InjectionError> {
        self.log("🔄 Attempting direct WAD injection fallback...");
        
        // Check if there are any valid mods
        if mod_names.is_empty() {
            self.log("❌ No valid mods found for direct injection");
            return Err(InjectionError::ProcessError("No valid mods to inject".into()));
        }
        
        // First, ensure game directory is accessible
        let game_data_dir = self.game_path.join("DATA");
        if !game_data_dir.exists() {
            self.log("❌ Game DATA directory not found");
            return Err(InjectionError::ProcessError("Game DATA directory not found".into()));
        }
        
        // Create temporary directory for WAD files
        let temp_wad_dir = self.app_dir.join("temp_wad");
        if temp_wad_dir.exists() {
            fs::remove_dir_all(&temp_wad_dir)?;
        }
        fs::create_dir_all(&temp_wad_dir)?;
        
        // Get paths to all mods
        let game_mods_dir = self.game_path.join("mods");
        let mut success_count = 0;
        
        // Process each mod
        for mod_name in mod_names {
            let mod_dir = game_mods_dir.join(mod_name);
            if !mod_dir.exists() {
                continue;
            }
            
            // Check if mod has WAD directory
            let wad_dir = mod_dir.join("WAD");
            if !wad_dir.exists() {
                continue;
            }
            
            // Find all WAD files - use a reference to wad_dir here to avoid moving it
            for entry in WalkDir::new(&wad_dir) {
                let entry = entry?;
                let path = entry.path();
                
                // Only process WAD files
                if path.is_file() && 
                   (path.extension().and_then(|ext| ext.to_str()) == Some("wad") ||
                    path.to_string_lossy().ends_with(".wad.client")) {
                    
                    // Get relative path from WAD directory
                    let rel_path = path.strip_prefix(&wad_dir)
                        .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
                    
                    // Target path in game's DATA directory
                    let target_path = game_data_dir.join(rel_path);
                    
                    // Create parent directories if needed
                    if let Some(parent) = target_path.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent)?;
                        }
                    }
                    
                    // Copy WAD file directly
                    let mut options = fs::OpenOptions::new();
                    options.write(true).create(true);
                    
                    if target_path.exists() {
                        // If file exists, back it up first if we haven't already 
                        let backup_path = temp_wad_dir.join(rel_path);
                        if let Some(parent) = backup_path.parent() {
                            if !parent.exists() {
                                fs::create_dir_all(parent)?;
                            }
                        }
                        
                        // Only back up if we haven't already
                        if !backup_path.exists() {
                            if let Err(e) = fs::copy(&target_path, &backup_path) {
                                self.log(&format!("⚠️ Couldn't back up WAD file: {}", e));
                            } else {
                                self.log(&format!("💾 Backed up original WAD: {}", rel_path.display()));
                            }
                        }
                    }
                    
                    // Now copy the modded WAD
                    if let Err(e) = fs::copy(path, &target_path) {
                        self.log(&format!("❌ Failed to copy modded WAD: {}", e));
                    } else {
                        self.log(&format!("✅ Directly injected WAD: {}", rel_path.display()));
                        success_count += 1;
                    }
                }
            }
        }
        
        // Check if we were able to inject any WAD files
        if success_count > 0 {
            self.log(&format!("✅ Successfully injected {} WAD files directly", success_count));
            
            // Create a file to track our injections for cleanup later
            let tracking_file = self.app_dir.join("direct_injection.json");
            let tracking_data = serde_json::json!({
                "mods": mod_names,
                "timestamp": chrono::Local::now().timestamp(),
                "backup_dir": temp_wad_dir.to_string_lossy().to_string()
            });
            
            if let Err(e) = fs::write(&tracking_file, 
                                    serde_json::to_string_pretty(&tracking_data).unwrap_or_default()) {
                self.log(&format!("⚠️ Failed to write tracking file: {}", e));
            }
            
            self.log("⚠️ Using fallback injection method - skins should work but league may show integrity warning");
            
            return Ok(());
        } else {
            self.log("❌ No WAD files were found to inject");
            return Err(InjectionError::ProcessError("No WAD files found to inject".into()));
        }
    }
}

// Main wrapper function that is called from commands.rs
pub fn inject_skins(
    app_handle: &AppHandle, 
    game_path: &str, 
    skins: &[Skin], 
    fantome_files_dir: &Path
) -> Result<(), String> {
    // Create injector
    let mut injector = SkinInjector::new(app_handle, game_path)
        .map_err(|e| format!("Failed to create injector: {}", e))?;
    
    // Initialize
    injector.initialize()
        .map_err(|e| format!("Failed to initialize: {}", e))?;
    
    // Inject skins
    injector.inject_skins(skins, fantome_files_dir)
        .map_err(|e| format!("Failed to inject skins: {}", e))
}

// New function to clean up the injection when needed
pub fn cleanup_injection(
    app_handle: &AppHandle,
    game_path: &str
) -> Result<(), String> {
    // Create injector
    let mut injector = SkinInjector::new(app_handle, game_path)
        .map_err(|e| format!("Failed to create injector: {}", e))?;
    
    // Call cleanup
    injector.cleanup()
        .map_err(|e| format!("Failed to stop skin injection: {}", e))
}