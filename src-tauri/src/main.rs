// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod injection;

use commands::*;
use tauri::{Manager};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use tauri::menu::{Menu, MenuItem};

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
            select_league_directory,
            inject_skins,
            inject_game_skins,
            save_league_path,
            load_league_path,
            save_selected_skins,
            start_auto_inject,
            load_config,
            delete_champions_cache,
            auto_detect_league,
            
            // GitHub update commands
            check_github_updates,
            pull_github_updates,
            update_champion_data_from_github,
            
            // custom skin commands
            upload_custom_skin,
            get_custom_skins,
            delete_custom_skin,
            
            // LCU API and party mode commands
            get_lcu_friends,
            send_lcu_message,
            get_lcu_messages,
            start_lcu_watcher,
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
