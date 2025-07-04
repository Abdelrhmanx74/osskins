use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use once_cell::sync::Lazy;
use walkdir::WalkDir;
use tauri::{AppHandle, Manager};
use serde_json;
use crate::injection::error::{InjectionError, Skin};

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
