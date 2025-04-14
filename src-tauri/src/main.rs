// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod injection;

use commands::*;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_path::init())
        .invoke_handler(tauri::generate_handler![
            check_champions_data,
            check_data_updates,
            get_champion_data,
            update_champion_data,
            save_fantome_file,
            select_league_directory,
            inject_skins,
            ensure_mod_tools,
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
