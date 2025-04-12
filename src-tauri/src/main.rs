// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{
    check_data_updates,
    update_champion_data,
    save_fantome_file,
    get_champion_data,
    check_champions_data,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            check_data_updates,
            update_champion_data,
            save_fantome_file,
            get_champion_data,
            check_champions_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
