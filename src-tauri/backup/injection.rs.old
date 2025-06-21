use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Emitter};
use walkdir::WalkDir;
use zip::ZipArchive;
use std::env;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use once_cell::sync::Lazy;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use memmap2::{Mmap, MmapOptions};
use crate::commands::TerminalLog;
use chrono::Utc;

// Define Windows-specific constants at the module level
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Add overlay cache to optimize injection performance
pub static OVERLAY_CACHE: Lazy<Arc<Mutex<OverlayCache>>> = Lazy::new(|| {
    Arc::new(Mutex::new(OverlayCache::new()))
});

// Structure to cache pre-built overlays
#[derive(Debug, Default)]
pub struct OverlayCache {
    // Map mod directory hash to pre-built overlay directory
    overlays: HashMap<String, (PathBuf, Instant)>,
    // Last time the cache was cleaned up
    last_cleanup: Option<Instant>,
}

impl OverlayCache {
    // Create a new empty cache
    pub fn new() -> Self {
        Self {
            overlays: HashMap::new(),
            last_cleanup: None,
        }
    }
    
    // Generate a hash key for the set of mods
    fn generate_hash(&self, mods: &[String]) -> String {
        let mut sorted_mods = mods.to_vec();
        sorted_mods.sort();
        
        // Create a simple hash by joining all mod names
        let combined = sorted_mods.join("_");
        format!("{:x}", md5::compute(combined))
    }
    
    // Check if we have a valid cached overlay for the given mods
    pub fn get_cached_overlay(&mut self, mods: &[String]) -> Option<PathBuf> {
        // Clean up old entries first if needed
        self.cleanup_old_entries();
        
        let hash = self.generate_hash(mods);
        
        // Check if we have a cached overlay
        if let Some((path, time)) = self.overlays.get_mut(&hash) {
            // Check if the overlay exists and is not too old (30 minutes max)
            if path.exists() && time.elapsed() < Duration::from_secs(30 * 60) {
                // Update the timestamp to keep this entry fresh
                *time = Instant::now();
                return Some(path.clone());
            }
        }
        
        None
    }
    
    // Add a newly built overlay to the cache
    pub fn add_overlay(&mut self, mods: &[String], path: PathBuf) {
        let hash = self.generate_hash(mods);
        self.overlays.insert(hash, (path, Instant::now()));
        
        // Maybe clean up old entries if we haven't in a while
        self.cleanup_old_entries();
    }
    
    // Clean up old cache entries
    fn cleanup_old_entries(&mut self) {
        // Only clean up once per hour
        if let Some(last_time) = self.last_cleanup {
            if last_time.elapsed() < Duration::from_secs(3600) {
                return;
            }
        }
        
        // Remove entries older than 2 hours
        let max_age = Duration::from_secs(2 * 3600);
        let now = Instant::now();
        
        self.overlays.retain(|_, (path, time)| {
            let keep = time.elapsed() < max_age && path.exists();
            // Try to remove the directory if we're discarding it
            if !keep && path.exists() {
                let _ = std::fs::remove_dir_all(path);
            }
            keep
        });
        
        self.last_cleanup = Some(now);
    }
}

// Error handling similar to CS LOL Manager
#[derive(Debug)]
pub enum InjectionError {
    IoError(io::Error),
    InvalidGamePath(String),
    MissingFantomeFile(String),
    ProcessError(String),
    ConfigError(String),
    OverlayError(String),
    Timeout(String),
    Aborted(String),
    WalkdirError(walkdir::Error),
    ZipError(zip::result::ZipError),
}

impl std::fmt::Display for InjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IO Error: {}", err),
            Self::InvalidGamePath(msg) => write!(f, "Invalid game path: {}", msg),
            Self::MissingFantomeFile(msg) => write!(f, "Missing fantome file: {}", msg),
            Self::ProcessError(msg) => write!(f, "Process error: {}", msg),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::OverlayError(msg) => write!(f, "Overlay error: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::Aborted(msg) => write!(f, "Aborted: {}", msg),
            Self::WalkdirError(err) => write!(f, "Walkdir error: {}", err),
            Self::ZipError(err) => write!(f, "Zip error: {}", err),
        }
    }
}

impl std::error::Error for InjectionError {}

impl From<io::Error> for InjectionError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<walkdir::Error> for InjectionError {
    fn from(err: walkdir::Error) -> Self {
        Self::WalkdirError(err)
    }
}

impl From<zip::result::ZipError> for InjectionError {
    fn from(err: zip::result::ZipError) -> Self {
        Self::ZipError(err)
    }
}

// Define the types we need
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome_path: Option<String>, // Add fantome path from the JSON
}

// ModState enum - Similar to CS LOL Manager's state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModState {
    Uninitialized,
    Idle,
    Busy,
    Running,
    CriticalError,
}

// This represents a message event for the patcher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatcherMessage {
    WaitStart,
    Found,
    WaitInit,
    Scan,
    NeedSave,
    WaitPatchable,
    Patch,
    WaitExit,
    Done,
}

impl PatcherMessage {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::WaitStart => "Waiting for league match to start",
            Self::Found => "Found League",
            Self::WaitInit => "Wait initialized",
            Self::Scan => "Scanning",
            Self::NeedSave => "Saving",
            Self::WaitPatchable => "Wait patchable",
            Self::Patch => "Patching",
            Self::WaitExit => "Waiting for exit",
            Self::Done => "League exited",
        }
    }
}

// Main skin injector class - simplified without profiles
pub struct SkinInjector {
    state: ModState,
    app_dir: PathBuf,
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
        if let Some(app) = &self.app_handle {
            emit_terminal_log_injection(app, &emoji_message);
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
        if (!champions_dir.exists()) {
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
        if let Some(app) = &self.app_handle {
            // Try to get the path from our optimized index first
            if let Ok(index) = get_global_index(app) {
                let start = Instant::now();
                if let Some(path) = index.lock().unwrap().find_fantome_for_skin(skin, fantome_files_dir) {
                    self.log(&format!("Found fantome file in index (took {:?}): {}", 
                                     start.elapsed(), path.display()));
                    return Ok(Some(path));
                }
            }
        }
        
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
        if let Some(app) = &self.app_handle {
            let error_msg = format!("No fantome file found for skin: champion_id={}, skin_id={}, chroma_id={:?}", skin.champion_id, skin.skin_id, skin.chroma_id);
            let _ = app.emit("injection-status", false);
            let _ = app.emit("skin-injection-error", error_msg);
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
        if (!game_cfg_path.exists()) {
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
        // Check if mod-tools.exe exists
        let mod_tools_path = match &self.mod_tools_path {
            Some(path) => {
                if (!path.exists()) {
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
        
        // First create the overlay
        let game_mods_dir = self.game_path.join("mods");
        let overlay_dir = self.app_dir.join("overlay");
        
        // Make sure overlay directory exists or is recreated
        if overlay_dir.exists() {
            // Try to remove the overlay dir multiple times with delays
            // This helps with Windows file locks that might be causing access violations
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
            
            // If still exists, return error
            if overlay_dir.exists() && attempts >= max_attempts {
                return Err(InjectionError::OverlayError(
                    "Cannot remove existing overlay directory. It may be locked by another process.".into()
                ));
            }
        }
        fs::create_dir_all(&overlay_dir)?;

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

        // Check if we have any valid mods
        if mod_names.is_empty() {
            self.log("No valid mods found in game directory");
        } else {
            self.log(&format!("Found {} mods to include in overlay", mod_names.len()));
        }
        
        // First try to use a pre-built default overlay from resources
        // This is especially helpful for the first injection which is often slow
        let used_prebuilt_empty = if let Some(app_handle) = &self.app_handle {
            if mod_names.is_empty() {
                // For empty mod list, try the most optimized path - use the pre-built empty overlay
                match copy_default_overlay(app_handle, &overlay_dir) {
                    Ok(true) => {
                        self.log("Using pre-built empty overlay template for faster injection");
                        true
                    },
                    _ => false
                }
            } else {
                false
            }
        } else {
            false
        };
        
        if (!used_prebuilt_empty) {
            // Check if we have a pre-built overlay for this set of mods in cache
            let mut cached_overlay = None;
            if let Ok(mut cache) = OVERLAY_CACHE.lock() {
                cached_overlay = cache.get_cached_overlay(&mod_names);
            }
            
            if let Some(cached_dir) = cached_overlay {
                self.log(&format!("Using pre-built overlay from cache: {}", cached_dir.display()));
                
                // Copy cached overlay to our overlay directory
                for entry in WalkDir::new(&cached_dir) {
                    let entry = entry?;
                    let path = entry.path();
                    let rel_path = path.strip_prefix(&cached_dir)
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
                
                self.log("Cached overlay copied successfully");
            } else {
                // Join mod names with / as CSLOL expects
                let mods_arg = mod_names.join("/");

                self.log("Creating mod overlay...");
                
                // Try the mkoverlay command with retries for access violation errors (0xc0000005)
                let max_retries = 5; // Increased from 3 to 5
                let mut retry_count = 0;
                let mut last_error = None;
                
                // Create a temp directory for our overlay that will be moved to the cache later
                let temp_overlay_dir = self.app_dir.join("temp_overlay");
                if temp_overlay_dir.exists() {
                    fs::remove_dir_all(&temp_overlay_dir)?;
                }
                fs::create_dir_all(&temp_overlay_dir)?;
                
                while retry_count < max_retries {
                    // Small delay between retries to let resources free up
                    if retry_count > 0 {
                        self.log(&format!("Retrying overlay creation (attempt {}/{})", retry_count + 1, max_retries));
                        // Increased delay between retries
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        
                        // Make sure any lingering processes are killed
                        self.cleanup_mod_tools_processes();
                        
                        // For additional retries, recreate the overlay directory to ensure it's clean
                        if temp_overlay_dir.exists() {
                            match fs::remove_dir_all(&temp_overlay_dir) {
                                Ok(_) => fs::create_dir_all(&temp_overlay_dir)?,
                                Err(e) => {
                                    self.log(&format!("Could not clean overlay directory: {}", e));
                                    // Try to continue anyway
                                }
                            }
                        } else {
                            fs::create_dir_all(&temp_overlay_dir)?;
                        }
                    }
                    
                    // Add more explicit memory cleanup hint
                    #[cfg(target_os = "windows")]
                    {
                        use std::process::Command;
                        // Run a quick garbage collection via PowerShell to help free memory
                        if retry_count > 0 {
                            let mut gc_cmd = Command::new("powershell");
                            gc_cmd.args(["-Command", "[System.GC]::Collect()"]);
                            gc_cmd.creation_flags(CREATE_NO_WINDOW);
                            let _ = gc_cmd.output(); // Ignore output
                        }
                    }
                    
                    let mut command = std::process::Command::new(&mod_tools_path);
                    command.args([
                        "mkoverlay",
                        game_mods_dir.to_str().unwrap(),
                        temp_overlay_dir.to_str().unwrap(),
                        &format!("--game:{}", self.game_path.to_str().unwrap()),  // Use game_path which points to Game directory
                        &format!("--mods:{}", mods_arg),
                        "--noTFT",
                        "--ignoreConflict"
                    ]);
                    
                    #[cfg(target_os = "windows")]
                    command.creation_flags(CREATE_NO_WINDOW);
                    
                    // Hide command output for cleaner logs, just show we're working
                    if retry_count == 0 {
                        self.log("Running mkoverlay command...");
                    }
                    
                    match command.output() {
                        Ok(output) => {
                            if output.status.success() {
                                // Success - break out of retry loop
                                self.log("Overlay creation succeeded!");
                                
                                // Cache the overlay if we have a non-empty list of mods
                                if !mod_names.is_empty() {
                                    // Create a permanent cache directory for this overlay
                                    let cache_base = self.app_dir.join("overlay_cache");
                                    if !cache_base.exists() {
                                        fs::create_dir_all(&cache_base)?;
                                    }
                                    
                                    // Use a timestamped name to avoid conflicts
                                    let timestamp = chrono::Local::now().timestamp();
                                    let cache_dir = cache_base.join(format!("overlay_{}", timestamp));
                                    
                                    // Copy files from temp_overlay_dir to cache_dir
                                    fs::create_dir_all(&cache_dir)?;
                                    for entry in WalkDir::new(&temp_overlay_dir) {
                                        let entry = entry?;
                                        let path = entry.path();
                                        let rel_path = path.strip_prefix(&temp_overlay_dir)
                                            .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
                                        let target_path = cache_dir.join(rel_path);
                                        
                                        if path.is_dir() {
                                            fs::create_dir_all(&target_path)?;
                                        } else if path.is_file() {
                                            if let Some(parent) = target_path.parent() {
                                                fs::create_dir_all(parent)?;
                                            }
                                            fs::copy(path, &target_path)?;
                                        }
                                    }
                                    
                                    // Add to our cache
                                    if let Ok(mut cache) = OVERLAY_CACHE.lock() {
                                        cache.add_overlay(&mod_names, cache_dir);
                                    }
                                    
                                    self.log("Overlay saved to cache for faster future use");
                                }
                                
                                // Now copy from temp directory to actual overlay directory
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
                
                // Check if we exhausted our retries
                if retry_count >= max_retries {
                    if let Some(err) = last_error {
                        return Err(err);
                    }
                    return Err(InjectionError::ProcessError("Failed to create overlay after multiple attempts".into()));
                }
            }
        }

        // Create config.json
        let config_path = self.app_dir.join("config.json");
        let config_content = r#"{"enableMods":true}"#;
        fs::write(&config_path, config_content)?;

        self.log("Starting overlay process...");

        // Important: Set state to Running BEFORE spawning process
        self.set_state(ModState::Running);

        // Try running the overlay, with retries
        let max_run_retries = 2;
        let mut run_retry_count = 0;
        let mut last_run_error = None;
        
        while run_retry_count < max_run_retries {
            if run_retry_count > 0 {
                self.log(&format!("Retrying overlay run (attempt {}/{})", run_retry_count + 1, max_run_retries));
                std::thread::sleep(std::time::Duration::from_millis(1000));
                
                // Make sure any lingering processes are killed
                self.cleanup_mod_tools_processes();
            }
            
            // Run the overlay process - EXACT format from CSLOL
            let mut command = std::process::Command::new(&mod_tools_path);
            command.args([
                "runoverlay",
                overlay_dir.to_str().unwrap(),
                config_path.to_str().unwrap(),
                &format!("--game:{}", self.game_path.to_str().unwrap()),  // Use game_path which points to Game directory
                "--opts:configless"
            ]);
            
            #[cfg(target_os = "windows")]
            command.creation_flags(CREATE_NO_WINDOW);

            match command.spawn() {
                Ok(_) => {
                    self.log("Overlay process started successfully");
                    return Ok(());
                },
                Err(e) => {
                    run_retry_count += 1;
                    
                    // Store error for potential later use
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
                    let game_fantome_path = game_mods_dir.join(fantome_path.file_name().unwrap());
                    fs::copy(&fantome_path, &game_fantome_path)?;
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
                    return Err(InjectionError::OverlayError(format!(
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

    // Add this method above the new method or other impl methods
    
    // Initialize cache for faster first-time injection
    pub fn initialize_cache(&mut self) -> Result<(), InjectionError> {
        self.log("Pre-building overlay cache for faster injections...");
        
        // First ensure we have the basic directories
        let game_mods_dir = self.game_path.join("mods");
        if !game_mods_dir.exists() {
            fs::create_dir_all(&game_mods_dir)?;
        }
        
        // Create empty overlay cache directories
        let overlay_cache_dir = self.app_dir.join("overlay_cache");
        let temp_overlay_dir = self.app_dir.join("temp_overlay");
        
        if !overlay_cache_dir.exists() {
            fs::create_dir_all(&overlay_cache_dir)?;
        }
        
        // Clean any temp directories
        if temp_overlay_dir.exists() {
            fs::remove_dir_all(&temp_overlay_dir)?;
        }
        
        // Create a temporary empty mod for caching
        fs::create_dir_all(&temp_overlay_dir)?;
        let empty_mod_dir = temp_overlay_dir.join("empty_mod");
        fs::create_dir_all(&empty_mod_dir.join("META"))?;
        fs::create_dir_all(&empty_mod_dir.join("WAD"))?;
        
        // Create a basic info.json
        let info_json = r#"{
            "Name": "EmptyMod",
            "Version": "1.0.0",
            "Author": "osskins",
            "Description": "Pre-built empty mod for faster first injection"
        }"#;
        fs::write(empty_mod_dir.join("META").join("info.json"), info_json)?;
        
        // Look for mod-tools.exe
        let mod_tools_path = match &self.mod_tools_path {
            Some(path) => {
                if !path.exists() {
                    self.log("mod-tools.exe not found at expected path");
                    return Ok(());
                }
                path.clone()
            },
            None => {
                self.log("mod-tools.exe not found");
                return Ok(());
            },
        };
        
        self.log("Building empty overlay for cache...");
        
        // Create the overlay cache entry for empty mod (no skins)
        let timestamp = chrono::Local::now().timestamp();
        let cache_dir = overlay_cache_dir.join(format!("overlay_empty_{}", timestamp));
        fs::create_dir_all(&cache_dir)?;
        
        #[cfg(target_os = "windows")]
        {
            // Build empty overlay
            let mut command = std::process::Command::new(&mod_tools_path);
            command.args([
                "mkoverlay",
                temp_overlay_dir.to_str().unwrap(),
                cache_dir.to_str().unwrap(),
                &format!("--game:{}", self.game_path.to_str().unwrap()),
                "--mods:empty_mod",
                "--noTFT",
                "--ignoreConflict"
            ]);
            command.creation_flags(CREATE_NO_WINDOW);
            
            // This runs in the background during startup, so errors are non-fatal
            match command.output() {
                Ok(output) => {
                    if output.status.success() {
                        self.log("Successfully pre-built empty overlay cache");
                        
                        // Add to our cache
                        if let Ok(mut cache) = OVERLAY_CACHE.lock() {
                            cache.add_overlay(&Vec::<String>::new(), cache_dir);
                            self.log("Added empty overlay to cache");
                        }
                    } else {
                        self.log(&format!("Warning: Failed to pre-build empty overlay cache: {:?}", 
                                output.status.code()));
                    }
                },
                Err(e) => {
                    self.log(&format!("Error pre-building empty overlay: {}", e));
                }
            }
        }
        
        // Clean up temp directory
        if temp_overlay_dir.exists() {
            let _ = fs::remove_dir_all(&temp_overlay_dir);
        }
        
        // Also initialize the global file index
        if let Some(app_handle) = &self.app_handle {
            if let Ok(index) = get_global_index(app_handle) {
                let app_dir = match app_handle.path().app_data_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        // Convert tauri::Error to an io::Error with appropriate error kind
                        self.log(&format!("Failed to get app data dir: {}", e));
                        return Ok(()); // Continue instead of failing
                    }
                };
                
                let champions_dir = app_dir.join("champions");
                
                let mut index_guard = index.lock().unwrap();
                let _ = index_guard.index_champions(&champions_dir);
                let _ = index_guard.index_fantome_files(&champions_dir);
                
                self.log("Initialized global file index for faster lookups");
            }
        }
        
        self.log("Cache initialization completed");
        Ok(())
    }

    // Direct WAD injection fallback for when mod-tools overlay fails
    fn inject_skin_direct_fallback(&mut self, mod_names: &[String]) -> Result<(), InjectionError> {
        self.log("🔄 Attempting direct WAD injection fallback...");
        
        // Check if there are any valid mods
        if mod_names.is_empty() {
            self.log("❌ No valid mods found for direct injection");
            return Err(InjectionError::OverlayError("No valid mods to inject".into()));
        }
        
        // First, ensure game directory is accessible
        let game_data_dir = self.game_path.join("DATA");
        if !game_data_dir.exists() {
            self.log("❌ Game DATA directory not found");
            return Err(InjectionError::OverlayError("Game DATA directory not found".into()));
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
            return Err(InjectionError::OverlayError("No WAD files found to inject".into()));
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

// Add a FileIndex struct to cache paths and champion data
#[derive(Debug, Default)]
pub struct FileIndex {
    // Map champion_id to champion name
    champion_names: HashMap<u32, String>,
    // Map (champion_id, skin_id) to fantome file path
    skin_paths: HashMap<(u32, Option<u32>), Vec<PathBuf>>,
    // Map champion name to champion ID
    champion_ids: HashMap<String, u32>,
    // Track all discovered fantome files
    all_fantome_files: Vec<(PathBuf, Instant)>,
    // Track fantome files by filename for quick lookup
    fantome_by_filename: HashMap<String, PathBuf>,
    // Last time the index was built
    last_indexed: Option<Instant>,
}

impl FileIndex {
    // Create a new empty index
    pub fn new() -> Self {
        Self::default()
    }
    
    // Index all champions in a directory
    pub fn index_champions(&mut self, champions_dir: &Path) -> Result<(), InjectionError> {
        println!("Indexing champions in {}", champions_dir.display());
        let start = Instant::now();
        
        if !champions_dir.exists() {
            return Ok(());
        }
        
        let entries = fs::read_dir(champions_dir)?;
        
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            
            let champion_name = path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
                
            // Look for champion JSON file
            let json_file = path.join(format!("{}.json", champion_name));
            
            if let Ok(content) = fs::read_to_string(&json_file) {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = data.get("id").and_then(|v| v.as_u64()) {
                        let champion_id = id as u32;
                        self.champion_names.insert(champion_id, champion_name.clone());
                        self.champion_ids.insert(champion_name, champion_id);
                    }
                }
            }
        }
        
        println!("Indexed {} champions in {:?}", self.champion_names.len(), start.elapsed());
        Ok(())
    }
    
    // Index all fantome files in a directory structure
    pub fn index_fantome_files(&mut self, base_dir: &Path) -> Result<(), InjectionError> {
        println!("Indexing fantome files in {}", base_dir.display());
        let start = Instant::now();
        
        // Clear existing data
        self.skin_paths.clear();
        self.fantome_by_filename.clear();
        self.all_fantome_files.clear();
        
        if !base_dir.exists() {
            return Ok(());
        }
        
        // Walk the directory tree
        for entry in WalkDir::new(base_dir) {
            let entry = entry?;
            let path = entry.path();
            
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("fantome") {
                continue;
            }
            
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
                
            // Store by filename for direct lookups
            self.fantome_by_filename.insert(filename.clone(), path.to_path_buf());
            
            // Store in all files collection
            self.all_fantome_files.push((path.to_path_buf(), Instant::now()));
            
            // Try to parse the filename for champion/skin IDs
            // Format examples: ChampionName_SkinID.fantome or ChampionName_SkinID_chroma_ChromaID.fantome
            let parts: Vec<&str> = filename.split('_').collect();
            
            if parts.len() >= 2 {
                // The part before the first underscore might be the champion name
                let possible_champion_name = parts[0].to_lowercase();
                
                // Try to find champion ID from name
                if let Some(&champion_id) = self.champion_ids.get(&possible_champion_name) {
                    // The second part might be the skin ID
                    if let Some(skin_id_str) = parts.get(1) {
                        if let Ok(skin_id) = skin_id_str.parse::<u32>() {
                            // Check if it's a chroma
                            let chroma_id = if parts.len() >= 4 && parts[2] == "chroma" {
                                parts.get(3).and_then(|id_str| id_str.parse::<u32>().ok())
                            } else {
                                None
                            };
                            
                            // Store the path indexed by (champion_id, skin_id, chroma_id)
                            let key = (champion_id, chroma_id);
                            self.skin_paths.entry(key)
                                .or_insert_with(Vec::new)
                                .push(path.to_path_buf());
                        }
                    }
                }
            }
        }
        
        self.last_indexed = Some(Instant::now());
        println!("Indexed {} fantome files in {:?}", 
            self.all_fantome_files.len(), start.elapsed());
        
        Ok(())
    }
    
    // Find fantome file for a skin using the indexed data
    pub fn find_fantome_for_skin(&self, skin: &Skin, fantome_files_dir: &Path) -> Option<PathBuf> {
        // First, check if we have it in our skin paths table
        let key = (skin.champion_id, skin.chroma_id);
        
        if let Some(paths) = self.skin_paths.get(&key) {
            for path in paths {
                // For indexed paths, verify they exist and contain the skin ID
                if path.exists() {
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    
                    if filename.contains(&skin.skin_id.to_string()) {
                        return Some(path.clone());
                    }
                }
            }
        }
        
        // If not found, check direct path from JSON if provided
        if let Some(fantome_path) = &skin.fantome_path {
            // Try direct file lookup first (fastest)
            let filename = fantome_path.split('/').last().unwrap_or(fantome_path);
            if let Some(path) = self.fantome_by_filename.get(filename) {
                if path.exists() {
                    return Some(path.clone());
                }
            }
            
            // Try direct path
            let direct_path = fantome_files_dir.join(fantome_path);
            if direct_path.exists() {
                return Some(direct_path);
            }
        }
        
        // Not found in index
        None
    }
    
    // Get champion name, preferring the cached version
    pub fn get_champion_name(&self, champion_id: u32) -> Option<String> {
        self.champion_names.get(&champion_id).cloned()
    }
    
    // Check if index needs refresh (older than 5 minutes)
    pub fn needs_refresh(&self) -> bool {
        match self.last_indexed {
            Some(time) => time.elapsed().as_secs() > 300, // 5 minutes
            None => true,
        }
    }
}

// Create a global static instance for caching across the application
pub static GLOBAL_FILE_INDEX: Lazy<Arc<Mutex<FileIndex>>> = Lazy::new(|| {
    Arc::new(Mutex::new(FileIndex::new()))
});

// Function to get or initialize the global index
pub fn get_global_index(app_handle: &AppHandle) -> Result<Arc<Mutex<FileIndex>>, InjectionError> {
    let index = GLOBAL_FILE_INDEX.clone();
    
    // Check if we need to initialize the index
    let needs_init = {
        let locked_index = index.lock().unwrap();
        locked_index.champion_names.is_empty() || locked_index.needs_refresh()
    };
    
    if needs_init {
        // Get the champions directory path
        let app_data_dir = app_handle.path().app_data_dir()
            .map_err(|e| InjectionError::IoError(io::Error::new(io::ErrorKind::NotFound, format!("{}", e))))?;
        let champions_dir = app_data_dir.join("champions");
        
        // Initialize with locked access
        let mut locked_index = index.lock().unwrap();
        locked_index.index_champions(&champions_dir)?;
        locked_index.index_fantome_files(&champions_dir)?;
    }
    
    Ok(index)
}

// Add a function to check for and copy the pre-built default overlay
fn copy_default_overlay(app_handle: &AppHandle, destination: &Path) -> Result<bool, InjectionError> {
    // Check if we have a pre-built overlay in resources
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        // First check in cslol-tools subfolder
        let default_overlay = resource_dir.join("cslol-tools").join("empty_overlay");
        if default_overlay.exists() && default_overlay.is_dir() {
            println!("Found pre-built overlay at: {}", default_overlay.display());
            
            // Create the destination directory if it doesn't exist
            if !destination.exists() {
                fs::create_dir_all(destination)?;
            }
            
            // Copy the files from the default overlay
            for entry in WalkDir::new(&default_overlay) {
                let entry = entry?;
                let path = entry.path();
                let rel_path = path.strip_prefix(&default_overlay)
                    .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
                
                let target_path = destination.join(rel_path);
                
                if path.is_dir() {
                    fs::create_dir_all(&target_path)?;
                } else if path.is_file() {
                    if let Some(parent) = target_path.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent)?;
                        }
                    }
                    fs::copy(path, &target_path)?;
                }
            }
            
            println!("Successfully copied pre-built overlay template");
            return Ok(true);
        }
    }
    
    Ok(false)
}

fn emit_terminal_log_injection(app: &AppHandle, message: &str) {
    let log = TerminalLog {
        message: message.to_string(),
        log_type: "injection".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };
    let _ = app.emit("terminal-log", log);
}