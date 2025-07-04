use tauri::{AppHandle, Manager};
use std::fs;

// File operations for fantome and ZIP files

#[tauri::command]
/// Save a skin ZIP file to the champions directory
pub async fn save_zip_file(
    app: tauri::AppHandle,
    champion_name: String,
    file_name: String,
    content: Vec<u8>,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let champions_dir = app_data_dir.join("champions");
    std::fs::create_dir_all(&champions_dir)
        .map_err(|e| format!("Failed to create champions directory: {}", e))?;

    let champion_dir = champions_dir.join(&champion_name);
    std::fs::create_dir_all(&champion_dir)
        .map_err(|e| format!("Failed to create champion directory: {}", e))?;

    let zip_path = champion_dir.join(&file_name);
    if let Some(parent) = zip_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    std::fs::write(&zip_path, &content)
        .map_err(|e| format!("Failed to write ZIP file: {}", e))?;

    Ok(())
}