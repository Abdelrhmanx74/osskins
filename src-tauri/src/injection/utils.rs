use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use once_cell::sync::Lazy;
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

use crate::injection::error::InjectionError;
use crate::injection::types::FileIndex;

// Create a global static instance for caching across the application
pub static GLOBAL_FILE_INDEX: Lazy<Arc<Mutex<FileIndex>>> = Lazy::new(|| {
    Arc::new(Mutex::new(FileIndex::new()))
});

impl FileIndex {
    // Index all champions in a directory
    pub fn index_champions(&mut self, champions_dir: &Path) -> Result<(), InjectionError> {
        println!("Indexing champions in {}", champions_dir.display());
        let start = Instant::now();
        
        if !champions_dir.exists() {
            return Err(InjectionError::IoError(io::Error::new(
                io::ErrorKind::NotFound, 
                format!("Champions directory not found: {}", champions_dir.display())
            )));
        }
        
        let entries = fs::read_dir(champions_dir)?;
        
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                // Look for champion ID file
                let champion_id_file = path.join("champion_id.txt");
                if champion_id_file.exists() {
                    if let Ok(id_str) = fs::read_to_string(&champion_id_file) {
                        if let Ok(champ_id) = id_str.trim().parse::<u32>() {
                            self.champion_names.insert(champ_id, dir_name.clone());
                            self.champion_ids.insert(dir_name, champ_id);
                        }
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
            return Err(InjectionError::IoError(io::Error::new(
                io::ErrorKind::NotFound, 
                format!("Base directory not found: {}", base_dir.display())
            )));
        }
        
        // Walk the directory tree
        for entry in WalkDir::new(base_dir) {
            let entry = entry?;
            let path = entry.path();
            
            // Only process .fantome files
            if path.is_file() && path.extension().map_or(false, |ext| ext == "fantome") {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                // Add to all_fantome_files list
                self.all_fantome_files.push((path.to_path_buf(), Instant::now()));
                
                // Add to filename index
                self.fantome_by_filename.insert(file_name.clone(), path.to_path_buf());
                
                // Try to extract champion_id and skin_id from path or file name
                if let Some((champion_id, _skin_id)) = extract_skin_info_from_path(path) {
                    let key = (champion_id, None);  // No chroma support yet in this simplified version
                    self.skin_paths.entry(key).or_insert_with(Vec::new).push(path.to_path_buf());
                }
            }
        }
        
        self.last_indexed = Some(Instant::now());
        println!("Indexed {} fantome files in {:?}", self.all_fantome_files.len(), start.elapsed());
        
        Ok(())
    }
    
    // Find fantome file for a skin using the indexed data
    pub fn find_fantome_for_skin(&self, skin: &crate::injection::types::Skin, fantome_files_dir: &Path) -> Option<PathBuf> {
        // First, check if we have it in our skin paths table
        let key = (skin.champion_id, skin.chroma_id);
        
        if let Some(paths) = self.skin_paths.get(&key) {
            if !paths.is_empty() {
                return Some(paths[0].clone());
            }
        }
        
        // If not found, check direct path from JSON if provided
        if let Some(fantome_path) = &skin.fantome_path {
            // Check by direct path
            let full_path = fantome_files_dir.join(fantome_path);
            if full_path.exists() {
                return Some(full_path);
            }
            
            // Check by filename
            if let Some(filename) = fantome_path.split('/').last() {
                if let Some(path) = self.fantome_by_filename.get(filename) {
                    return Some(path.clone());
                }
            }
        }
        
        // Not found in index
        None
    }
    
    #[allow(dead_code)]
    // Get champion name, preferring the cached version
    pub fn get_champion_name(&self, champion_id: u32) -> Option<String> {
        self.champion_names.get(&champion_id).cloned()
    }
    
    // Check if index needs refresh (older than 5 minutes)
    pub fn needs_refresh(&self) -> bool {
        match self.last_indexed {
            Some(time) => time.elapsed() > Duration::from_secs(5 * 60),
            None => true,
        }
    }
}

// Function to extract skin information from a file path
fn extract_skin_info_from_path(path: &Path) -> Option<(u32, u32)> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    
    // Try to extract skin/champion IDs from filename
    // Common formats like "Champion_Skin_1234.fantome" or "1234_Champion_Skin.fantome"
    let parts: Vec<&str> = file_name.split(|c: char| !c.is_alphanumeric()).collect();
    
    for part in parts {
        if let Ok(id) = part.parse::<u32>() {
            if id > 0 {
                // Very simplistic approach - assume first numeric part over 0 is either champion or skin ID
                // In a real implementation, you'd need more sophisticated parsing based on your file naming conventions
                return Some((id, 0)); // For simplicity, we're defaulting skin_id to 0
            }
        }
    }
    
    None
}

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