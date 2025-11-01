use tauri::{AppHandle, Manager};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::injection::MiscItem;
use crate::commands::types::SavedConfig;

// Misc item management commands

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadMiscItemRequest {
    pub name: String,
    pub item_type: String, // "map", "font", "hud", "misc"
}

#[tauri::command]
pub async fn upload_misc_item(
    app: AppHandle,
    request: UploadMiscItemRequest,
) -> Result<MiscItem, String> {
    // Get app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Create misc_items directory if it doesn't exist
    let misc_items_dir = app_data_dir.join("misc_items");
    fs::create_dir_all(&misc_items_dir)
        .map_err(|e| format!("Failed to create misc items directory: {}", e))?;

    // Create misc_items.json if it doesn't exist
    let misc_items_file = misc_items_dir.join("misc_items.json");
    if !misc_items_file.exists() {
        let empty_items: Vec<MiscItem> = vec![];
        let json_content = serde_json::to_string_pretty(&empty_items)
            .map_err(|e| format!("Failed to serialize empty misc items: {}", e))?;
        fs::write(&misc_items_file, json_content)
            .map_err(|e| format!("Failed to create misc items file: {}", e))?;
    }

    // Read existing misc items
    let existing_content = fs::read_to_string(&misc_items_file)
        .map_err(|e| format!("Failed to read misc items file: {}", e))?;
    let mut misc_items: Vec<MiscItem> = serde_json::from_str(&existing_content)
        .map_err(|e| format!("Failed to parse misc items: {}", e))?;

    // Open file dialog to select skin_file file
    let file_dialog = rfd::FileDialog::new()
        .add_filter("Fantome Files", &["skin_file"])
        .set_title("Select Misc Item Fantome File");

    let selected_file = file_dialog.pick_file()
        .ok_or_else(|| "No file selected".to_string())?;

    // Generate unique ID for the misc item
    let item_id = format!("{}_{}", request.item_type, chrono::Utc::now().timestamp_millis());
    
    // Create filename based on item type and name
    let safe_name = request.name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();
    let filename = format!("{}_{}.skin_file", request.item_type, safe_name);
    let dest_path = misc_items_dir.join(&filename);

    // Copy the selected file to the misc items directory
    fs::copy(&selected_file, &dest_path)
        .map_err(|e| format!("Failed to copy misc item file: {}", e))?;

    // Create misc item entry
    let misc_item = MiscItem {
        id: item_id,
        name: request.name.clone(),
        item_type: request.item_type.clone(),
        skin_file_path: filename,
    };

    // Add to the list
    misc_items.push(misc_item.clone());

    // Save updated misc items
    let json_content = serde_json::to_string_pretty(&misc_items)
        .map_err(|e| format!("Failed to serialize misc items: {}", e))?;
    fs::write(&misc_items_file, json_content)
        .map_err(|e| format!("Failed to save misc items: {}", e))?;

    println!("Misc item uploaded successfully: {}", misc_item.name);
    Ok(misc_item)
}

#[tauri::command]
pub async fn get_misc_items(app: AppHandle) -> Result<Vec<MiscItem>, String> {
    // Get app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let misc_items_file = app_data_dir.join("misc_items").join("misc_items.json");
    
    if !misc_items_file.exists() {
        return Ok(vec![]);
    }

    // Read and parse misc items file
    let content = fs::read_to_string(&misc_items_file)
        .map_err(|e| format!("Failed to read misc items file: {}", e))?;
    
    let misc_items: Vec<MiscItem> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse misc items: {}", e))?;

    Ok(misc_items)
}

#[tauri::command]
pub async fn delete_misc_item(app: AppHandle, item_id: String) -> Result<(), String> {
    // Get app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let misc_items_dir = app_data_dir.join("misc_items");
    let misc_items_file = misc_items_dir.join("misc_items.json");
    
    if !misc_items_file.exists() {
        return Err("Misc items file not found".to_string());
    }

    // Read existing misc items
    let content = fs::read_to_string(&misc_items_file)
        .map_err(|e| format!("Failed to read misc items file: {}", e))?;
    let mut misc_items: Vec<MiscItem> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse misc items: {}", e))?;

    // Find and remove the item
    let item_to_remove = misc_items.iter()
        .find(|item| item.id == item_id)
        .cloned()
        .ok_or_else(|| "Misc item not found".to_string())?;

    // Remove the skin_file file
    let skin_file_path = misc_items_dir.join(&item_to_remove.skin_file_path);
    if skin_file_path.exists() {
        fs::remove_file(&skin_file_path)
            .map_err(|e| format!("Failed to delete skin_file file: {}", e))?;
    }

    // Remove from the list
    misc_items.retain(|item| item.id != item_id);

    // Save updated misc items
    let json_content = serde_json::to_string_pretty(&misc_items)
        .map_err(|e| format!("Failed to serialize misc items: {}", e))?;
    fs::write(&misc_items_file, json_content)
        .map_err(|e| format!("Failed to save misc items: {}", e))?;

    println!("Misc item deleted successfully: {}", item_to_remove.name);
    Ok(())
}

#[tauri::command]
pub async fn upload_multiple_misc_items(app: AppHandle, item_type: String) -> Result<Vec<MiscItem>, String> {
    // Show file dialog for multiple file selection
    let files = rfd::FileDialog::new()
        .add_filter("Fantome Files", &["skin_file"])
        .set_title(&format!("Select {} files", item_type))
        .pick_files()
        .ok_or("No files selected")?;

    if files.is_empty() {
        return Err("No files selected".to_string());
    }

    // Get app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let misc_items_dir = app_data_dir.join("misc_items");
    
    // Create misc_items directory if it doesn't exist
    fs::create_dir_all(&misc_items_dir)
        .map_err(|e| format!("Failed to create misc items directory: {}", e))?;

    let misc_items_file = misc_items_dir.join("misc_items.json");

    // Load existing misc items
    let mut misc_items: Vec<MiscItem> = if misc_items_file.exists() {
        let content = fs::read_to_string(&misc_items_file)
            .map_err(|e| format!("Failed to read misc items file: {}", e))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        vec![]
    };

    let mut uploaded_items = Vec::new();

    // Process each selected file
    for file_path in files {
        let file_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");

        // Generate unique ID
        let item_id = format!("{}_{}_{}", 
            item_type, 
            file_name.replace(' ', "_"), 
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
        );

        // Create safe filename
        let safe_name = file_name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
            .collect::<String>();
        let filename = format!("{}_{}.skin_file", item_type, safe_name);
        let dest_path = misc_items_dir.join(&filename);

        // Copy the selected file to the misc items directory
        fs::copy(&file_path, &dest_path)
            .map_err(|e| format!("Failed to copy misc item file {}: {}", file_name, e))?;

        // Create misc item entry
        let misc_item = MiscItem {
            id: item_id,
            name: safe_name,
            item_type: item_type.clone(),
            skin_file_path: filename,
        };

        // Add to the list
        misc_items.push(misc_item.clone());
        uploaded_items.push(misc_item);
    }

    // Save updated misc items
    let json_content = serde_json::to_string_pretty(&misc_items)
        .map_err(|e| format!("Failed to serialize misc items: {}", e))?;
    fs::write(&misc_items_file, json_content)
        .map_err(|e| format!("Failed to save misc items: {}", e))?;

    println!("Successfully uploaded {} misc items of type {}", uploaded_items.len(), item_type);
    Ok(uploaded_items)
}

/// Get the list of misc items that are selected by the user
pub fn get_selected_misc_items(app: &AppHandle) -> Result<Vec<MiscItem>, String> {
    println!("=== DEBUG: Getting selected misc items ===");
    
    // First, get all available misc items
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let misc_items_dir = app_data_dir.join("misc_items");
    let misc_items_file = misc_items_dir.join("misc_items.json");
    
    if !misc_items_file.exists() {
        println!("DEBUG: misc_items.json does not exist");
        return Ok(Vec::new());
    }
    
    // Read all available misc items
    let content = fs::read_to_string(&misc_items_file)
        .map_err(|e| format!("Failed to read misc items file: {}", e))?;
    let all_misc_items: Vec<MiscItem> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse misc items: {}", e))?;
    
    println!("DEBUG: Found {} total misc items", all_misc_items.len());
    
    // Read the config to get selected item IDs
    let config_dir = app_data_dir.join("config");
    let config_file = config_dir.join("config.json");
    
    if !config_file.exists() {
        println!("DEBUG: config.json does not exist, no selections");
        // No config file means no selections
        return Ok(Vec::new());
    }
    
    let config_content = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
        
    let config: SavedConfig = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    
    println!("DEBUG: Parsed config, selected_misc_items: {:?}", config.selected_misc_items);
    
    // Filter misc items based on selections
    let mut selected_items = Vec::new();
    
    for (item_type, selected_ids) in &config.selected_misc_items {
        println!("DEBUG: Processing type '{}' with {} selected IDs: {:?}", item_type, selected_ids.len(), selected_ids);
        for selected_id in selected_ids {
            // Handle built-in font IDs (e.g., "builtin-font-korean") by mapping to resource fonts
            if selected_id.starts_with("builtin-font-") && item_type == "font" {
                // Determine the builtin name suffix
                if let Some(suffix) = selected_id.strip_prefix("builtin-font-") {
                    // Resource directory where bundled fonts may live. Try several common locations to
                    // handle differences between dev (crate resources) and packaged layouts.
                    if let Ok(resource_dir) = app.path().resource_dir() {
                        // Build a list of candidate font paths to check (dev vs packaged layouts)
                        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
                        candidates.push(resource_dir.join("fonts").join(format!("{}.skin_file", suffix)));
                        candidates.push(resource_dir.join("resources").join("fonts").join(format!("{}.skin_file", suffix)));
                        candidates.push(resource_dir.join("..").join("resources").join("fonts").join(format!("{}.skin_file", suffix)));

                        // Also check the crate's resources folder (useful during cargo run from workspace)
                        let manifest_fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("fonts").join(format!("{}.skin_file", suffix));
                        candidates.push(manifest_fonts);

                        // Find first existing candidate
                        let mut found_candidate: Option<std::path::PathBuf> = None;
                        for cand in candidates.iter() {
                            if cand.exists() {
                                found_candidate = Some(cand.clone());
                                break;
                            }
                        }

                        if let Some(candidate) = found_candidate {
                            // Ensure misc_items_dir exists
                            let _ = std::fs::create_dir_all(&misc_items_dir);

                            // Destination filename in misc_items dir
                            let dest_filename = format!("font_builtin_{}.skin_file", suffix);
                            let dest_path = misc_items_dir.join(&dest_filename);

                            // Copy if not already present
                            if !dest_path.exists() {
                                if let Err(e) = std::fs::copy(&candidate, &dest_path) {
                                    println!("DEBUG: Failed to copy builtin font {} to misc_items: {}", candidate.display(), e);
                                } else {
                                    println!("DEBUG: Copied builtin font {} -> {}", candidate.display(), dest_path.display());
                                }
                            } else {
                                println!("DEBUG: Builtin font already copied: {}", dest_path.display());
                            }

                            // Construct a MiscItem entry matching expectations of injector
                            let builtin_misc = crate::injection::MiscItem {
                                id: selected_id.clone(),
                                name: suffix.to_string(),
                                item_type: item_type.clone(),
                                skin_file_path: dest_filename,
                            };
                            println!("DEBUG: Adding builtin selected item: {} ({})", builtin_misc.name, builtin_misc.id);
                            selected_items.push(builtin_misc);
                            continue;
                        } else {
                            // Log the primary candidate path for debugging
                            let primary = resource_dir.join("fonts").join(format!("{}.skin_file", suffix));
                            println!("DEBUG: Builtin font resource not found in known locations, primary checked: {}", primary.display());
                        }
                    } else {
                        println!("DEBUG: resource_dir unavailable; cannot resolve builtin font {}", selected_id);
                    }
                }
            }

            // Fallback: match against uploaded/available misc items
            for misc_item in &all_misc_items {
                if misc_item.item_type == *item_type && selected_ids.contains(&misc_item.id) {
                    println!("DEBUG: Adding selected item: {} ({})", misc_item.name, misc_item.id);
                    selected_items.push(misc_item.clone());
                }
            }
        }
    }
    
    println!("DEBUG: Found {} selected misc items total", selected_items.len());
    Ok(selected_items)
}
