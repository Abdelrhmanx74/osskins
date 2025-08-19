use tauri::Manager;

#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) -> Result<(), String> {
    // Use the app handle to exit the application immediately
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|e| format!("Failed to hide window: {}", e))?;
    }
    Ok(())
}
