use tauri::{Manager};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use serde_json;
use crate::commands::types::CustomSkinData;
use crate::commands::champion_data::get_champion_name;

// Custom skin upload and management

#[tauri::command]
pub async fn upload_custom_skin(
    app: tauri::AppHandle,
    champion_id: u32,
    skin_name: String,
) -> Result<CustomSkinData, String> {
    println!("Uploading custom skin: {}", skin_name);
    println!("For champion ID: {}", champion_id);
    
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
    let champion_name = if let Ok(champion_data) = get_champion_name(&app, champion_id).await {
        champion_data
    } else {
        format!("champion_{}", champion_id) // Fallback if name not found
    };
    
    // Create directory for this champion's custom skins
    let champion_dir = custom_skins_dir.join(&champion_name);
    std::fs::create_dir_all(&champion_dir)
        .map_err(|e| format!("Failed to create champion directory: {}", e))?;
        
    // Generate a unique ID for this skin
    let skin_id = format!("custom_{}_{}", champion_id, chrono::Utc::now().timestamp());
    
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
        name: skin_name,
        champion_id: champion_id,
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

#[tauri::command]
pub async fn upload_multiple_custom_skins(
    app: tauri::AppHandle,
    champion_id: u32,
) -> Result<Vec<CustomSkinData>, String> {
    println!("Uploading multiple custom skins for champion ID: {}", champion_id);
    
    // Show file dialog for multiple file selection
    let files = rfd::FileDialog::new()
        .add_filter("Skin files", &["fantome", "wad", "client", "zip"])
        .set_title("Select Custom Skin Files")
        .pick_files()
        .ok_or("No files selected")?;

    if files.is_empty() {
        return Err("No files selected".to_string());
    }

    // Get app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Create custom_skins directory if it doesn't exist
    let custom_skins_dir = app_data_dir.join("custom_skins");
    std::fs::create_dir_all(&custom_skins_dir)
        .map_err(|e| format!("Failed to create custom skins directory: {}", e))?;

    let mut uploaded_skins = Vec::new();

    // Process each selected file
    for file_path in files {
        let file_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");

        // Generate unique ID
        let skin_id = format!("custom_{}_{}_{}",
            champion_id,
            file_name.replace(' ', "_"),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        // Get file extension
        let file_extension = file_path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("fantome");

        // Create safe filename
        let safe_name = file_name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
            .collect::<String>();
        let dest_filename = format!("{}_{}.{}", champion_id, safe_name, file_extension);
        let dest_path = custom_skins_dir.join(&dest_filename);

        // Copy the selected file to the custom skins directory
        std::fs::copy(&file_path, &dest_path)
            .map_err(|e| format!("Failed to copy custom skin file {}: {}", file_name, e))?;

        // Get champion name - use a simple fallback since this is in a synchronous context
        let champion_name = format!("Champion {}", champion_id);

        // Create custom skin entry
        let custom_skin = CustomSkinData {
            id: skin_id,
            name: safe_name,
            champion_id,
            champion_name,
            file_path: dest_path.to_string_lossy().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            preview_image: None,
        };

        // Save the custom skin
        save_custom_skin(&app, &custom_skin).await?;
        uploaded_skins.push(custom_skin);
    }

    println!("Successfully uploaded {} custom skins for champion {}", uploaded_skins.len(), champion_id);
    Ok(uploaded_skins)
}