use std::fs;
use std::io;
use std::path::{Path};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use walkdir::WalkDir;
use crate::injection::error::{InjectionError, ModState};
use crate::injection::fantome::copy_default_overlay;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Mod-tools operations and overlay management

impl crate::injection::core::SkinInjector {
    // Copy a processed mod directory to the game's mods directory
    pub(crate) fn copy_mod_to_game(&mut self, mod_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Copying mod to game directory: {}", mod_dir.display()));

        // Use the mod directory name as the subfolder
        let mod_name = mod_dir.file_name().unwrap();
        let game_mod_dir = self.game_path.join("mods").join(mod_name);

        // Remove any existing mod with the same name
        if game_mod_dir.exists() {
            fs::remove_dir_all(&game_mod_dir)?;
        }
        fs::create_dir_all(&game_mod_dir)?;

        // Copy everything from mod_dir into game_mod_dir
        for entry in WalkDir::new(mod_dir) {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path.strip_prefix(mod_dir)
                .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
            let target_path = game_mod_dir.join(rel_path);

            if path.is_dir() {
                fs::create_dir_all(&target_path)?;
            } else if path.is_file() {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &target_path)?;
            }
        }
        Ok(())
    }
    
    // Run the overlay process using mod-tools.exe
    pub(crate) fn run_overlay(&mut self) -> Result<(), InjectionError> {
        // Check if mod-tools.exe exists
        let mod_tools_path = match &self.mod_tools_path {
            Some(path) => {
                if !path.exists() {
                    return Err(InjectionError::ProcessError(format!(
                        "mod-tools.exe was found during initialization but is no longer at path: {}. Please reinstall the application or obtain mod-tools.exe from CSLOL Manager.",
                        path.display()
                    )));
                }
                path.clone()
            },
            None => return Err(InjectionError::ProcessError(
                "mod-tools.exe not found. Please install CSLOL Manager or copy mod-tools.exe to the application directory.".into()
            )),
        };

        self.log(&format!("Using mod-tools.exe from: {}", mod_tools_path.display()));

        // First, ensure no mod-tools processes are running before we start
        let _ = self.cleanup_mod_tools_processes();
        
        // First create the overlay
        let game_mods_dir = self.game_path.join("mods");
        let overlay_dir = self.app_dir.join("overlay");
        
        // Make sure overlay directory exists or is recreated
        if overlay_dir.exists() {
            // Try to remove the overlay dir multiple times with delays
            // This helps with Windows file locks that might be causing access violations
            let mut attempts = 0;
            let max_attempts = 3;
            while attempts < max_attempts {
                match fs::remove_dir_all(&overlay_dir) {
                    Ok(_) => break,
                    Err(e) => {
                        self.log(&format!("Failed to remove overlay directory (attempt {}/{}): {}", 
                            attempts + 1, max_attempts, e));
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        attempts += 1;
                    }
                }
            }
            
            // If still exists, return error
            if overlay_dir.exists() && attempts >= max_attempts {
                return Err(InjectionError::ProcessError(
                    "Cannot remove existing overlay directory. It may be locked by another process.".into()
                ));
            }
        }
        fs::create_dir_all(&overlay_dir)?;

        // Get list of mod names (just the directory names, no paths)
        let mut mod_names = Vec::new();
        for entry in fs::read_dir(&game_mods_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("META").join("info.json").exists() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        mod_names.push(name_str.to_string());
                    }
                }
            }
        }

        // Check if we have any valid mods
        if mod_names.is_empty() {
            self.log("No valid mods found in game directory");
        } else {
            self.log(&format!("Found {} mods to include in overlay", mod_names.len()));
        }
        
        // First try to use a pre-built default overlay from resources
        // This is especially helpful for the first injection which is often slow
        let used_prebuilt_empty = if let Some(app_handle) = &self.app_handle {
            if mod_names.is_empty() {
                // For empty mod list, try the most optimized path - use the pre-built empty overlay
                match copy_default_overlay(app_handle, &overlay_dir) {
                    Ok(true) => {
                        self.log("Using pre-built empty overlay template for faster injection");
                        true
                    },
                    _ => false
                }
            } else {
                false
            }
        } else {
            false
        };
        
        if !used_prebuilt_empty {
            // Join mod names with / as CSLOL expects
            let mods_arg = mod_names.join("/");

            self.log("Creating mod overlay...");
            
            // Try the mkoverlay command with retries for access violation errors (0xc0000005)
            let max_retries = 5;
            let mut retry_count = 0;
            let mut last_error = None;
            
            while retry_count < max_retries {
                // Small delay between retries to let resources free up
                if retry_count > 0 {
                    self.log(&format!("Retrying overlay creation (attempt {}/{})", retry_count + 1, max_retries));
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    
                    // Make sure any lingering processes are killed
                    let _ = self.cleanup_mod_tools_processes();
                    
                    // For additional retries, recreate the overlay directory to ensure it's clean
                    if overlay_dir.exists() {
                        match fs::remove_dir_all(&overlay_dir) {
                            Ok(_) => fs::create_dir_all(&overlay_dir)?,
                            Err(e) => {
                                self.log(&format!("Could not clean overlay directory: {}", e));
                                // Try to continue anyway
                            }
                        }
                    } else {
                        fs::create_dir_all(&overlay_dir)?;
                    }
                }
                
                // Add more explicit memory cleanup hint
                #[cfg(target_os = "windows")]
                {
                    use std::process::Command;
                    // Run a quick garbage collection via PowerShell to help free memory
                    if retry_count > 0 {
                        let mut gc_cmd = Command::new("powershell");
                        gc_cmd.args(["-Command", "[System.GC]::Collect()"]);
                        gc_cmd.creation_flags(CREATE_NO_WINDOW);
                        let _ = gc_cmd.output(); // Ignore output
                    }
                }
                
                let mut command = std::process::Command::new(&mod_tools_path);
                command.args([
                    "mkoverlay",
                    game_mods_dir.to_str().unwrap(),
                    overlay_dir.to_str().unwrap(),
                    &format!("--game:{}", self.game_path.to_str().unwrap()),
                    &format!("--mods:{}", mods_arg),
                    "--noTFT",
                    "--ignoreConflict"
                ]);
                
                #[cfg(target_os = "windows")]
                command.creation_flags(CREATE_NO_WINDOW);
                
                // Hide command output for cleaner logs, just show we're working
                if retry_count == 0 {
                    self.log("Running mkoverlay command...");
                }
                
                match command.output() {
                    Ok(output) => {
                        if output.status.success() {
                            // Success - break out of retry loop
                            self.log("Overlay creation succeeded!");
                            break;
                        } else {
                            // Command ran but had error status
                            let stderr_output = String::from_utf8_lossy(&output.stderr).into_owned();
                            let stdout_output = String::from_utf8_lossy(&output.stdout).into_owned();
                            let error_message = if stderr_output.is_empty() { stdout_output } else { stderr_output };
                            
                            // Check if it's an access violation error
                            if output.status.to_string().contains("0xc0000005") {
                                self.log(&format!("Access violation error in attempt {}/{}. Retrying...", 
                                    retry_count + 1, max_retries));
                                last_error = Some(InjectionError::ProcessError(format!(
                                    "mkoverlay command failed: {}. Exit code: {}", 
                                    error_message, output.status
                                )));
                                retry_count += 1;
                                continue;
                            } else {
                                // Other error, only show error log if this is the final attempt
                                if retry_count + 1 >= max_retries {
                                    self.log(&format!("Overlay creation failed: {}", error_message));
                                }
                                retry_count += 1;
                                
                                last_error = Some(InjectionError::ProcessError(format!(
                                    "mkoverlay command failed: {}. Exit code: {}", 
                                    error_message, output.status
                                )));
                                
                                // Try again if we haven't exhausted retries
                                if retry_count < max_retries {
                                    continue;
                                }
                                
                                return Err(last_error.unwrap());
                            }
                        }
                    },
                    Err(e) => {
                        // Command couldn't be started
                        return Err(InjectionError::ProcessError(format!(
                            "Failed to create overlay: {}. The mod-tools.exe might be missing or incompatible.", e
                        )));
                    }
                }
            }
            
            // Check if we exhausted our retries
            if retry_count >= max_retries {
                if let Some(err) = last_error {
                    return Err(err);
                }
                return Err(InjectionError::ProcessError("Failed to create overlay after multiple attempts".into()));
            }
        }

        // Create config.json
        let config_path = self.app_dir.join("config.json");
        let config_content = r#"{"enableMods":true}"#;
        fs::write(&config_path, config_content)?;

        self.log("Starting overlay process...");

        // Important: Set state to Running BEFORE spawning process
        self.set_state(ModState::Running);

        // Try running the overlay, with retries
        let max_run_retries = 2;
        let mut run_retry_count = 0;
        let mut last_run_error = None;
        
        while run_retry_count < max_run_retries {
            if run_retry_count > 0 {
                self.log(&format!("Retrying overlay run (attempt {}/{})", run_retry_count + 1, max_run_retries));
                std::thread::sleep(std::time::Duration::from_millis(1000));
                
                // Make sure any lingering processes are killed
                let _ = self.cleanup_mod_tools_processes();
            }
            
            // Run the overlay process - EXACT format from CSLOL
            let mut command = std::process::Command::new(&mod_tools_path);
            command.args([
                "runoverlay",
                overlay_dir.to_str().unwrap(),
                config_path.to_str().unwrap(),
                &format!("--game:{}", self.game_path.to_str().unwrap()),  // Use game_path which points to Game directory
                "--opts:configless"
            ]);
            
            #[cfg(target_os = "windows")]
            command.creation_flags(CREATE_NO_WINDOW);

            match command.spawn() {
                Ok(_) => {
                    self.log("Overlay process started successfully");
                    return Ok(());
                },
                Err(e) => {
                    run_retry_count += 1;
                    
                    // Store error for potential later use
                    last_run_error = Some(match e.kind() {
                        io::ErrorKind::NotFound => InjectionError::ProcessError(format!(
                            "mod-tools.exe not found or is inaccessible at path: {}. Please install CSLOL Manager or copy the correct mod-tools.exe to the application directory.", 
                            mod_tools_path.display()
                        )),
                        io::ErrorKind::PermissionDenied => InjectionError::ProcessError(format!(
                            "Permission denied when trying to run mod-tools.exe. Try running the application as administrator."
                        )),
                        _ => InjectionError::ProcessError(format!(
                            "Error running mod-tools.exe: {}. Please ensure it's correctly installed and compatible with your system.", 
                            e
                        ))
                    });
                    
                    if run_retry_count < max_run_retries {
                        self.log(&format!("Failed to start overlay process: {}. Retrying...", e));
                        continue;
                    }
                }
            }
        }
        
        // If we got here, all retries failed
        self.set_state(ModState::Idle); // Reset state on error
        
        if let Some(err) = last_run_error {
            self.log(&format!("Failed to start overlay process after {} attempts", max_run_retries));
            Err(err)
        } else {
            Err(InjectionError::ProcessError("Failed to start overlay process after multiple attempts".into()))
        }
    }
}
