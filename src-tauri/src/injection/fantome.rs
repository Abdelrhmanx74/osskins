use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;
use zip::ZipArchive;
use walkdir::WalkDir;
use memmap2::MmapOptions;
use tauri::{AppHandle, Manager};
use crate::injection::error::{InjectionError, Skin};
use crate::injection::file_index::get_global_index;

// Fantome file processing operations

impl crate::injection::core::SkinInjector {
    // Extract .fantome file (similar to utility::unzip in CSLOL Manager)
    pub(crate) fn extract_fantome(&mut self, fantome_path: &Path, output_dir: &Path) -> Result<(), InjectionError> {
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
    pub(crate) fn extract_fantome_mmap(&mut self, fantome_path: &Path, output_dir: &Path) -> Result<(), InjectionError> {
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
    pub(crate) fn is_valid_mod_dir(&self, dir_path: &Path) -> bool {
        dir_path.join("META").join("info.json").exists()
    }
    
    // Find appropriate .fantome file for a skin
    pub(crate) fn find_fantome_for_skin(&mut self, skin: &Skin, fantome_files_dir: &Path) -> Result<Option<PathBuf>, InjectionError> {
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
                for entry in fs::read_dir(&champ_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    // detect .fantome
                    if path.extension().and_then(|e| e.to_str()) == Some("fantome") {
                        if path.file_stem().and_then(|s| s.to_str()).map_or(false, |stem| stem.ends_with(&skin_id_str)) {
                            return Ok(Some(path));
                        }
                    }
                    // detect .zip archives too
                    if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                        if path.file_name().and_then(|s| s.to_str()).map_or(false, |name| name.contains(&skin_id_str)) {
                            self.log(&format!("Found ZIP archive for skin: {}", path.display()));
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }
        
        // As a last fallback, scan entire directory for matching zip files
        for entry in WalkDir::new(fantome_files_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "fantome" || ext == "zip" {
                        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        if name.contains(&skin_id_str) {
                            self.log(&format!("Found archive in fallback scan: {}", path.display()));
                            return Ok(Some(path.to_path_buf()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
    
    
    // Create a mod directory structure from extracted fantome files
    pub(crate)  fn create_mod_from_extracted(&mut self, extract_dir: &Path, mod_dir: &Path) -> Result<(), InjectionError> {
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
    pub(crate) fn process_fantome_file(&mut self, fantome_path: &Path) -> Result<PathBuf, InjectionError> {
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
    
}

// Add a function to check for and copy the pre-built default overlay
pub fn copy_default_overlay(app_handle: &AppHandle, destination: &Path) -> Result<bool, InjectionError> {
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
