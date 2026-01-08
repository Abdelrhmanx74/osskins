// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod injection;

use commands::*;
use tauri::async_runtime::block_on;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

fn extract_fonts(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  use std::fs;
  use tauri::path::BaseDirectory;

  let app_data_dir = app.path().app_data_dir()?;
  let fonts_dir = app_data_dir.join("fonts");

  // Check if we need to extract
  // For now, we extract if fonts_dir doesn't exist
  if fonts_dir.exists() {
    return Ok(());
  }

  // Try to find the zip file
  // 1. In the resource root (prod)
  let mut resource_path = app.path().resolve("fonts.zip", BaseDirectory::Resource)?;

  if !resource_path.exists() {
    // 2. In resources/fonts.zip (dev?)
    if let Ok(p) = app
      .path()
      .resolve("resources/fonts.zip", BaseDirectory::Resource)
    {
      if p.exists() {
        resource_path = p;
      }
    }
  }

  // 3. Fallback to CARGO_MANIFEST_DIR for dev
  if !resource_path.exists() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    if !manifest_dir.is_empty() {
      let p = std::path::Path::new(&manifest_dir)
        .join("resources")
        .join("fonts.zip");
      if p.exists() {
        resource_path = p;
      }
    }
  }

  if resource_path.exists() {
    fs::create_dir_all(&fonts_dir)?;
    let file = fs::File::open(&resource_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(&fonts_dir)?;
  }
  Ok(())
}

fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .setup(|app| {
      // Initialize ConfigLock
      app.manage(commands::ConfigLock::new());

      // Extract fonts in background
      let app_handle = app.handle().clone();
      std::thread::spawn(move || {
        if let Err(e) = extract_fonts(&app_handle) {
          eprintln!("Failed to extract fonts: {}", e);
        }
      });

      // Native injection is now used - no need to check for mod-tools.exe
      // Preload overlays during startup for better performance
      // (run even in debug so first injection behaves consistently)
      let app_handle_preload = app.handle().clone();
      std::thread::spawn(move || {
        // We spawn a background thread to preload resources
        // This prevents blocking the UI during startup
        let _ = commands::preload_resources(&app_handle_preload);
      });

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
          }
          "exit" => {
            app.exit(0);
          }
          _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
          TrayIconEvent::Click {
            button: MouseButton::Left,
            ..
          } => {
            let app = tray.app_handle();
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.show();
              let _ = window.set_focus();
            }
          }
          _ => {}
        })
        .icon(app.default_window_icon().unwrap().clone())
        .build(app_handle)
        .unwrap();
      // On startup, respect previously saved tray state and hide window if configured.
      let start_hidden = match block_on(get_start_hidden(app_handle.clone())) {
        Ok(v) => v,
        Err(_) => false,
      };
      if start_hidden {
        if let Some(window) = app_handle.get_webview_window("main") {
          let _ = window.hide();
        }
      }
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
      save_skin_file,
      save_zip_file,
      download_and_save_file,
      download_file_to_champion_with_progress,
      cancel_download,
      select_league_directory,
      inject_skins,
      inject_skins_with_misc,
      inject_all_selected_skins,
      warmup_injection,
      ensure_mod_tools,
      extract_sfx,
      inject_game_skins,
      save_league_path,
      load_league_path,
      save_selected_skins,
      start_auto_inject,
      load_config,
      debug_config,
      delete_champions_cache,
      auto_detect_league,
      set_auto_update_data,
      // unified skin commands
      select_skin_for_champion,
      remove_skin_for_champion,
      save_custom_skin,
      get_all_custom_skins,
      // legacy custom skin commands (may be deprecated)
      upload_custom_skin,
      rename_custom_skin,
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
      set_party_mode_verbose_logging,
      get_party_mode_verbose_logging,
      set_party_mode_max_share_age,
      get_party_mode_diagnostic_state,
      resend_skin_to_friends,
      start_party_mode_chat_monitor,
      print_logs,
      // data commit tracking
      set_last_data_commit,
      get_latest_data_commit,
      // manual injection commands
      start_manual_injection,
      stop_manual_injection,
      get_changed_champions_since,
      get_changed_champions_from_config,
      get_changed_skin_files,
      // new ID-based skin download commands
      download_skin_by_id,
      batch_download_skins,
      check_skin_exists,
      get_skin_file_size,
      cancel_batch_download,
      // app control
      exit_app,
      hide_window,
      get_injection_state,
      set_start_hidden,
      get_start_hidden,
      set_manual_injection_mode,
      get_manual_injection_mode,
    ])
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_opener::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
