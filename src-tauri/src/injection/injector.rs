use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use crate::injection::error::InjectionError;
use crate::injection::types::{Skin, ModState};
use tauri::{AppHandle, Manager, Emitter};
use walkdir::WalkDir;
use zip::ZipArchive;
use std::env;
use memmap2::MmapOptions;
use std::time::Instant;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// Define Windows-specific constants at the module level
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Main skin injector class - simplified without profiles
pub struct SkinInjector {
    state: ModState,
    app_dir: PathBuf,
    #[allow(dead_code)]
    root_path: PathBuf,  // Store the root League directory path
    game_path: PathBuf,  // Store the Game subdirectory path
    status: String,
    log_file: Option<File>,
    mod_tools_path: Option<PathBuf>, // Add mod_tools path
    champion_names: std::collections::HashMap<u32, String>, // Add cache for champion names
    app_handle: Option<AppHandle>,
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
        let champion_names = std::collections::HashMap::new();

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
    
    fn log(&mut self, message: &str) {
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
        if let Some(_app) = &self.app_handle {
            println!("[injection] {}", emoji_message);
        }
        
        self.status = message.to_string();
    }
    
    fn set_state(&mut self, new_state: ModState) {
        if self.state != new_state {
            self.state = new_state;
            self.log(&format!("State changed to: {:?}", new_state));
        }
    }

    pub fn initialize(&mut self) -> Result<(), InjectionError> {
        if self.state != ModState::Uninitialized {
            return Ok(());
        }
        
        self.set_state(ModState::Busy);
        self.log("Initializing...");
        
        // Create required directories
        fs::create_dir_all(self.app_dir.join("mods"))?;
        fs::create_dir_all(self.app_dir.join("temp"))?;
        
        // Set to idle state when done
        self.set_state(ModState::Idle);
        Ok(())
    }

    // Replace the hardcoded get_champion_name with a function that uses JSON data
    fn get_champion_name(&mut self, champion_id: u32) -> Option<String> {
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
    
    // Extract .fantome file (similar to utility::unzip in CSLOL Manager)
    fn extract_fantome(&mut self, fantome_path: &Path, output_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Extracting fantome file: {}", fantome_path.display()));
        
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Open and extract the zip file
        let file = fs::File::open(fantome_path)?;
        let mut archive = ZipArchive::new(file)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir.join(path),
                None => continue,
            };
            
            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }
        
        Ok(())
    }

    // Add this memory-optimized extraction function
    fn extract_fantome_mmap(&mut self, fantome_path: &Path, output_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Extracting fantome file with memory mapping: {}", fantome_path.display()));
        
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Open the file for memory mapping
        let file = fs::File::open(fantome_path)?;
        let file_size = file.metadata()?.len();
        
        // Only use memory mapping for larger files (>1MB)
        if file_size > 1_048_576 {
            // Use memory mapping for better performance with large files
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            
            // Use the memory-mapped data to create a zip archive
            let mut archive = ZipArchive::new(std::io::Cursor::new(&mmap[..]))?;
            
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => output_dir.join(path),
                    None => continue,
                };
                
                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath)?;
                    io::copy(&mut file, &mut outfile)?;
                }
            }
            
            // Memory-mapped file is automatically unmapped when dropped
            self.log("Memory-mapped extraction completed successfully");
        } else {
            // For smaller files, use the standard approach
            let mut archive = ZipArchive::new(file)?;
            
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => output_dir.join(path),
                    None => continue,
                };
                
                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = fs::File::create(&outpath)?;
                    io::copy(&mut file, &mut outfile)?;
                }
            }
        }
        
        Ok(())
    }
    
    // Check if directory contains META/info.json to confirm it's a valid mod
    fn is_valid_mod_dir(&self, dir_path: &Path) -> bool {
        dir_path.join("META").join("info.json").exists()
    }
    
    // Find appropriate .fantome file for a skin
    fn find_fantome_for_skin(&mut self, skin: &Skin, fantome_files_dir: &Path) -> Result<Option<PathBuf>, InjectionError> {
        // Fall back to the original slow method if index lookup failed
        self.log("Using fallback file search method");
        
        // First try direct path from JSON
        if let Some(fantome_path) = &skin.fantome_path {
            self.log(&format!("Using fantome path from JSON: {}", fantome_path));
            
            // Try direct path
            let direct_path = fantome_files_dir.join(fantome_path);
            if direct_path.exists() {
                self.log(&format!("Found exact file at path: {}", direct_path.display()));
                return Ok(Some(direct_path));
            }
            
            // Try path using champion name
            if let Some(champion_name) = self.get_champion_name(skin.champion_id) {
                let champ_path = fantome_files_dir.join(champion_name).join(fantome_path.split('/').last().unwrap_or(""));
                if champ_path.exists() {
                    self.log(&format!("Found file at champion path: {}", champ_path.display()));
                    return Ok(Some(champ_path));
                }
            }
            
            // Search for matching filename
            let file_name = fantome_path.split('/').last().unwrap_or("");
            for entry in WalkDir::new(fantome_files_dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if path.file_name()
                       .map(|name| name.to_string_lossy() == file_name)
                       .unwrap_or(false) {
                        self.log(&format!("Found matching file: {}", path.display()));
                        return Ok(Some(path.to_path_buf()));
                    }
                }
            }
        }
        
        // Fall back to searching by ID
        self.log(&format!("Searching for skin with champion_id={}, skin_id={}, chroma_id={:?}", 
            skin.champion_id, skin.skin_id, skin.chroma_id));
            
        let skin_id_str = skin.skin_id.to_string();
        
        // Try champion directory first
        if let Some(champion_name) = self.get_champion_name(skin.champion_id) {
            let champ_dir = fantome_files_dir.join(champion_name);
            if champ_dir.exists() {
                for entry in fs::read_dir(champ_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("fantome") {
                        continue;
                    }
                    
                    let file_name = path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                        
                    if file_name.contains(&skin_id_str) {
                        // Check for chroma
                        if let Some(chroma_id) = skin.chroma_id {
                            if file_name.contains("chroma") && file_name.contains(&chroma_id.to_string()) {
                                self.log(&format!("Found chroma match: {}", path.display()));
                                return Ok(Some(path.to_path_buf()));
                            }
                        } else if !file_name.contains("chroma") {
                            self.log(&format!("Found non-chroma match: {}", path.display()));
                            return Ok(Some(path.to_path_buf()));
                        }
                    }
                }
            }
        }
        
        // Search all files as last resort
        for entry in WalkDir::new(fantome_files_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("fantome") {
                    continue;
                }
                
                let file_name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                    
                if file_name.contains(&skin_id_str) {
                    // Check for chroma
                    if let Some(chroma_id) = skin.chroma_id {
                        if file_name.contains("chroma") && file_name.contains(&chroma_id.to_string()) {
                            self.log(&format!("Found chroma match in full search: {}", path.display()));
                            return Ok(Some(path.to_path_buf()));
                        }
                    } else if !file_name.contains("chroma") {
                        self.log(&format!("Found non-chroma match in full search: {}", path.display()));
                        return Ok(Some(path.to_path_buf()));
                    }
                }
            }
        }
        
        self.log(&format!("No fantome file found for skin: champion_id={}, skin_id={}, chroma_id={:?}",
            skin.champion_id, skin.skin_id, skin.chroma_id));
        self.set_state(ModState::Idle);
        // Emit error to frontend
        if let Some(_app) = &self.app_handle {
            let error_msg = format!("No fantome file found for skin: champion_id={}, skin_id={}, chroma_id={:?}", skin.champion_id, skin.skin_id, skin.chroma_id);
            let _ = _app.emit("injection-status", false);
            let _ = _app.emit("skin-injection-error", error_msg);
        }
        Ok(None)
    }
    
    // Create a mod directory structure from extracted fantome files
    fn create_mod_from_extracted(&mut self, extract_dir: &Path, mod_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Creating mod from extracted files at: {}", extract_dir.display()));
        
        // Create mod directories
        fs::create_dir_all(mod_dir.join("META"))?;
        fs::create_dir_all(mod_dir.join("WAD"))?;
        
        // Check if there's already a META/info.json in the extracted content
        let extracted_info_json = extract_dir.join("META").join("info.json");
        let mod_info_json = mod_dir.join("META").join("info.json");
        
        if extracted_info_json.exists() {
            // Copy the existing info.json
            fs::copy(&extracted_info_json, &mod_info_json)?;
        } else {
            // Create a basic info.json
            let info_json = format!(r#"{{
                "Name": "ExtractedMod",
                "Version": "1.0.0",
                "Author": "osskins",
                "Description": "Extracted from fantome file at {}"
            }}"#, chrono::Local::now().to_rfc3339());
            
            fs::write(&mod_info_json, info_json)?;
        }
        
        // Look for WAD directory in extracted content
        let extracted_wad_dir = extract_dir.join("WAD");
        if extracted_wad_dir.exists() {
            // Copy WAD files
            for entry in WalkDir::new(&extracted_wad_dir) {
                let entry = entry?;
                let path = entry.path();
                let rel_path = path.strip_prefix(&extracted_wad_dir)
                    .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
                
                let target_path = mod_dir.join("WAD").join(rel_path);
                
                if path.is_dir() {
                    fs::create_dir_all(&target_path)?;
                } else if path.is_file() {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(path, &target_path)?;
                }
            }
        } else {
            // If no WAD directory, look for WAD files in the root
            for entry in WalkDir::new(extract_dir) {
                let entry = entry?;
                let path = entry.path();
                
                // Skip META directory
                if path.starts_with(extract_dir.join("META")) {
                    continue;
                }
                
                // Check if this is a WAD file
                if path.is_file() && 
                   (path.extension().and_then(|ext| ext.to_str()) == Some("wad") ||
                    path.to_string_lossy().ends_with(".wad.client")) {
                    
                    let file_name = path.file_name().unwrap();
                    let target_path = mod_dir.join("WAD").join(file_name);
                    
                    fs::copy(path, &target_path)?;
                }
            }
        }
        
        Ok(())
    }
    
    // Process .fantome files to create proper mod structure with memory optimization
    fn process_fantome_file(&mut self, fantome_path: &Path) -> Result<PathBuf, InjectionError> {
        self.log(&format!("Processing fantome file: {}", fantome_path.display()));
        
        // Create temp extraction directory
        let file_stem = fantome_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let extract_dir = self.app_dir.join("temp").join(&file_stem);
        let mod_dir = self.app_dir.join("mods").join(&file_stem);
        
        // Clean up any existing directories
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        if mod_dir.exists() {
            fs::remove_dir_all(&mod_dir)?;
        }
        
        // Check file size to decide which extraction method to use
        let file_size = match fs::metadata(fantome_path) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0, // Default to standard extraction if we can't get size
        };
        
        // Use memory-mapped extraction for larger files
        if file_size > 1_048_576 { // >1MB
            self.extract_fantome_mmap(fantome_path, &extract_dir)?;
        } else {
            // Use standard extraction for smaller files
            self.extract_fantome(fantome_path, &extract_dir)?;
        }
        
        // Create mod structure
        self.create_mod_from_extracted(&extract_dir, &mod_dir)?;
        
        // Clean up extraction directory
        fs::remove_dir_all(&extract_dir)?;
        
        Ok(mod_dir)
    }
    
    // Enable mods in Game.cfg
    fn enable_mods_in_game_cfg(&mut self) -> Result<(), InjectionError> {
        let game_cfg_path = self.game_path.join("Game.cfg");
        
        // If file doesn't exist, create it with EnableMods=1
        if !game_cfg_path.exists() {
            fs::write(game_cfg_path, "[General]\nEnableMods=1\n")?;
            self.log("Created Game.cfg with EnableMods=1");
            return Ok(());
        }
        
        // Otherwise, read and modify the file
        let content = fs::read_to_string(&game_cfg_path)?;
        
        // Check if EnableMods is already set correctly
        if content.contains("EnableMods=1") {
            self.log("Game.cfg already has EnableMods=1");
            return Ok(());
        }
        
        // Replace EnableMods=0 with EnableMods=1 if it exists
        let mut new_content = content.clone();
        if content.contains("EnableMods=0") {
            new_content = content.replace("EnableMods=0", "EnableMods=1");
        } else {
            // Add EnableMods=1 to the [General] section if it exists
            if content.contains("[General]") {
                let parts: Vec<&str> = content.split("[General]").collect();
                if parts.len() >= 2 {
                    // Fix the temporary value borrowed error
                    let new_part = format!("\nEnableMods=1{}", parts[1]);
                    new_content = format!("{}[General]{}", parts[0], new_part);
                }
            } else {
                // If no [General] section, add it
                new_content = format!("{}\n[General]\nEnableMods=1\n", content);
            }
        }
        
        // Write the updated content
        fs::write(game_cfg_path, new_content)?;
        self.log("Updated Game.cfg to enable mods");
        
        Ok(())
    }
    
    // Copy a processed mod directory to the game's mods directory
    fn copy_mod_to_game(&mut self, mod_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Copying mod to game directory: {}", mod_dir.display()));

        // Use the mod directory name as the subfolder
        let mod_name = mod_dir.file_name().unwrap();
        let game_mod_dir = self.game_path.join("mods").join(mod_name);

        // Remove any existing mod with the same name
        if game_mod_dir.exists() {
            fs::remove_dir_all(&game_mod_dir)?;
        }
        fs::create_dir_all(&game_mod_dir)?;

        // Copy everything from mod_dir into game_mod_dir
        for entry in WalkDir::new(mod_dir) {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path.strip_prefix(mod_dir)
                .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
            let target_path = game_mod_dir.join(rel_path);

            if path.is_dir() {
                fs::create_dir_all(&target_path)?;
            } else if path.is_file() {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &target_path)?;
            }
        }
        Ok(())
    }
    
    // Run the overlay process using mod-tools.exe
    fn run_overlay(&mut self) -> Result<(), InjectionError> {
        let mod_tools_path = match &self.mod_tools_path {
            Some(path) => {
                if !path.exists() {
                    return Err(InjectionError::OverlayError(format!(
                        "mod-tools.exe was found during initialization but is no longer at path: {}. Please reinstall the application or obtain mod-tools.exe from CSLOL Manager.",
                        path.display()
                    )));
                }
                path.clone()
            },
            None => return Err(InjectionError::OverlayError(
                "mod-tools.exe not found. Please install CSLOL Manager or copy mod-tools.exe to the application directory.".into()
            )),
        };

        self.log(&format!("Using mod-tools.exe from: {}", mod_tools_path.display()));

        // First, ensure no mod-tools processes are running before we start
        self.cleanup_mod_tools_processes();
        
        // Set up directory paths
        let game_mods_dir = self.game_path.join("mods");
        let overlay_dir = self.app_dir.join("overlay");
        let temp_overlay_dir = self.app_dir.join("temp_overlay");
        
        // Clean up the final overlay directory
        if overlay_dir.exists() {
            let mut attempts = 0;
            let max_attempts = 3;
            while attempts < max_attempts {
                match fs::remove_dir_all(&overlay_dir) {
                    Ok(_) => break,
                    Err(e) => {
                        self.log(&format!("Failed to remove overlay directory (attempt {}/{}): {}", 
                            attempts + 1, max_attempts, e));
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        attempts += 1;
                    }
                }
            }
            if overlay_dir.exists() && attempts >= max_attempts {
                return Err(InjectionError::OverlayError(
                    "Cannot remove existing overlay directory. It may be locked by another process.".into()
                ));
            }
        }
        
        // Create the final overlay directory
        fs::create_dir_all(&overlay_dir)?;
        
        // Clean up and recreate the temp overlay directory
        if temp_overlay_dir.exists() {
            match fs::remove_dir_all(&temp_overlay_dir) {
                Ok(_) => {},
                Err(e) => {
                    self.log(&format!("Could not clean temp overlay directory: {}", e));
                    // Try to continue anyway
                }
            }
        }
        fs::create_dir_all(&temp_overlay_dir)?;
        
        // Get list of mod names (just the directory names, no paths)
        let mut mod_names = Vec::new();
        for entry in fs::read_dir(&game_mods_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("META").join("info.json").exists() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        mod_names.push(name_str.to_string());
                    }
                }
            }
        }
        
        // Log the status
        if mod_names.is_empty() {
            self.log("No valid mods found in game directory");
        } else {
            self.log(&format!("Found {} mods to include in overlay", mod_names.len()));
        }
        
        // Join mod names with / as CSLOL expects
        let mods_arg = mod_names.join("/");
        
        // Create mod overlay - use more retries and a temporary directory
        self.log("Creating mod overlay...");
        let max_retries = 5; // Increased from 3 to 5
        let mut retry_count = 0;
        let mut last_error = None;
        
        while retry_count < max_retries {
            // Add delays and cleanup between retries
            if retry_count > 0 {
                self.log(&format!("Retrying overlay creation (attempt {}/{})", retry_count + 1, max_retries));
                std::thread::sleep(std::time::Duration::from_millis(1000));
                self.cleanup_mod_tools_processes();
                
                // For additional retries, recreate the temp overlay directory to ensure it's clean
                if temp_overlay_dir.exists() {
                    match fs::remove_dir_all(&temp_overlay_dir) {
                        Ok(_) => fs::create_dir_all(&temp_overlay_dir)?,
                        Err(e) => {
                            self.log(&format!("Could not clean temp overlay directory: {}", e));
                            // Try to continue anyway
                        }
                    }
                } else {
                    fs::create_dir_all(&temp_overlay_dir)?;
                }
                
                // Run garbage collection to help free memory
                #[cfg(target_os = "windows")]
                {
                    use std::process::Command;
                    // Run a quick garbage collection via PowerShell to help free memory
                    if retry_count > 0 {
                        self.log("Running garbage collection to free memory...");
                        let mut gc_cmd = Command::new("powershell");
                        gc_cmd.args(["-Command", "[System.GC]::Collect()"]);
                        #[cfg(target_os = "windows")]
                        gc_cmd.creation_flags(CREATE_NO_WINDOW);
                        let _ = gc_cmd.output(); // Ignore output
                    }
                }
            }
            
            // Create the mkoverlay command - using the TEMP directory
            let mut command = std::process::Command::new(&mod_tools_path);
            command.args([
                "mkoverlay",
                game_mods_dir.to_str().unwrap(),
                temp_overlay_dir.to_str().unwrap(),  // Use temp directory instead of final directory
                &format!("--game:{}", self.game_path.to_str().unwrap()),
                &format!("--mods:{}", mods_arg),
                "--noTFT",
                "--ignoreConflict"
            ]);
            
            #[cfg(target_os = "windows")]
            command.creation_flags(CREATE_NO_WINDOW);
            
            if retry_count == 0 {
                self.log("Running mkoverlay command...");
            }
            
            // Execute the command
            match command.output() {
                Ok(output) => {
                    if output.status.success() {
                        // Success - copy from temp directory to final overlay directory
                        self.log("Overlay creation succeeded!");
                        
                        // Copy from temp_overlay_dir to overlay_dir
                        for entry in WalkDir::new(&temp_overlay_dir) {
                            let entry = entry?;
                            let path = entry.path();
                            let rel_path = path.strip_prefix(&temp_overlay_dir)
                                .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
                            let target_path = overlay_dir.join(rel_path);
                            
                            if path.is_dir() {
                                fs::create_dir_all(&target_path)?;
                            } else if path.is_file() {
                                if let Some(parent) = target_path.parent() {
                                    fs::create_dir_all(parent)?;
                                }
                                fs::copy(path, &target_path)?;
                            }
                        }
                        
                        // Clean up temp directory
                        let _ = fs::remove_dir_all(&temp_overlay_dir);
                        
                        break;
                    } else {
                        // Command ran but had error status
                        let stderr_output = String::from_utf8_lossy(&output.stderr).into_owned();
                        let stdout_output = String::from_utf8_lossy(&output.stdout).into_owned();
                        let error_message = if stderr_output.is_empty() { stdout_output } else { stderr_output };
                        
                        // Check if it's an access violation error
                        if output.status.to_string().contains("0xc0000005") {
                            self.log(&format!("Access violation error in attempt {}/{}. Retrying...", 
                                retry_count + 1, max_retries));
                            last_error = Some(InjectionError::ProcessError(format!(
                                "mkoverlay command failed: {}. Exit code: {}", 
                                error_message, output.status
                            )));
                            retry_count += 1;
                            continue;
                        } else {
                            // Other error, only show error log if this is the final attempt
                            if retry_count + 1 >= max_retries {
                                self.log(&format!("Overlay creation failed: {}", error_message));
                            }
                            retry_count += 1;
                            
                            last_error = Some(InjectionError::ProcessError(format!(
                                "mkoverlay command failed: {}. Exit code: {}", 
                                error_message, output.status
                            )));
                            
                            // Try again if we haven't exhausted retries
                            if retry_count < max_retries {
                                continue;
                            }
                            
                            // Clean up temp directory
                            let _ = fs::remove_dir_all(&temp_overlay_dir);
                            
                            return Err(last_error.unwrap());
                        }
                    }
                },
                Err(e) => {
                    // Command couldn't be started
                    return Err(InjectionError::ProcessError(format!(
                        "Failed to create overlay: {}. The mod-tools.exe might be missing or incompatible.", e
                    )));
                }
            }
        }
        
        // Check if all retries were exhausted
        if retry_count >= max_retries {
            if let Some(err) = last_error {
                return Err(err);
            }
            return Err(InjectionError::ProcessError("Failed to create overlay after multiple attempts".into()));
        }
        // Create config.json
        let config_path = self.app_dir.join("config.json");
        let config_content = r#"{"enableMods":true}"#;
        fs::write(&config_path, config_content)?;
        
        self.log("Starting overlay process...");
        
        // Important: Set state to Running BEFORE spawning process
        self.set_state(ModState::Running);
        
        // Try running the overlay, with retries
        let max_run_retries = 3; // Increased from 2 to 3
        let mut run_retry_count = 0;
        let mut last_run_error = None;
        
        while run_retry_count < max_run_retries {
            if run_retry_count > 0 {
                self.log(&format!("Retrying overlay run (attempt {}/{})", run_retry_count + 1, max_run_retries));
                std::thread::sleep(std::time::Duration::from_millis(1000));
                self.cleanup_mod_tools_processes();
                
                // Additional cleanup - run garbage collection to free memory
                #[cfg(target_os = "windows")]
                {
                    use std::process::Command;
                    let mut gc_cmd = Command::new("powershell");
                    gc_cmd.args(["-Command", "[System.GC]::Collect()"]);
                    #[cfg(target_os = "windows")]
                    gc_cmd.creation_flags(CREATE_NO_WINDOW);
                    let _ = gc_cmd.output();
                }
            }
            
            // Run the overlay process - EXACT format from CSLOL
            let mut command = std::process::Command::new(&mod_tools_path);
            command.args([
                "runoverlay",
                overlay_dir.to_str().unwrap(),
                config_path.to_str().unwrap(),
                &format!("--game:{}", self.game_path.to_str().unwrap()),
                "--opts:configless"
            ]);
            
            #[cfg(target_os = "windows")]
            command.creation_flags(CREATE_NO_WINDOW);
            
            match command.spawn() {
                Ok(_) => {
                    self.log("Overlay process started successfully");
                    
                    // Emit success to frontend if available
                    if let Some(app) = &self.app_handle {
                        let _ = app.emit("injection-status", "success");
                    }
                    
                    return Ok(());
                },
                Err(e) => {
                    if run_retry_count + 1 >= max_run_retries {
                        self.log(&format!("Failed to start overlay process: {}", e));
                    }
                    run_retry_count += 1;
                    last_run_error = Some(match e.kind() {
                        io::ErrorKind::NotFound => InjectionError::OverlayError(format!(
                            "mod-tools.exe not found or is inaccessible at path: {}. Please install CSLOL Manager or copy the correct mod-tools.exe to the application directory.", 
                            mod_tools_path.display()
                        )),
                        io::ErrorKind::PermissionDenied => InjectionError::OverlayError(format!(
                            "Permission denied when trying to run mod-tools.exe. Try running the application as administrator."
                        )),
                        _ => InjectionError::OverlayError(format!(
                            "Error running mod-tools.exe: {}. Please ensure it's correctly installed and compatible with your system.", 
                            e
                        ))
                    });
                    if run_retry_count < max_run_retries {
                        self.log(&format!("Failed to start overlay process: {}. Retrying...", e));
                        continue;
                    }
                }
            }
        }
        
        // If we got here, all retries failed
        self.set_state(ModState::Idle); // Reset state on error
        
        if let Some(err) = last_run_error {
            self.log(&format!("Failed to start overlay process after {} attempts", max_run_retries));
            Err(err)
        } else {
            Err(InjectionError::OverlayError("Failed to start overlay process after multiple attempts".into()))
        }
    }
    
    // Helper function to kill mod-tools processes - extracted from cleanup for reuse
    fn cleanup_mod_tools_processes(&self) {
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
    }

    // Main injection method that does all steps
    pub fn inject_skins(&mut self, skins: &[Skin], fantome_files_dir: &Path) -> Result<(), InjectionError> {
        // Emit start event to frontend
        if let Some(_app) = &self.app_handle {
            let _ = _app.emit("injection-status", "injecting");
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
                    self.log("ERROR: Processing failed, mod structure invalid");
                    return Err(InjectionError::MissingFantomeFile("Mod structure invalid".into()));
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
                    }
                }
            }
        }
        
        // Start the overlay process - THIS is the key part that makes skins actually show in-game!
        self.run_overlay()?;
        
        self.log("Skin injection completed successfully");
        // Note: We don't set state to Idle because we're now in Running state with the overlay active
        // After all steps complete successfully, emit end event
        if let Some(_app) = &self.app_handle {
            let _ = _app.emit("injection-status", "completed");
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
#[allow(dead_code)]
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
