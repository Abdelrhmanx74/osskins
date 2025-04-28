// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod injection;

use commands::*;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Native injection is now used - no need to check for mod-tools.exe
            // Preload overlays during startup for better performance
            #[cfg(not(debug_assertions))]
            {
                // Get the app_handle BEFORE spawning the thread
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    // We spawn a background thread to preload resources
                    // This prevents blocking the UI during startup
                    let _ = commands::preload_resources(&app_handle);
                });
            }
            
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_champions_data,
            check_data_updates,
            get_champion_data,
            update_champion_data,
            save_fantome_file,
            select_league_directory,
            inject_skins,
            ensure_mod_tools,
            inject_game_skins,
            save_league_path,
            load_league_path,
            save_selected_skins,
            start_auto_inject,
            load_config,
            delete_champions_cache,
            auto_detect_league,
            
            // custom skin commands
            upload_custom_skin,
            get_custom_skins,
            delete_custom_skin,
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
