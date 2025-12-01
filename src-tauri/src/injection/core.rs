use crate::injection::error::{InjectionError, MiscItem, ModState, Skin};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Main skin injector class - simplified without profiles
pub struct SkinInjector {
  pub(crate) state: ModState,
  pub(crate) app_dir: PathBuf,
  #[allow(dead_code)]
  pub(crate) root_path: PathBuf, // Store the root League directory path
  pub(crate) game_path: PathBuf, // Store the Game subdirectory path
  pub(crate) status: String,
  pub(crate) log_file: Option<File>,
  pub(crate) mod_tools_path: Option<PathBuf>, // Add mod_tools path
  #[allow(dead_code)]
  pub(crate) champion_names: HashMap<u32, String>, // Keep for compatibility but not used actively
  pub(crate) app_handle: Option<AppHandle>,
  // Stored handle for the spawned overlay process so we can stop it cleanly
  pub(crate) overlay_process: Option<std::process::Child>,
}

impl SkinInjector {
  pub fn new(app_handle: &AppHandle, root_path: &str) -> Result<Self, InjectionError> {
    // Get the app directory
    let app_dir = app_handle.path().app_data_dir().map_err(|e| {
      InjectionError::IoError(io::Error::new(io::ErrorKind::NotFound, format!("{}", e)))
    })?;

    // Store both root and game paths
    let root_path = PathBuf::from(root_path);
    let game_path = root_path.join("Game");

    // Validate game path
    if !game_path.join("League of Legends.exe").exists() {
      return Err(InjectionError::InvalidGamePath(
        "Game\\League of Legends.exe not found".into(),
      ));
    }

    // Create directories needed
    fs::create_dir_all(app_dir.join("mods"))?;
    fs::create_dir_all(app_dir.join("temp"))?;

    // Create log file
    let log_path = app_dir.join("log.txt");
    let log_file = File::create(&log_path)?;

    // Initialize empty champion names cache
    let champion_names = HashMap::new();

    // Look for mod-tools executable in multiple locations
    let mut mod_tools_path = None;

    // Prefer tools downloaded/managed by the app in app data directories

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
      let candidate = app_data_dir.join("cslol-tools").join("mod-tools.exe");

      if candidate.exists() {
        mod_tools_path = Some(candidate);
      }
    }

    // Tauri's app-local directory is used by some installers; check there as well
    if mod_tools_path.is_none() {
      if let Ok(app_local_dir) = app_handle.path().app_local_data_dir() {
        let managed = app_local_dir.join("cslol-tools").join("mod-tools.exe");

        if managed.exists() {
          mod_tools_path = Some(managed);
        }
      }
    }

    // Legacy installers might have dropped mod-tools.exe directly in app-local data
    if mod_tools_path.is_none() {
      if let Ok(app_local_dir) = app_handle.path().app_local_data_dir() {
        let legacy_single = app_local_dir.join("mod-tools.exe");
        if legacy_single.exists() {
          mod_tools_path = Some(legacy_single);
        }
      }
    }

    // Fall back to bundled resources shipped with the app
    if mod_tools_path.is_none() {
      if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let direct = resource_dir.join("mod-tools.exe");
        if direct.exists() {
          mod_tools_path = Some(direct);
        }
        if mod_tools_path.is_none() {
          let bundled = resource_dir.join("cslol-tools").join("mod-tools.exe");
          if bundled.exists() {
            mod_tools_path = Some(bundled);
          }
        }
      }
    }

    // Finally search near the running executable (useful in dev environments)
    if mod_tools_path.is_none() {
      if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
          let candidates = [
            exe_dir.join("cslol-tools").join("mod-tools.exe"),
            exe_dir
              .join("resources")
              .join("cslol-tools")
              .join("mod-tools.exe"),
            exe_dir.join("mod-tools.exe"),
          ];
          for candidate in &candidates {
            if candidate.exists() {
              mod_tools_path = Some(candidate.clone());

              break;
            }
          }
        }
      }
    }

    Ok(Self {
      state: ModState::Uninitialized,
      app_dir,
      root_path,
      game_path,
      status: String::new(),
      log_file: Some(log_file),
      mod_tools_path,
      champion_names,
      app_handle: Some(app_handle.clone()),
      overlay_process: None,
    })
  }

  // Initialize the injector - sets up necessary state
  pub(crate) fn initialize(&mut self) -> Result<(), InjectionError> {
    self.log("Initializing skin injector...");

    // Set state to Idle (ready to inject)
    self.set_state(ModState::Idle);

    // Load champion names if available
    if let Err(e) = self.load_champion_names() {
      self.log(&format!("Warning: Could not load champion names: {}", e));
    }

    self.log("Skin injector initialized successfully");
    Ok(())
  }

  // Load champion names from CDN
  fn load_champion_names(&mut self) -> Result<(), InjectionError> {
    // Moved to champion_data.rs module
    // This is a placeholder - actual implementation is in the champion_data module
    Ok(())
  }

  pub(crate) fn log(&mut self, message: &str) {
    // Add emoji based on message content
    let emoji_message = if message.contains("Initializing") {
      format!("🔄 {}", message)
    } else if message.contains("State changed to") {
      if message.contains("Busy") {
        format!("⏳ {}", message)
      } else if message.contains("Idle") {
        format!("💤 {}", message)
      } else if message.contains("Running") {
        format!("▶️ {}", message)
      } else {
        format!("ℹ️ {}", message)
      }
    } else if message.contains("Starting skin injection") {
      format!("🚀 {}", message)
    } else if message.contains("Stopping skin injection") {
      format!("🛑 {}", message)
    } else if message.contains("Skin injection stopped") {
      format!("⏹️ {}", message)
    } else if message.contains("Cleaning up") {
      format!("🧹 {}", message)
    } else if message.contains("Processing skin") {
      format!("⚙️ {}", message)
    } else if message.contains("Found skin_file file") {
      format!("📂 {}", message)
    } else if message.contains("Processing skin_file file") {
      format!("📦 {}", message)
    } else if message.contains("Extracting skin_file file") {
      format!("📤 {}", message)
    } else if message.contains("Creating mod") {
      format!("🛠️ {}", message)
    } else if message.contains("valid") {
      format!("✅ {}", message)
    } else if message.contains("Copying mod") {
      format!("📋 {}", message)
    } else if message.contains("already has EnableMods=1") {
      format!("✨ {}", message)
    } else if message.contains("Using mod-tools") {
      format!("🔧 {}", message)
    } else if message.contains("overlay") && !message.contains("failed") {
      format!("🔮 {}", message)
    } else if message.contains("succeeded") || message.contains("successfully") {
      format!("✅ {}", message)
    } else if message.contains("failed") || message.contains("error") || message.contains("Error") {
      format!("❌ {}", message)
    } else {
      format!("ℹ️ {}", message)
    };

    // Write to per-injector log file if present (best-effort)
    if let Some(log_file) = &mut self.log_file {
      let _ = writeln!(log_file, "{}", emoji_message);
      let _ = log_file.flush();
    }

    // Also append to global logs so Print Logs captures injection details
    crate::commands::lcu_watcher::append_global_log(&emoji_message);

    // Emit the log to the frontend
    if let Some(app) = &self.app_handle {
      let _ = app.emit("terminal-log", &emoji_message);
    }

    self.status = message.to_string();
  }

  pub(crate) fn set_state(&mut self, new_state: ModState) {
    if self.state != new_state {
      self.state = new_state;
      self.log(&format!("State changed to: {:?}", new_state));
    }
  }

  // Simplified champion name lookup - no longer uses cache or fallback
  #[allow(dead_code)]
  pub(crate) fn get_champion_name(&mut self, _champion_id: u32) -> Option<String> {
    // Champion names are not used in the simplified injection process
    None
  }

  // Functions split across modular files:
  // - skin_file.rs: extract_skin_file,  extract_skin_file_mmap, find_skin_file_for_skin, create_mod_from_extracted, process_skin_file
  // - mod_tools.rs: copy_mod_to_game, run_overlay
  // - game_config.rs: enable_mods_in_game_cfg

  #[allow(dead_code)]
  pub fn inject_skins(
    &mut self,
    skins: &[Skin],
    skin_file_files_dir: &Path,
  ) -> Result<(), InjectionError> {
    self.inject_skins_and_misc(skins, &[], skin_file_files_dir)
  }

  pub fn inject_skins_and_misc(
    &mut self,
    skins: &[Skin],
    misc_items: &[MiscItem],
    skin_file_files_dir: &Path,
  ) -> Result<(), InjectionError> {
    self.inject_skins_and_misc_internal(skins, misc_items, skin_file_files_dir, true)
  }

  pub fn inject_skins_and_misc_no_events(
    &mut self,
    skins: &[Skin],
    misc_items: &[MiscItem],
    skin_file_files_dir: &Path,
  ) -> Result<(), InjectionError> {
    self.inject_skins_and_misc_internal(skins, misc_items, skin_file_files_dir, false)
  }

  fn inject_skins_and_misc_internal(
    &mut self,
    skins: &[Skin],
    misc_items: &[MiscItem],
    skin_file_files_dir: &Path,
    emit_events: bool,
  ) -> Result<(), InjectionError> {
    // Emit start event to frontend (only if requested)
    if emit_events {
      if let Some(app) = &self.app_handle {
        let _ = app.emit("injection-status", "injecting");
      }
    }

    // NOTE: Cleanup is now handled by the LCU watcher on phase changes instead of before each injection
    // This improves performance and is better design

    // Now we can properly initialize for a new injection
    self.set_state(ModState::Busy);
    self.log("Starting skin injection process...");

    // First, clean up the game's mods directory
    let game_mods_dir = self.game_path.join("mods");
    if game_mods_dir.exists() {
      self.log("Cleaning up existing mods in game directory");
      fs::remove_dir_all(&game_mods_dir)?;
    }
    fs::create_dir_all(&game_mods_dir)?;

    // Process each skin
    for (i, skin) in skins.iter().enumerate() {
      self.log(&format!(
        "🔄 Processing skin {}/{}: champion_id={}, skin_id={}, chroma_id={:?}",
        i + 1,
        skins.len(),
        skin.champion_id,
        skin.skin_id,
        skin.chroma_id
      ));
      self.log(&format!(
        "📁 Skin skin_file path: {:?}",
        skin.skin_file_path
      ));

      // Find the skin_file file
      let skin_file_path = self.find_skin_file_for_skin(skin, skin_file_files_dir)?;
      if let Some(skin_file_path) = skin_file_path {
        self.log(&format!(
          "✅ Found skin_file file: {}",
          skin_file_path.display()
        ));

        // Process the skin_file file to create a proper mod structure
        let mod_dir = self.process_skin_file(&skin_file_path)?;

        // Copy the processed mod to the game
        if self.is_valid_mod_dir(&mod_dir) {
          self.log(&format!(
            "✅ Mod structure is valid, copying to game directory for skin {}",
            skin.skin_id
          ));
          self.copy_mod_to_game(&mod_dir)?;
        } else {
          // If processing failed, return error
          self.log("ERROR: Processing failed, mod structure is invalid");
          self.set_state(ModState::Idle);
          return Err(InjectionError::ProcessError(
            "Failed to process skin_file file".into(),
          ));
        }
      } else {
        let msg = format!(
          "No skin_file file found for skin: champion_id={}, skin_id={}, chroma_id={:?}",
          skin.champion_id, skin.skin_id, skin.chroma_id
        );
        self.log(&format!("ERROR: {}", msg));
        self.set_state(ModState::Idle);
        return Err(InjectionError::MissingFantomeFile(msg));
      }
    }

    // Process misc items
    for (i, misc_item) in misc_items.iter().enumerate() {
      self.log(&format!(
        "Processing misc item {}/{}: type={}, name={}",
        i + 1,
        misc_items.len(),
        misc_item.item_type,
        misc_item.name
      ));

      // Find the skin_file file for the misc item in the misc_items directory
      let misc_items_dir = if let Some(app_handle) = &self.app_handle {
        app_handle
          .path()
          .app_data_dir()
          .unwrap_or_else(|_| PathBuf::from("."))
          .join("misc_items")
      } else {
        PathBuf::from(".").join("misc_items")
      };
      let skin_file_path = misc_items_dir.join(&misc_item.skin_file_path);

      self.log(&format!(
        "[DEBUG] Looking for misc item skin_file at: {}",
        skin_file_path.display()
      ));

      if skin_file_path.exists() {
        self.log(&format!(
          "Found misc item skin_file file: {}",
          skin_file_path.display()
        ));

        // Process the skin_file file to create a proper mod structure
        let mod_dir = self.process_skin_file(&skin_file_path)?;

        // Copy the processed mod to the game
        if self.is_valid_mod_dir(&mod_dir) {
          self.log("Misc item mod structure is valid, copying to game directory");
          self.copy_mod_to_game(&mod_dir)?;
        } else {
          self.log("ERROR: Misc item processing failed, mod structure is invalid");
          self.set_state(ModState::Idle);
          return Err(InjectionError::ProcessError(
            "Failed to process misc item skin_file file".into(),
          ));
        }
      } else {
        let msg = format!(
          "No skin_file file found for misc item: {} (looked in {})",
          misc_item.skin_file_path,
          skin_file_path.display()
        );
        self.log(&format!("WARNING: {}", msg));
        // Continue processing other items even if one is missing
      }
    }

    // Enable mods in Game.cfg
    self.enable_mods_in_game_cfg()?;

    // Get the list of mod names we've installed
    let mut mod_names = Vec::new();
    for entry in fs::read_dir(&game_mods_dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.is_dir() && path.join("META").join("info.json").exists() {
        if let Some(name) = path.file_name() {
          if let Some(name_str) = name.to_str() {
            mod_names.push(name_str.to_string());
          } else {
            self.log("WARNING: Could not convert mod directory name to string");
          }
        } else {
          self.log("WARNING: Could not get mod directory file name");
        }
      }
    }

    // Start the overlay process - THIS is the key part that makes skins actually show in-game!
    if let Err(e) = self.run_overlay() {
      self.log(&format!("ERROR: Failed to start overlay process: {}", e));
      self.set_state(ModState::Idle);
      return Err(e);
    }

    self.log("Skin injection completed successfully");
    // Note: We don't set state to Idle because we're now in Running state with the overlay active
    // After all steps complete successfully, emit end event (only if requested)
    if emit_events {
      if let Some(app) = &self.app_handle {
        let _ = app.emit("injection-status", "completed");
      }
    }
    Ok(())
  }

  // Add a cleanup method to stop the injection
  pub fn cleanup(&mut self) -> Result<(), InjectionError> {
    self.log("Stopping skin injection process...");
    // If we have an overlay process that we started, try to terminate it cleanly
    if let Some(mut child) = self.overlay_process.take() {
      self.log("Terminating overlay process started by injector...");
      // Try graceful kill
      let _ = child.kill();
      // Wait briefly for it to exit
      let _ = child.wait();
    }

    // Find and kill the mod-tools processes - more aggressive approach
    #[cfg(target_os = "windows")]
    {
      // First try normal taskkill
      let mut command = std::process::Command::new("taskkill");
      command.args(["/F", "/IM", "mod-tools.exe"]);

      #[cfg(target_os = "windows")]
      const CREATE_NO_WINDOW: u32 = 0x08000000;
      #[cfg(target_os = "windows")]
      command.creation_flags(CREATE_NO_WINDOW);

      let _ = command.output();

      // Then check if any processes are still running with wmic (more reliable)
      let mut check_command = std::process::Command::new("wmic");
      check_command.args([
        "process",
        "where",
        "name='mod-tools.exe'",
        "get",
        "processid",
      ]);
      #[cfg(target_os = "windows")]
      check_command.creation_flags(CREATE_NO_WINDOW);

      // If we find any processes still running, use taskkill with /PID for each one
      if let Ok(output) = check_command.output() {
        if output.status.success() {
          let output_str = String::from_utf8_lossy(&output.stdout);
          for line in output_str.lines() {
            let line = line.trim();
            if line != "ProcessId" && !line.is_empty() && line.chars().all(|c| c.is_digit(10)) {
              // Found a PID, kill it specifically
              let mut kill_pid = std::process::Command::new("taskkill");
              kill_pid.args(["/F", "/PID", line]);
              #[cfg(target_os = "windows")]
              kill_pid.creation_flags(CREATE_NO_WINDOW);
              let _ = kill_pid.output();
            }
          }
        }
      }
    }

    // Clean up the overlay directory
    let overlay_dir = self.app_dir.join("overlay");
    if overlay_dir.exists() {
      // Try multiple times if needed - sometimes Windows file locks take time to release
      for _ in 0..3 {
        match fs::remove_dir_all(&overlay_dir) {
          Ok(_) => break,
          Err(_) => {
            // Sleep briefly to allow file locks to clear
            std::thread::sleep(std::time::Duration::from_millis(100));
          }
        }
      }
    }

    // Reset the state regardless of previous state to ensure cleanup
    self.set_state(ModState::Idle);

    // Emit idle status to frontend so UI updates properly
    if let Some(app) = &self.app_handle {
      let _ = app.emit("injection-status", "idle");
    }

    self.log("Skin injection stopped");

    Ok(())
  }

  // Check if injection cleanup is needed (non-destructive check)
  pub fn needs_cleanup(&self) -> bool {
    // Check if we have running mod-tools processes
    #[cfg(target_os = "windows")]
    {
      let mut check_command = std::process::Command::new("wmic");
      check_command.args([
        "process",
        "where",
        "name='mod-tools.exe'",
        "get",
        "processid",
      ]);

      const CREATE_NO_WINDOW: u32 = 0x08000000;
      check_command.creation_flags(CREATE_NO_WINDOW);

      if let Ok(output) = check_command.output() {
        if output.status.success() {
          let output_str = String::from_utf8_lossy(&output.stdout);
          for line in output_str.lines() {
            let line = line.trim();
            if line != "ProcessId" && !line.is_empty() && line.chars().all(|c| c.is_digit(10)) {
              return true; // Found at least one mod-tools process
            }
          }
        }
      }
    }

    // Also check if overlay directory exists with content
    let overlay_dir = self.app_dir.join("overlay");
    if overlay_dir.exists() {
      if let Ok(entries) = fs::read_dir(&overlay_dir) {
        return entries.count() > 0;
      }
    }

    false
  }

  // Cleanup mod-tools processes specifically
  pub(crate) fn cleanup_mod_tools_processes(&mut self) -> Result<(), InjectionError> {
    self.log("Cleaning up mod-tools processes...");

    #[cfg(target_os = "windows")]
    {
      // Kill any running mod-tools processes
      let mut command = std::process::Command::new("taskkill");
      command.args(["/F", "/IM", "mod-tools.exe"]);
      command.creation_flags(CREATE_NO_WINDOW);

      match command.output() {
        Ok(output) => {
          if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("not found") && !stderr.contains("No tasks are running") {
              self.log(&format!(
                "Warning: Failed to kill mod-tools processes: {}",
                stderr
              ));
            }
          } else {
            self.log("Successfully terminated mod-tools processes");
          }
        }
        Err(e) => {
          self.log(&format!("Warning: Could not run taskkill: {}", e));
        }
      }
    }

    #[cfg(not(target_os = "windows"))]
    {
      // For non-Windows systems, use pkill
      let mut command = std::process::Command::new("pkill");
      command.args(["-f", "mod-tools"]);
      let _ = command.output();
    }

    Ok(())
  }
}

// Main wrapper function that is called from commands.rs
pub fn inject_skins(
  app_handle: &AppHandle,
  game_path: &str,
  skins: &[Skin],
  skin_file_files_dir: &Path,
) -> Result<(), String> {
  inject_skins_and_misc(app_handle, game_path, skins, &[], skin_file_files_dir)
}

// Enhanced wrapper function that supports both skins and misc items
pub fn inject_skins_and_misc(
  app_handle: &AppHandle,
  game_path: &str,
  skins: &[Skin],
  misc_items: &[MiscItem],
  skin_file_files_dir: &Path,
) -> Result<(), String> {
  // Create injector
  let mut injector = SkinInjector::new(app_handle, game_path)
    .map_err(|e| format!("Failed to create injector: {}", e))?;

  // Initialize
  injector
    .initialize()
    .map_err(|e| format!("Failed to initialize: {}", e))?;

  // Inject skins and misc items
  injector
    .inject_skins_and_misc(skins, misc_items, skin_file_files_dir)
    .map_err(|e| format!("Failed to inject: {}", e))
}

// New function to check if cleanup is needed without performing it
pub fn needs_injection_cleanup(app_handle: &AppHandle, game_path: &str) -> Result<bool, String> {
  // Create injector
  let injector = SkinInjector::new(app_handle, game_path)
    .map_err(|e| format!("Failed to create injector: {}", e))?;

  // Check if cleanup is needed
  Ok(injector.needs_cleanup())
}

// New function to clean up the injection when needed
pub fn cleanup_injection(app_handle: &AppHandle, game_path: &str) -> Result<(), String> {
  // Create injector
  let mut injector = SkinInjector::new(app_handle, game_path)
    .map_err(|e| format!("Failed to create injector: {}", e))?;

  // Call cleanup
  injector
    .cleanup()
    .map_err(|e| format!("Failed to stop skin injection: {}", e))
}
