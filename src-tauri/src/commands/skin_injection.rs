use crate::commands::config::save_league_path;
use crate::commands::lcu_watcher::start_lcu_watcher;
use crate::commands::types::{SavedConfig, SkinData, SkinInjectionRequest};
use crate::commands::{ensure_mod_tools, load_league_path};
use crate::injection::{inject_skins as inject_skins_impl, inject_skins_and_misc, MiscItem, Skin};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InjectionStatusValue {
  Idle,
  Injecting,  // Legacy/Compatibility
  Busy,       // Processing/preparing mods (like cslol StateBusy)
  Running,    // Overlay is running, waiting for game (like cslol StateRunning)
  Patching,   // Actively patching the game
  Success,    // Injection completed for this game
  Error,
}

impl InjectionStatusValue {
  fn as_str(&self) -> &'static str {
    match self {
      InjectionStatusValue::Idle => "idle",
      InjectionStatusValue::Injecting => "injecting",
      InjectionStatusValue::Busy => "busy",
      InjectionStatusValue::Running => "running",
      InjectionStatusValue::Patching => "patching",
      InjectionStatusValue::Success => "success",
      InjectionStatusValue::Error => "error",
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InjectionStateSnapshot {
  pub status: InjectionStatusValue,
  pub status_message: Option<String>,  // Detailed status like "Waiting for league match to start"
  pub last_error: Option<String>,
  pub updated_at_ms: u64,
}

static INJECTION_STATE: Lazy<RwLock<InjectionStateSnapshot>> = Lazy::new(|| RwLock::new(
  InjectionStateSnapshot {
    status: InjectionStatusValue::Idle,
    status_message: None,
    last_error: None,
    updated_at_ms: current_millis(),
  },
));

fn current_millis() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0)
}

pub fn record_injection_state(
  app: &AppHandle,
  status: InjectionStatusValue,
  error: Option<String>,
) {
  record_injection_state_with_message(app, status, None, error);
}

pub fn record_injection_state_with_message(
  app: &AppHandle,
  status: InjectionStatusValue,
  message: Option<String>,
  error: Option<String>,
) {
  {
    let mut guard = INJECTION_STATE
      .write()
      .expect("INJECTION_STATE poisoned");
    guard.status = status;
    guard.status_message = message.clone();
    guard.last_error = error.clone();
    guard.updated_at_ms = current_millis();
  }

  // Emit detailed status with message
  #[derive(Serialize, Clone)]
  struct StatusPayload {
    status: &'static str,
    message: Option<String>,
  }
  let _ = app.emit("injection-status", StatusPayload {
    status: status.as_str(),
    message: message.clone(),
  });

  // Also emit legacy status string for backwards compatibility
  let _ = app.emit("injection-status-simple", status.as_str());

  if let Some(err) = error {
    let _ = app.emit("skin-injection-error", err);
  }
}

#[tauri::command]
pub fn get_injection_state() -> InjectionStateSnapshot {
  INJECTION_STATE
    .read()
    .expect("INJECTION_STATE poisoned")
    .clone()
}

pub fn get_injection_status() -> InjectionStatusValue {
  INJECTION_STATE
    .read()
    .expect("INJECTION_STATE poisoned")
    .status
}

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
  record_injection_state(&app, InjectionStatusValue::Injecting, None);

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
      record_injection_state_with_message(
        &app,
        InjectionStatusValue::Running,
        Some("Waiting for league match to start".to_string()),
        None,
      );
      Ok(())
    }
    Err(err) => {
      println!("Skin injection failed: {}", err);
      let message = format!("Injection failed: {}", err);
      record_injection_state(&app, InjectionStatusValue::Error, Some(message.clone()));
      Err(message)
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
  record_injection_state(&app_handle, InjectionStatusValue::Injecting, None);

  // Validate game path exists
  if !Path::new(&game_path).exists() {
    let message = format!("League of Legends directory not found: {}", game_path);
    record_injection_state(&app_handle, InjectionStatusValue::Error, Some(message.clone()));
    return Err(message);
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
      let message = format!("Failed to create skin_file directory: {}", e);
      record_injection_state(&app_handle, InjectionStatusValue::Error, Some(message.clone()));
      message
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
      record_injection_state_with_message(
        &app_handle,
        InjectionStatusValue::Running,
        Some("Waiting for league match to start".to_string()),
        None,
      );
      Ok("Skin injection completed successfully".to_string())
    }
    Err(e) => {
      println!("Skin injection failed: {}", e);
      let message = format!("Skin injection failed: {}", e);
      record_injection_state(
        &app_handle,
        InjectionStatusValue::Error,
        Some(message.clone()),
      );
      Err(message)
    }
  };

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
  record_injection_state(&app_handle, InjectionStatusValue::Injecting, None);

  // Validate game path exists
  if !Path::new(&game_path).exists() {
    let message = format!("League of Legends directory not found: {}", game_path);
    record_injection_state(&app_handle, InjectionStatusValue::Error, Some(message.clone()));
    return Err(message);
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
      let message = format!("Failed to create skin_file directory: {}", e);
      record_injection_state(&app_handle, InjectionStatusValue::Error, Some(message.clone()));
      message
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
      record_injection_state_with_message(
        &app_handle,
        InjectionStatusValue::Running,
        Some("Waiting for league match to start".to_string()),
        None,
      );
      Ok("Enhanced skin injection completed successfully".to_string())
    }
    Err(e) => {
      println!("Enhanced skin injection failed: {}", e);
      let message = format!("Enhanced skin injection failed: {}", e);
      record_injection_state(
        &app_handle,
        InjectionStatusValue::Error,
        Some(message.clone()),
      );
      Err(message)
    }
  };

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
  record_injection_state(&app, InjectionStatusValue::Injecting, None);

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
      record_injection_state_with_message(
        &app,
        InjectionStatusValue::Running,
        Some("Waiting for league match to start".to_string()),
        None,
      );
      println!(
        "Successfully injected {} skins and {} misc items",
        skins.len(),
        misc_items.len()
      );
      Ok(())
    }
    Err(e) => {
      let message = e.to_string();
      record_injection_state(
        &app,
        InjectionStatusValue::Error,
        Some(message.clone()),
      );
      Err(message)
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

fn run_mod_tools_warmup(
  mod_tools_path: &Path,
  game_dir: &Path,
  mods_dir: &Path,
  overlay_dir: &Path,
) -> Result<(), String> {
  // Best-effort warmup: build a minimal overlay to warm caches and AV
  // This pre-warms Windows Defender by exercising both mkoverlay and runoverlay
  println!("[Warmup] Starting mod-tools warmup...");

  if overlay_dir.exists() {
    if let Err(e) = fs::remove_dir_all(overlay_dir) {
      println!("[Warmup] Failed to clear warmup overlay dir: {}", e);
    }
  }
  fs::create_dir_all(overlay_dir)
    .map_err(|e| format!("[Warmup] Failed to create overlay dir: {}", e))?;

  if !mods_dir.exists() {
    fs::create_dir_all(mods_dir)
      .map_err(|e| format!("[Warmup] Failed to create mods dir: {}", e))?;
  }

  let game_dir_str = game_dir
    .to_str()
    .ok_or_else(|| "Invalid game path".to_string())?;
  let mods_dir_str = mods_dir
    .to_str()
    .ok_or_else(|| "Invalid mods path".to_string())?;
  let overlay_dir_str = overlay_dir
    .to_str()
    .ok_or_else(|| "Invalid overlay path".to_string())?;

  // Step 1: Run mkoverlay to warm that code path
  let mut cmd = std::process::Command::new(mod_tools_path);
  cmd.args([
    "mkoverlay",
    mods_dir_str,
    overlay_dir_str,
    &format!("--game:{}", game_dir_str),
    "--mods:", // empty mods list just to warm binary and filesystem
    "--noTFT",
    "--ignoreConflict",
  ]);

  #[cfg(target_os = "windows")]
  cmd.creation_flags(crate::injection::core::CREATE_NO_WINDOW);

  match cmd.status() {
    Ok(status) if status.success() => {
      println!("[Warmup] mod-tools mkoverlay warmup succeeded");
    }
    Ok(status) => {
      println!("[Warmup] mod-tools mkoverlay warmup completed with status {}", status);
    }
    Err(e) => {
      println!("[Warmup] Failed to spawn mod-tools mkoverlay for warmup: {}", e);
    }
  }

  // Step 2: Also warm up runoverlay code path (it will exit quickly since game isn't running)
  // This pre-loads the DLL injection code and warms Windows Defender for that path too
  let config_path = overlay_dir.join("warmup-config.json");
  let _ = fs::write(&config_path, r#"{"enableMods":true}"#);

  let mut run_cmd = std::process::Command::new(mod_tools_path);
  run_cmd.args([
    "runoverlay",
    overlay_dir_str,
    config_path.to_str().unwrap_or(""),
    &format!("--game:{}", game_dir_str),
    "--opts:none",
  ]);

  #[cfg(target_os = "windows")]
  run_cmd.creation_flags(crate::injection::core::CREATE_NO_WINDOW);

  // Spawn and immediately try to wait with timeout - runoverlay should exit fast
  // if game isn't running
  match run_cmd.spawn() {
    Ok(mut child) => {
      // Give it a short time to warm up, then kill if still running
      std::thread::sleep(std::time::Duration::from_millis(500));
      let _ = child.kill();
      let _ = child.wait();
      println!("[Warmup] mod-tools runoverlay warmup completed");
    }
    Err(e) => {
      println!("[Warmup] Failed to spawn mod-tools runoverlay for warmup: {}", e);
    }
  }

  // Clean up warmup files
  let _ = fs::remove_dir_all(overlay_dir);

  println!("[Warmup] mod-tools warmup completed");
  Ok(())
}

#[tauri::command]
pub async fn warmup_injection(app: AppHandle, game_path: Option<String>) -> Result<(), String> {
  // Ensure tools exist and load once to warm Windows Defender caches
  let ensured = ensure_mod_tools(app.clone(), Some(false)).await?;

  let league_root = if let Some(path) = game_path {
    path
  } else {
    load_league_path(app.clone()).await.unwrap_or_default()
  };

  if league_root.is_empty() {
    return Ok(());
  }

  let mod_tools_path = match ensured.path {
    Some(p) => PathBuf::from(p),
    None => return Ok(()),
  };

  let game_dir = PathBuf::from(&league_root).join("Game");
  if !game_dir.exists() {
    return Ok(());
  }

  let mods_dir = game_dir.join("mods");
  let overlay_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to resolve app data dir: {}", e))?
    .join("overlay-warmup");

  tokio::task::spawn_blocking(move || {
    run_mod_tools_warmup(&mod_tools_path, &game_dir, &mods_dir, &overlay_dir)
  })
  .await
  .map_err(|e| format!("Warmup task failed: {}", e))??;

  Ok(())
}

// Manual injection mode commands
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// Global state for manual injection
static MANUAL_INJECTION_ACTIVE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static MANUAL_INJECTION_DATA: Lazy<Arc<Mutex<Option<ManualInjectionData>>>> =
  Lazy::new(|| Arc::new(Mutex::new(None)));
static MANUAL_INJECTION_TRIGGERED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

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

  // Allow a fresh trigger for this session
  MANUAL_INJECTION_TRIGGERED.store(false, Ordering::Relaxed);

  // Set manual injection as active
  MANUAL_INJECTION_ACTIVE.store(true, Ordering::Relaxed);

  // Emit event to update UI
  let _ = app.emit("manual-injection-status", "waiting");

  println!("[Manual Injection] Manual injection mode activated - waiting for champion select");

  // If we are already in champ select, trigger immediately
  if crate::commands::lcu_watcher::is_in_champ_select() {
    println!("[Manual Injection] Already in ChampSelect, triggering immediately");
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
      let _ = trigger_manual_injection(&app_clone).await;
    });
  }

  Ok(())
}

// Stop manual injection mode
#[tauri::command]
pub async fn stop_manual_injection(app: AppHandle) -> Result<(), String> {
  println!("[Manual Injection] Stopping manual injection mode");

  // Deactivate manual injection
  MANUAL_INJECTION_ACTIVE.store(false, Ordering::Relaxed);
  MANUAL_INJECTION_TRIGGERED.store(false, Ordering::Relaxed);

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
  record_injection_state(&app, InjectionStatusValue::Idle, None);

  println!("[Manual Injection] Manual injection mode stopped");

  Ok(())
}

// Check if manual injection mode is enabled (from config preference)
// This is separate from is_manual_injection_active() which checks if a manual injection session is running
pub fn is_manual_injection_mode_enabled(app: &AppHandle) -> bool {
  let config_dir = match app.path().app_data_dir() {
    Ok(dir) => dir.join("config"),
    Err(_) => return false,
  };
  let cfg_file = config_dir.join("config.json");
  if let Ok(data) = std::fs::read_to_string(&cfg_file) {
    if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
      if let Some(mode_val) = cfg.get("manual_injection_mode").and_then(|v| v.as_bool()) {
        return mode_val;
      }
    }
  }
  false
}

// Check if manual injection is active (session is running)
pub fn is_manual_injection_active() -> bool {
  MANUAL_INJECTION_ACTIVE.load(Ordering::Relaxed)
}

// Check if manual injection mode is enabled OR if a manual injection session is active
// This is the main check to use when deciding whether to skip automatic injection
pub fn should_skip_automatic_injection(app: &AppHandle) -> bool {
  is_manual_injection_mode_enabled(app) || is_manual_injection_active()
}

// Check if manual injection has been triggered
pub fn is_manual_injection_triggered() -> bool {
  MANUAL_INJECTION_TRIGGERED.load(Ordering::Relaxed)
}

// Trigger manual injection (called by LCU watcher when entering champ select or manually)
pub async fn trigger_manual_injection(app: &AppHandle) -> Result<(), String> {
  println!("[Manual Injection] Triggering manual injection");

  // Check if manual injection is active
  if !MANUAL_INJECTION_ACTIVE.load(Ordering::Relaxed) {
    println!("[Manual Injection] Trigger called but manual injection is NOT active");
    return Ok(());
  }

  // Prevent duplicate triggers within the same manual session
  // UNLESS the process is currently Idle (meaning it died or hasn't started)
  let current_status = get_injection_status();
  if MANUAL_INJECTION_TRIGGERED.load(Ordering::Relaxed) && current_status != InjectionStatusValue::Idle {
    println!("[Manual Injection] Trigger already handled and process is running, skipping");
    return Ok(());
  }

  MANUAL_INJECTION_TRIGGERED.store(true, Ordering::Relaxed);

  println!("[Manual Injection] Processing trigger...");


  // Get the stored injection data
  let data = {
    let guard = MANUAL_INJECTION_DATA.lock().unwrap();
    guard.clone()
  };

  let data = match data {
    Some(d) => d,
    None => {
      println!("[Manual Injection] No injection data found");
      record_injection_state(
        app,
        InjectionStatusValue::Error,
        Some("No injection data found".to_string()),
      );
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
  record_injection_state(app, InjectionStatusValue::Injecting, None);
  let _ = app.emit("manual-injection-status", "injecting");

  // Perform injection - this starts the overlay process which waits for the game
  let result = inject_skins_and_misc(
    app,
    &league_path,
    &data.skins,
    &data.misc_items,
    &skin_file_files_dir,
  );

  match result {
    Ok(_) => {
      // Overlay is now running and waiting for the game to start
      // Status will be updated by the stdout reader thread as runoverlay reports progress
      // Set status to Running (waiting for game) - this may get updated by patcher output
      record_injection_state_with_message(
        app, 
        InjectionStatusValue::Running, 
        Some("Waiting for league match to start".to_string()),
        None
      );
      let _ = app.emit("manual-injection-status", "running");
      println!(
        "[Manual Injection] Overlay started for {} skins and {} misc items - waiting for game",
        data.skins.len(),
        data.misc_items.len()
      );
      Ok(())
    }
    Err(e) => {
      let message = e.to_string();
      record_injection_state(app, InjectionStatusValue::Error, Some(message.clone()));
      let _ = app.emit("manual-injection-status", "error");
      Err(message)
    }
  }
}
