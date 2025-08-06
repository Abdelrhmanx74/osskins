// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod injection;

use commands::*;
use tauri::Manager;
use tauri::menu::{MenuItem, Menu};
use tauri::tray::{TrayIconBuilder, MouseButton, TrayIconEvent};

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
        
            // Set up system tray
            let app_handle = app.handle();
            // Use with_id for menu items as per docs
            let osskins_item = MenuItem::with_id(app_handle, "osskins", "Osskins", true, None::<&str>)?;
            let exit_item = MenuItem::with_id(app_handle, "exit", "Exit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app_handle, &[&osskins_item, &exit_item])?;
            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "osskins" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    },
                    "exit" => {
                        app.exit(0);
                    },
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click { button: MouseButton::Left, .. } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    },
                    _ => {}
                })
                .icon(app.default_window_icon().unwrap().clone())
                .build(app_handle)
                .unwrap();
            // Listen for window close and hide instead
            if let Some(main_window) = app_handle.get_webview_window("main") {
                main_window.clone().on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_window.hide();
                    }
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
            save_zip_file,
            select_league_directory,
            inject_skins,
            inject_skins_with_misc,
            inject_all_selected_skins,
            ensure_mod_tools,
            inject_game_skins,
            save_league_path,
            load_league_path,
            save_selected_skins,
            start_auto_inject,
            load_config,
            debug_config,
            delete_champions_cache,
            auto_detect_league,
            
            // unified skin commands
            select_skin_for_champion,
            remove_skin_for_champion,
            save_custom_skin,
            get_all_custom_skins,
            
            // legacy custom skin commands (may be deprecated)
            upload_custom_skin,
            upload_multiple_custom_skins,
            get_custom_skins,
            delete_custom_skin,
            
            // misc item commands
            upload_misc_item,
            upload_multiple_misc_items,
            get_misc_items,
            delete_misc_item,
            
            // party mode commands
            get_lcu_friends,
            add_party_friend,
            remove_paired_friend,
            get_paired_friends,
            get_party_mode_settings,
            update_party_mode_settings,
            start_party_mode_chat_monitor,
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
