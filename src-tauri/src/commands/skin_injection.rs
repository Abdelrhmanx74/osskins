use crate::commands::config::save_league_path;
use crate::commands::lcu_watcher::start_lcu_watcher;
use crate::commands::types::{SavedConfig, SkinData, SkinInjectionRequest};
use crate::injection::{inject_skins as inject_skins_impl, inject_skins_and_misc, MiscItem, Skin};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

// Skin injection related commands

#[tauri::command]
pub fn inject_skins(app: tauri::AppHandle, request: SkinInjectionRequest) -> Result<(), String> {
  println!("Starting skin injection process");
  println!("League path: {}", request.league_path);
  println!("Number of skins to inject: {}", request.skins.len());

  // Get the app data directory (where champion data is stored)
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;

  // Get the path to the champions directory where skin_file files are stored
  let skin_file_files_dir = app_data_dir.join("champions");
  println!("Fantome files directory: {}", skin_file_files_dir.display());

  // Emit injection started event to update UI
  let _ = app.emit("injection-status", "injecting");

  // Call the native Rust implementation of skin injection using our new SkinInjector
  let result = inject_skins_impl(
    &app,
    &request.league_path,
    &request.skins,
    &skin_file_files_dir,
  );

  // Handle result with proper error propagation to frontend
  match result {
    Ok(_) => {
      println!("Skin injection completed successfully");
      let _ = app.emit("injection-status", "success");
      Ok(())
    }
    Err(err) => {
      println!("Skin injection failed: {}", err);
      let _ = app.emit("injection-status", "error");
      let _ = app.emit("skin-injection-error", format!("Injection failed: {}", err));
      Err(format!("Injection failed: {}", err))
    }
  }
}

#[tauri::command]
pub async fn inject_game_skins(
  app_handle: AppHandle,
  game_path: String,
  skins: Vec<SkinData>,
  skin_file_files_dir: String,
) -> Result<String, String> {
  println!("Starting skin injection process");
  println!("League path: {}", game_path);
  println!("Number of skins to inject: {}", skins.len());
  println!("Fantome files directory: {}", skin_file_files_dir);

  // Emit injection started event
  let _ = app_handle.emit("injection-status", true);

  // Validate game path exists
  if !Path::new(&game_path).exists() {
    let _ = app_handle.emit("injection-status", false);
    return Err(format!(
      "League of Legends directory not found: {}",
      game_path
    ));
  }

  // Validate skin_file directory exists
  let base_path = Path::new(&skin_file_files_dir);
  if !base_path.exists() {
    // Create the directory if it doesn't exist
    println!(
      "Creating skin_file files directory: {}",
      base_path.display()
    );
    fs::create_dir_all(base_path).map_err(|e| {
      let _ = app_handle.emit("injection-status", false);
      format!("Failed to create skin_file directory: {}", e)
    })?;
  }

  // Save the league path for future use
  save_league_path(app_handle.clone(), game_path.clone()).await?;

  // Convert SkinData to the internal Skin type
  let internal_skins: Vec<Skin> = skins
    .iter()
    .map(|s| Skin {
      champion_id: s.champion_id,
      skin_id: s.skin_id,
      chroma_id: s.chroma_id,
      skin_file_path: s.skin_file.clone(),
    })
    .collect();

  // Call the injection function
  let result = match inject_skins_impl(&app_handle, &game_path, &internal_skins, base_path) {
    Ok(_) => {
      println!("Skin injection completed successfully");
      Ok("Skin injection completed successfully".to_string())
    }
    Err(e) => {
      println!("Skin injection failed: {}", e);
      Err(format!("Skin injection failed: {}", e))
    }
  };

  // Always emit injection ended event, regardless of success/failure
  let _ = app_handle.emit("injection-status", false);

  result
}

// Enhanced injection command that supports both skins and misc items
#[tauri::command]
pub async fn inject_skins_with_misc(
  app_handle: AppHandle,
  game_path: String,
  skins: Vec<SkinData>,
  misc_items: Vec<MiscItem>,
  skin_file_files_dir: String,
) -> Result<String, String> {
  println!("Starting enhanced skin injection process");
  println!("League path: {}", game_path);
  println!("Number of skins to inject: {}", skins.len());
  println!("Number of misc items to inject: {}", misc_items.len());

  // Emit injection started event
  let _ = app_handle.emit("injection-status", true);

  // Validate game path exists
  if !Path::new(&game_path).exists() {
    let _ = app_handle.emit("injection-status", false);
    return Err(format!(
      "League of Legends directory not found: {}",
      game_path
    ));
  }

  // Validate skin_file directory exists
  let base_path = Path::new(&skin_file_files_dir);
  if !base_path.exists() {
    // Create the directory if it doesn't exist
    println!(
      "Creating skin_file files directory: {}",
      base_path.display()
    );
    fs::create_dir_all(base_path).map_err(|e| {
      let _ = app_handle.emit("injection-status", false);
      format!("Failed to create skin_file directory: {}", e)
    })?;
  }

  // Save the league path for future use
  save_league_path(app_handle.clone(), game_path.clone()).await?;

  // Convert SkinData to the internal Skin type
  let internal_skins: Vec<Skin> = skins
    .iter()
    .map(|s| Skin {
      champion_id: s.champion_id,
      skin_id: s.skin_id,
      chroma_id: s.chroma_id,
      skin_file_path: s.skin_file.clone(),
    })
    .collect();

  // Call the enhanced injection function
  let result = match inject_skins_and_misc(
    &app_handle,
    &game_path,
    &internal_skins,
    &misc_items,
    base_path,
  ) {
    Ok(_) => {
      println!("Enhanced skin injection completed successfully");
      Ok("Enhanced skin injection completed successfully".to_string())
    }
    Err(e) => {
      println!("Enhanced skin injection failed: {}", e);
      Err(format!("Enhanced skin injection failed: {}", e))
    }
  };

  // Always emit injection ended event, regardless of success/failure
  let _ = app_handle.emit("injection-status", false);

  result
}

#[tauri::command]
pub async fn start_auto_inject(app: AppHandle, league_path: String) -> Result<(), String> {
  println!("Starting auto-inject for path: {}", league_path);

  // Start the LCU watcher in a separate thread
  start_lcu_watcher(app, league_path)?;

  Ok(())
}

// Helper function to convert SavedConfig to injection-ready skins
pub fn get_all_skins_for_injection(config: &SavedConfig) -> Vec<Skin> {
  let mut all_skins = Vec::new();

  // Add official skins
  for skin_data in &config.skins {
    all_skins.push(Skin {
      champion_id: skin_data.champion_id,
      skin_id: skin_data.skin_id,
      chroma_id: skin_data.chroma_id,
      skin_file_path: skin_data.skin_file.clone(),
    });
  }

  // Add custom skins (with skin_id = 0 and file path as skin_file_path)
  for custom_skin in &config.custom_skins {
    all_skins.push(Skin {
      champion_id: custom_skin.champion_id,
      skin_id: 0,      // Custom skins use skin_id 0
      chroma_id: None, // Custom skins don't have chromas
      skin_file_path: Some(custom_skin.file_path.clone()),
    });
  }

  all_skins
}

// Command to inject all selected skins from config
#[tauri::command]
pub async fn inject_all_selected_skins(app: AppHandle) -> Result<(), String> {
  // Load config
  let config = crate::commands::config::load_config(app.clone()).await?;

  // Get league path
  let league_path = config
    .league_path
    .clone()
    .ok_or("No League path configured")?;

  // Convert config to injection-ready skins
  let skins = get_all_skins_for_injection(&config);

  if skins.is_empty() {
    return Err("No skins selected for injection".to_string());
  }

  // Get the app data directory for skin_file files
  let app_data_dir = app
    .path()
    .app_data_dir()
    .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
  let skin_file_files_dir = app_data_dir.join("champions");

  // Get misc items
  let misc_items =
    crate::commands::misc_items::get_selected_misc_items(&app).unwrap_or_else(|_| Vec::new());

  // Emit injection started event
  let _ = app.emit("injection-status", "injecting");

  // Perform injection
  let result = inject_skins_and_misc(
    &app,
    &league_path,
    &skins,
    &misc_items,
    &skin_file_files_dir,
  );

  match result {
    Ok(_) => {
      let _ = app.emit("injection-status", "success");
      println!(
        "Successfully injected {} skins and {} misc items",
        skins.len(),
        misc_items.len()
      );
      Ok(())
    }
    Err(e) => {
      let _ = app.emit("injection-status", "error");
      Err(e.to_string())
    }
  }
}

// Preload resources function to improve first-time injection speed
#[allow(dead_code)]
pub fn preload_resources(_app_handle: &tauri::AppHandle) -> Result<(), String> {
  // Preloading is disabled - no caching or fallback logic
  println!("Preloading disabled - using direct file access only");
  Ok(())
}

// Manual injection mode commands
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// Global state for manual injection
static MANUAL_INJECTION_ACTIVE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static MANUAL_INJECTION_DATA: Lazy<Arc<Mutex<Option<ManualInjectionData>>>> =
  Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Clone, Debug)]
struct ManualInjectionData {
  skins: Vec<Skin>,
  misc_items: Vec<MiscItem>,
}

// Start manual injection mode - stores the selected skins and waits for champ select
#[tauri::command]
pub async fn start_manual_injection(
  app: AppHandle,
  skins: Vec<SkinData>,
  misc_items: Vec<MiscItem>,
) -> Result<(), String> {
  println!("[Manual Injection] Starting manual injection mode");
  println!("[Manual Injection] Skins to inject: {}", skins.len());
  println!(
    "[Manual Injection] Misc items to inject: {}",
    misc_items.len()
  );

  // Convert SkinData to internal Skin type
  let internal_skins: Vec<Skin> = skins
    .iter()
    .map(|s| Skin {
      champion_id: s.champion_id,
      skin_id: s.skin_id,
      chroma_id: s.chroma_id,
      skin_file_path: s.skin_file.clone(),
    })
    .collect();

  // Store the injection data
  let data = ManualInjectionData {
    skins: internal_skins,
    misc_items,
  };

  {
    let mut guard = MANUAL_INJECTION_DATA.lock().unwrap();
    *guard = Some(data);
  }

  // Set manual injection as active
  MANUAL_INJECTION_ACTIVE.store(true, Ordering::Relaxed);

  // Emit event to update UI
  let _ = app.emit("manual-injection-status", "waiting");

  println!("[Manual Injection] Manual injection mode activated - waiting for champion select");

  Ok(())
}

// Stop manual injection mode
#[tauri::command]
pub async fn stop_manual_injection(app: AppHandle) -> Result<(), String> {
  println!("[Manual Injection] Stopping manual injection mode");

  // Deactivate manual injection
  MANUAL_INJECTION_ACTIVE.store(false, Ordering::Relaxed);

  // Clear stored data
  {
    let mut guard = MANUAL_INJECTION_DATA.lock().unwrap();
    *guard = None;
  }

  // Clean up any active injection
  let config = crate::commands::config::load_config(app.clone()).await?;
  if let Some(league_path) = config.league_path {
    let _ = crate::injection::cleanup_injection(&app, &league_path);
  }

  // Emit event to update UI
  let _ = app.emit("manual-injection-status", "stopped");
  let _ = app.emit("injection-status", "idle");

  println!("[Manual Injection] Manual injection mode stopped");

  Ok(())
}

// Check if manual injection is active
pub fn is_manual_injection_active() -> bool {
  MANUAL_INJECTION_ACTIVE.load(Ordering::Relaxed)
}

// Trigger manual injection (called by LCU watcher when entering champ select or manually)
pub async fn trigger_manual_injection(app: &AppHandle) -> Result<(), String> {
  println!("[Manual Injection] Triggering manual injection");

  // Check if manual injection is active
  if !MANUAL_INJECTION_ACTIVE.load(Ordering::Relaxed) {
    return Ok(());
  }

  // Get the stored injection data
  let data = {
    let guard = MANUAL_INJECTION_DATA.lock().unwrap();
    guard.clone()
  };

  let data = match data {
    Some(d) => d,
    None => {
      println!("[Manual Injection] No injection data found");
      return Err("No injection data found".to_string());
    }
  };

  // Get league path from config
  let config = crate::commands::config::load_config(app.clone()).await?;
  let league_path = config.league_path.ok_or("No League path configured")?;

  // Get skin_file files directory
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data directory: {}", e))?;
  let skin_file_files_dir = app_data_dir.join("champions");

  // Emit injection started event
  let _ = app.emit("injection-status", "injecting");
  let _ = app.emit("manual-injection-status", "injecting");

  // Perform injection
  let result = inject_skins_and_misc(
    app,
    &league_path,
    &data.skins,
    &data.misc_items,
    &skin_file_files_dir,
  );

  match result {
    Ok(_) => {
      let _ = app.emit("injection-status", "success");
      let _ = app.emit("manual-injection-status", "success");
      println!(
        "[Manual Injection] Successfully injected {} skins and {} misc items",
        data.skins.len(),
        data.misc_items.len()
      );
      Ok(())
    }
    Err(e) => {
      let _ = app.emit("injection-status", "error");
      let _ = app.emit("manual-injection-status", "error");
      Err(e.to_string())
    }
  }
}
