// Main LCU watcher loop and file system monitoring

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use notify::{Watcher, RecursiveMode, Event as NotifyEvent};
use base64::{engine::general_purpose, Engine};

use crate::commands::party_mode::{PARTY_MODE_VERBOSE, RECEIVED_SKINS, clear_received_skins, clear_sent_shares};
use crate::commands::types::{SkinData, SavedConfig};
use crate::commands::misc_items::get_selected_misc_items;
use crate::injection::{inject_skins_and_misc, Skin};
use super::types::{InjectionMode, PHASE_STATE, PARTY_INJECTION_DONE_THIS_PHASE, LAST_PARTY_INJECTION_SIGNATURE, LAST_CHAMPION_SHARE_TIME};
use super::utils::{read_injection_mode, compute_party_injection_signature};
use super::logging::emit_terminal_log;
use super::session::{get_selected_champion_id, get_swift_play_champion_selections, extract_swift_play_champions_from_lobby};
use super::injection::{inject_skins_for_champions, trigger_party_mode_injection, trigger_party_mode_injection_for_champions};
use super::party_mode::check_for_party_mode_messages_with_connection;

#[tauri::command]
pub fn start_lcu_watcher(app: AppHandle, league_path: String) -> Result<(), String> {
  println!("Starting improved LCU watcher with file system monitoring for path: {}", league_path);
  let app_handle = app.clone();
  let league_path_clone = league_path.clone();

  thread::spawn(move || {
    // Decide injection mode early
    let injection_mode = read_injection_mode(&app_handle);
    println!(
      "[LCU Watcher] Injection mode: {}",
      if injection_mode == InjectionMode::ChampSelect {
        "ChampSelect"
      } else {
        "Lobby"
      }
    );

    if let Ok(cfg_dir) = app_handle.path().app_data_dir() {
      let cfg_file = cfg_dir.join("config").join("config.json");
      if let Ok(contents) = std::fs::read_to_string(&cfg_file) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
          if let Some(pm) = json.get("party_mode") {
            if let Some(flag) = pm.get("verbose_logging").and_then(|v| v.as_bool()) {
              PARTY_MODE_VERBOSE.store(flag, Ordering::Relaxed);
              println!("[LCU Watcher] Party mode verbose logging = {}", flag);
            }
          }
        }
      }
    }
    let mut last_phase = String::new();
    let mut was_in_game = false;
    let mut was_reconnecting = false;
    let _ = app_handle.emit("lcu-status", "None".to_string());

    // Track last seen selections to detect changes
    let mut last_selected_skins: std::collections::HashMap<u32, SkinData> =
      std::collections::HashMap::new();
    let mut last_skin_check_time = std::time::Instant::now();
    let mut last_champion_id: Option<u32> = None;
    let mut last_party_mode_check = std::time::Instant::now();
    let mut last_party_injection_check = std::time::Instant::now();
    let mut processed_message_ids: std::collections::HashSet<String> =
      std::collections::HashSet::new();
    let mut last_party_injection_time: std::time::Instant =
      std::time::Instant::now() - std::time::Duration::from_secs(60);

    // Set up file system watcher for lockfile - provides instant detection
    let (lockfile_tx, lockfile_rx) = mpsc::channel();
    let league_path_for_watcher = league_path_clone.clone();
    
    thread::spawn(move || {
      match notify::recommended_watcher(move |res: Result<NotifyEvent, notify::Error>| {
        if res.is_ok() {
          // Debounce: ignore rapid-fire events from same file operation
          let _ = lockfile_tx.send(());
        }
      }) {
        Ok(mut watcher) => {
          match watcher.watch(PathBuf::from(&league_path_for_watcher).as_path(), RecursiveMode::NonRecursive) {
            Ok(_) => {
              println!("[LCU Watcher] File system watcher active for instant lockfile detection");
              // Keep watcher alive - it will drop when the thread ends
              loop {
                thread::sleep(Duration::from_secs(3600)); // Sleep longer since we only need to keep thread alive
              }
            }
            Err(e) => {
              eprintln!("[LCU Watcher] Failed to start file watcher: {} - falling back to polling only", e);
            }
          }
        }
        Err(e) => {
          eprintln!("[LCU Watcher] Failed to create file watcher: {} - falling back to polling only", e);
        }
      }
    });

    loop {
      let mut sleep_duration = Duration::from_secs(2); // Responsive polling for phase changes (2s), file watcher provides instant lockfile detection

      // Only check the configured League directory for lockfile
      let search_dirs = [PathBuf::from(&league_path_clone)];
      let mut port = None;
      let mut token = None;
      let mut found_any_lockfile = false;
      let mut lockfile_path = None;

      // Lockfile detection - reduced logging to minimize console spam
      for dir in &search_dirs {

        // Check each possible lockfile name
        for name in [
          "lockfile",
          "LeagueClientUx.lockfile",
          "LeagueClient.lockfile",
        ] {
          let path = dir.join(name);
          if let Ok(content) = fs::read_to_string(&path) {
            found_any_lockfile = true;
            lockfile_path = Some(path.clone());
            // Only log when lockfile status changes (not on every loop)
            if lockfile_path.is_none() {
              println!("[LCU Watcher] Found lockfile: {}", path.display());
              emit_terminal_log(
                &app_handle,
                &format!("[LCU Watcher] Found lockfile: {}", path.display()),
              );
            }
            let parts: Vec<&str> = content.split(':').collect();
            if parts.len() >= 5 {
              port = Some(parts[2].to_string());
              token = Some(parts[3].to_string());
              found_any_lockfile = true;
              break;
            }
          }
        }

        if port.is_some() && token.is_some() {
          break;
        }
      }

      if !found_any_lockfile {
        // Handle no lockfile found cases...
        if was_in_game && (last_phase == "InProgress" || was_reconnecting) {
          thread::sleep(Duration::from_secs(5));
          continue;
        } else if was_in_game && last_phase == "None" {
          // Fallback cleanup when no lockfile is found after being in game
          // The primary cleanup is now handled by phase change detection for better performance
          if let Err(e) = crate::injection::cleanup_injection(&app_handle, &league_path_clone) {
            println!(
              "[LCU Watcher] Error in fallback cleanup after game exit: {}",
              e
            );
            emit_terminal_log(
              &app_handle,
              &format!(
                "[LCU Watcher] Error in fallback cleanup after game exit: {}",
                e
              ),
            );
          } else {
            println!("[LCU Watcher] Fallback cleanup completed after game exit");
            emit_terminal_log(
              &app_handle,
              "[LCU Watcher] Fallback cleanup completed after game exit",
            );
          }
          was_in_game = false;
        }

        let log_msg = format!(
          "[LCU Watcher] No valid lockfile found. Is League running? The lockfile should be at: {}",
          league_path_clone
        );
        println!("{}", log_msg);
        emit_terminal_log(&app_handle, &log_msg);
        thread::sleep(Duration::from_secs(5));
        continue;
      }

      let port = port.unwrap();
      let token = token.unwrap();
      let lockfile_path = lockfile_path.unwrap();

      'lcu_connected: loop {
        if !lockfile_path.exists() {
          break 'lcu_connected;
        }

        match reqwest::blocking::Client::builder()
          .danger_accept_invalid_certs(true)
          .build()
        {
          Ok(client) => {
            let endpoints = [
              "/lol-gameflow/v1/session",
              "/lol-gameflow/v1/gameflow-phase",
            ];

            let mut connected = false;
            let mut phase_value: Option<String> = None;

            for endpoint in endpoints {
              let url = format!("https://127.0.0.1:{}{}", port, endpoint);
              let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

              match client
                .get(&url)
                .header("Authorization", format!("Basic {}", auth))
                .send()
              {
                Ok(resp) => {
                  if resp.status().is_success() {
                    connected = true;

                    match resp.json::<serde_json::Value>() {
                      Ok(json) => {
                        if endpoint == "/lol-gameflow/v1/gameflow-phase" {
                          if let Some(phase) = json.as_str() {
                            phase_value = Some(phase.to_string());
                            break;
                          }
                        } else {
                          if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
                            phase_value = Some(phase.to_string());
                            break;
                          }
                        }
                      }
                      Err(e) => println!(
                        "[LCU Watcher] Failed to parse response from {}: {}",
                        endpoint, e
                      ),
                    }
                  }
                }
                Err(e) => println!(
                  "[LCU Watcher] Failed to connect to endpoint {}: {}",
                  endpoint, e
                ),
              }
            }

            if !connected {
              thread::sleep(Duration::from_secs(5));
              continue;
            }

            let phase = phase_value.unwrap_or_else(|| "None".to_string());

            if phase != last_phase {
              println!(
                "[LCU Watcher] LCU status changed: {} -> {}",
                last_phase, phase
              );
              emit_terminal_log(
                &app_handle,
                &format!(
                  "[LCU Watcher] LCU status changed: {} -> {}",
                  last_phase, phase
                ),
              );

              // Only clean up injection on specific phase transitions where injection is no longer needed
              // Keep injection active during InProgress (in-game) and Reconnect phases
              let should_cleanup = match (&*last_phase, &*phase) {
                // Clean up when leaving game back to lobby/none
                ("InProgress", "None") => true,
                ("InProgress", "Lobby") => true,
                ("InProgress", "Matchmaking") => true,
                // If we move from Matchmaking back to Lobby, treat it as a cancellation and cleanup
                ("Matchmaking", "Lobby") => true,
                ("Reconnect", "None") => true,
                ("Reconnect", "Lobby") => true,
                ("Reconnect", "Matchmaking") => true,
                // Clean up when going from ChampSelect to lobby/none (cancelled queue)
                ("ChampSelect", "None") => true,
                ("ChampSelect", "Lobby") => true,
                ("ChampSelect", "Matchmaking") => true,
                // Clean up when client disconnects
                (_, "None") if last_phase != "None" => true,
                // Don't clean up when entering game phases
                ("ChampSelect", "InProgress") => false,
                ("ChampSelect", "Reconnect") => false,
                ("InProgress", "Reconnect") => false,
                ("Reconnect", "InProgress") => false,
                // Default: don't clean up for other transitions
                _ => false,
              };

              if should_cleanup {
                // Reset party injection flag when cleaning up (allows new injection on next phase)
                PARTY_INJECTION_DONE_THIS_PHASE.store(false, Ordering::Relaxed);

                // ALWAYS emit idle status to frontend first, regardless of whether processes need cleanup
                // This ensures the UI updates immediately when user cancels matchmaking
                let _ = app_handle.emit("injection-status", "idle");

                match crate::injection::needs_injection_cleanup(&app_handle, &league_path_clone) {
                  Ok(needs_cleanup) => {
                    if needs_cleanup {
                      let log_msg = format!("[LCU Watcher] Injection cleanup needed for phase transition {} -> {}, cleaning up...", last_phase, phase);
                      println!("{}", log_msg);
                      emit_terminal_log(&app_handle, &log_msg);

                      if let Err(e) =
                        crate::injection::cleanup_injection(&app_handle, &league_path_clone)
                      {
                        let error_msg = format!(
                          "[LCU Watcher] Error cleaning up injection on phase change: {}",
                          e
                        );
                        println!("{}", error_msg);
                        emit_terminal_log(&app_handle, &error_msg);
                      } else {
                        let success_msg =
                          "[LCU Watcher] ✅ Injection cleanup completed successfully";
                        println!("{}", success_msg);
                        emit_terminal_log(&app_handle, success_msg);
                      }
                    } else {
                      let log_msg = format!("[LCU Watcher] Phase transition {} -> {} would trigger cleanup, but no injection active", last_phase, phase);
                      println!("{}", log_msg);
                      emit_terminal_log(&app_handle, &log_msg);
                    }
                  }
                  Err(e) => {
                    let error_msg =
                      format!("[LCU Watcher] Error checking if cleanup is needed: {}", e);
                    println!("{}", error_msg);
                    emit_terminal_log(&app_handle, &error_msg);
                  }
                }
              } else {
                let log_msg = format!("[LCU Watcher] Phase transition {} -> {} does not require cleanup, keeping injection active", last_phase, phase);
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, &log_msg);
              }

              // If entering ChampSelect, reset state, preload assets to speed up injection later
              if phase == "ChampSelect" {
                // Clear any previous party-mode state to avoid carrying over signatures or received skins
                if let Ok(mut g) = LAST_PARTY_INJECTION_SIGNATURE.lock() {
                  *g = None;
                }
                // Clear any previously received skins (start fresh for this ChampSelect)
                clear_received_skins();
                // Reset outbound share dedup for new phase
                clear_sent_shares();
                // Clear champion share timestamps (allows sharing in new phase)
                if let Ok(mut times) = LAST_CHAMPION_SHARE_TIME.lock() {
                  times.clear();
                }
                // Allow a single party-mode injection for this phase
                PARTY_INJECTION_DONE_THIS_PHASE.store(false, Ordering::Relaxed);

                println!("[LCU Watcher][DEBUG] Reset party-mode dedup signature and cleared received skins for new ChampSelect");
                emit_terminal_log(&app_handle, "[LCU Watcher][DEBUG] Reset party-mode dedup signature and cleared received skins for new ChampSelect");

                let champions_dir = app_handle
                  .path()
                  .app_data_dir()
                  .unwrap_or_else(|_| PathBuf::from("."))
                  .join("champions");

                if !champions_dir.exists() {
                  if let Err(e) = fs::create_dir_all(&champions_dir) {
                    println!("[LCU Watcher] Failed to create champions directory: {}", e);
                  }
                }

                let app_dir = app_handle
                  .path()
                  .app_data_dir()
                  .unwrap_or_else(|_| PathBuf::from("."));
                let overlay_dir = app_dir.join("overlay");
                if overlay_dir.exists() {
                  if let Err(e) = fs::remove_dir_all(&overlay_dir) {
                    println!("[LCU Watcher] Failed to clean overlay directory: {}", e);
                  }
                }

                // Check if manual injection mode is active and trigger injection
                if crate::commands::skin_injection::is_manual_injection_active() {
                  println!("[LCU Watcher] Manual injection mode active - triggering injection");
                  emit_terminal_log(
                    &app_handle,
                    "[LCU Watcher] Manual injection mode active - triggering injection",
                  );
                  let app_clone = app_handle.clone();
                  std::thread::spawn(move || {
                    let rt =
                      tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                    rt.block_on(async move {
                      match crate::commands::skin_injection::trigger_manual_injection(&app_clone)
                        .await
                      {
                        Ok(_) => {
                          println!("[LCU Watcher] Manual injection completed successfully");
                        }
                        Err(e) => {
                          eprintln!("[LCU Watcher] Manual injection failed: {}", e);
                        }
                      }
                    });
                  });
                }
              }
            }

            // Check for party mode messages (polling required as LCU doesn't provide WebSocket events for all chat messages)
            // Reduced from 3s to 1.5s for better responsiveness while still being reasonable
            if last_party_mode_check.elapsed().as_millis() >= 1500 {
              last_party_mode_check = std::time::Instant::now();
              if let Err(e) = check_for_party_mode_messages_with_connection(
                &app_handle,
                &port,
                &token,
                &mut processed_message_ids,
              ) {
                eprintln!("Error checking party mode messages: {}", e);
              }
            }

            // Check if party mode injection should be triggered
            // Runs independently of champion changes to handle timing properly
            // Skip party mode injection if manual injection mode is active
            if phase == "ChampSelect"
              && last_party_injection_check.elapsed().as_millis() >= 1000 // Reduced from 2s to 1s
              && !crate::commands::skin_injection::is_manual_injection_active()
            {
              last_party_injection_check = std::time::Instant::now();

              // Get current champion selection first
              let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
              let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

              let current_champion_id = if let Ok(resp) = client
                .get(&session_url)
                .header("Authorization", format!("Basic {}", auth))
                .send()
              {
                if resp.status().is_success() {
                  if let Ok(json) = resp.json::<serde_json::Value>() {
                    get_selected_champion_id(&json)
                      .map(|id| id as u32)
                      .unwrap_or(0)
                  } else {
                    0
                  }
                } else {
                  0
                }
              } else {
                0
              };

              // Check if all friends have shared and local player has locked in
              let should_inject = {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async {
                  crate::commands::party_mode::should_inject_now(&app_handle, current_champion_id)
                    .await
                    .unwrap_or(false)
                })
              };

              let already_done = PARTY_INJECTION_DONE_THIS_PHASE.load(Ordering::Relaxed);
              if should_inject
                && !already_done
                && last_party_injection_time.elapsed().as_secs() >= 5
              {
                // Only inject if the set of received skins changed since last injection
                let current_sig = compute_party_injection_signature(current_champion_id);
                let mut guard = LAST_PARTY_INJECTION_SIGNATURE.lock().unwrap();
                if guard.as_ref() != Some(&current_sig) {
                  println!("[Party Mode][DEBUG] Previous signature: {:?}", *guard);
                  println!("[Party Mode][DEBUG] Current signature: {}", current_sig);
                  println!("[Party Mode] All conditions met - triggering party mode injection");

                  // Immediately mark as done to prevent rapid re-triggering in ARAM/fast modes
                  // This prevents injection loop even if the injection itself fails
                  PARTY_INJECTION_DONE_THIS_PHASE.store(true, Ordering::Relaxed);

                  // Update signature immediately to prevent duplicate attempts
                  *guard = Some(current_sig.clone());
                  drop(guard); // Release lock before spawning thread

                  let app_handle_clone = app_handle.clone();
                  let champ_to_inject = current_champion_id;
                  // Use std::thread::spawn instead of tokio::spawn to avoid reactor issues
                  std::thread::spawn(move || {
                    let rt =
                      tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                    rt.block_on(async move {
                      match trigger_party_mode_injection(&app_handle_clone, champ_to_inject).await {
                        Ok(_) => {
                          println!(
                            "[Party Mode] Injection completed successfully for champion {}",
                            champ_to_inject
                          );
                        }
                        Err(e) => {
                          eprintln!("[Party Mode] Failed to trigger injection: {}", e);
                          // Note: We don't reset PARTY_INJECTION_DONE_THIS_PHASE here to prevent loop
                          // User can manually retry if needed
                        }
                      }
                    });
                  });
                  // Update last attempt time to debounce repeated triggers
                  last_party_injection_time = std::time::Instant::now();
                } else {
                  // Debounce: nothing new since last injection
                  println!("[Party Mode][DEBUG] Skipping injection; signature unchanged");
                }
              }
            }

            // CHAMP SELECT MODE: continuously monitor champ-select session and inject on lock-in
            // Skip automatic injection if manual injection mode is active
            if injection_mode == InjectionMode::ChampSelect
              && phase == "ChampSelect"
              && !crate::commands::skin_injection::is_manual_injection_active()
            {
              let now = std::time::Instant::now();
              if now.duration_since(last_skin_check_time).as_secs() >= 1 {
                last_skin_check_time = now;

                let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
                let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

                if let Ok(resp) = client
                  .get(&session_url)
                  .header("Authorization", format!("Basic {}", auth))
                  .send()
                {
                  if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>() {
                      if let Some(selected_champ_id) = get_selected_champion_id(&json) {
                        let current_champion_id = selected_champ_id as u32;

                        // Trigger skin sharing when locking in for the first time OR when changing to a different locked champion
                        let champion_changed = if let Some(last_champ) = last_champion_id {
                          last_champ != current_champion_id
                        } else {
                          true // Share on first lock-in
                        };

                        // Always update last_champion_id when a champion is locked
                        last_champion_id = Some(current_champion_id);

                        if champion_changed {
                          let config_dir = app_handle
                            .path()
                            .app_data_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join("config");
                          let cfg_file = config_dir.join("config.json");

                          if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                            if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                              let mut skins_to_inject = Vec::new();

                              // Add local skin if available
                              if let Some(skin) = config
                                .skins
                                .iter()
                                .find(|s| s.champion_id == current_champion_id)
                              {
                                skins_to_inject.push(Skin {
                                  champion_id: skin.champion_id,
                                  skin_id: skin.skin_id,
                                  chroma_id: skin.chroma_id,
                                  skin_file_path: skin.skin_file.clone(),
                                });

                                // Send skin share to paired friends (sharing is now controlled per-friend)
                                if !config.party_mode.paired_friends.is_empty() {
                                  // Debounce rapid champion changes (ARAM rerolls): only share if 2+ seconds since last share for this champion
                                  let should_share = {
                                    let mut last_shares = LAST_CHAMPION_SHARE_TIME.lock().unwrap();
                                    if let Some(last_time) = last_shares.get(&current_champion_id) {
                                      let elapsed = last_time.elapsed();
                                      if elapsed.as_secs() < 2 {
                                        println!("[Party Mode][ChampSelect] Skipping rapid share for champion {} (last shared {}ms ago)",
                                                                                         current_champion_id, elapsed.as_millis());
                                        false
                                      } else {
                                        last_shares
                                          .insert(current_champion_id, std::time::Instant::now());
                                        true
                                      }
                                    } else {
                                      // First time sharing this champion
                                      last_shares
                                        .insert(current_champion_id, std::time::Instant::now());
                                      true
                                    }
                                  };

                                  if should_share {
                                    let app_handle_clone = app_handle.clone();
                                    let skin_clone = skin.clone();
                                    // Use a Tokio runtime if not already inside one
                                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                                      handle.spawn(async move {
                                                                                if let Err(e) = crate::commands::party_mode::send_skin_share_to_paired_friends(
                                                                                    &app_handle_clone,
                                                                                    skin_clone.champion_id,
                                                                                    skin_clone.skin_id,
                                                                                    skin_clone.chroma_id,
                                                                                    skin_clone.skin_file.clone(),
                                                                                ).await {
                                                                                    eprintln!("Failed to send skin share: {}", e);
                                                                                }
                                                                            });
                                    } else {
                                      // No runtime, so create one just for this task
                                      std::thread::spawn(move || {
                                        let rt = tokio::runtime::Runtime::new()
                                          .expect("Failed to create Tokio runtime");
                                        rt.block_on(async move {
                                                                                    if let Err(e) = crate::commands::party_mode::send_skin_share_to_paired_friends(
                                                                                        &app_handle_clone,
                                                                                        skin_clone.champion_id,
                                                                                        skin_clone.skin_id,
                                                                                        skin_clone.chroma_id,
                                                                                        skin_clone.skin_file.clone(),
                                                                                    ).await {
                                                                                        eprintln!("Failed to send skin share: {}", e);
                                                                                    }
                                                                                });
                                      });
                                    }
                                  }
                                }

                                last_selected_skins.insert(current_champion_id, skin.clone());
                              }
                            }
                          }
                        }
                      } else {
                        // Champion not locked or changed, but still update last_champion_id if locked
                        if let Some(selected_champ_id) = get_selected_champion_id(&json) {
                          last_champion_id = Some(selected_champ_id as u32);
                        } else {
                          last_champion_id = None;
                        }
                      }
                    }
                  }
                }

                // Skip automatic injection if manual injection mode is active
                if !crate::commands::skin_injection::is_manual_injection_active() {
                  let config_dir = app_handle
                    .path()
                    .app_data_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("config");
                  let cfg_file = config_dir.join("config.json");

                  if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                    if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                      let mut skin_changes = false;

                      for skin in &config.skins {
                        let champ_id = skin.champion_id;

                        if last_champion_id == Some(champ_id) {
                          // Check if skin has changed - simplified to always re-inject when different
                          let skin_has_changed = !last_selected_skins.contains_key(&champ_id)
                            || last_selected_skins.get(&champ_id).map_or(true, |old_skin| {
                              old_skin.skin_id != skin.skin_id
                                || old_skin.chroma_id != skin.chroma_id
                                || old_skin.skin_file != skin.skin_file
                            });
                          
                          if skin_has_changed {
                            println!("[Auto Injection] Skin change detected for champion {}, triggering re-injection", champ_id);
                            
                            let mut skins_to_inject = vec![Skin {
                              champion_id: skin.champion_id,
                              skin_id: skin.skin_id,
                              chroma_id: skin.chroma_id,
                              skin_file_path: skin.skin_file.clone(),
                            }];

                            // Add received skins for this champion
                            for (_key, received_skin) in RECEIVED_SKINS.lock().unwrap().iter() {
                              if received_skin.champion_id == champ_id {
                                skins_to_inject.push(Skin {
                                  champion_id: received_skin.champion_id,
                                  skin_id: received_skin.skin_id,
                                  chroma_id: received_skin.chroma_id,
                                  skin_file_path: received_skin.skin_file_path.clone(),
                                });
                              }
                            }

                            let champions_dir = app_handle
                              .path()
                              .app_data_dir()
                              .unwrap_or_else(|_| PathBuf::from("."))
                              .join("champions");

                            // If we're in ChampSelect injection mode, ensure we only inject during ChampSelect
                            if injection_mode == InjectionMode::ChampSelect
                              && phase != "ChampSelect"
                            {
                              continue;
                            }

                            // For skin changes, check if we should wait for party mode friends
                            // should_inject_now returns true immediately if no friends are in party
                            let should_inject = {
                              let rt = tokio::runtime::Runtime::new()
                                .expect("Failed to create Tokio runtime");
                              rt.block_on(async {
                                match crate::commands::party_mode::should_inject_now(
                                  &app_handle,
                                  champ_id,
                                )
                                .await
                                {
                                  Ok(should) => should,
                                  Err(e) => {
                                    println!("[Auto Injection] Error checking party mode timing: {}", e);
                                    true // Default to inject if there's an error
                                  }
                                }
                              })
                            };

                            if !should_inject {
                              println!("[Auto Injection] Waiting for party friends to share before injecting champion {}", champ_id);
                              continue;
                            }

                            println!("[Auto Injection] Proceeding with injection for champion {} (skin change detected)", champ_id);

                            // Filter out skins whose skin_file files can't be found locally
                            let assets_skins_dir =
                              PathBuf::from(&league_path_clone).join("ASSETS/Skins");
                            let original_len = skins_to_inject.len();
                            let filtered_skins: Vec<Skin> = skins_to_inject
                                                        .into_iter()
                                                        .filter(|s| {
                                                            if let Some(ref fp_str) = s.skin_file_path {
                                                                let fp = PathBuf::from(fp_str);
                                                                let absolute_exists = fp.is_absolute() && fp.exists();
                                                                let exists_in_champions_rel = if fp.is_absolute() { false } else { champions_dir.join(&fp).exists() };
                                                                let exists_in_champions_name = fp.file_name()
                                                                    .map(|n| champions_dir.join(n).exists())
                                                                    .unwrap_or(false);
                                                                let exists_in_assets_rel = if fp.is_absolute() { false } else { assets_skins_dir.join(&fp).exists() };
                                                                let exists_in_assets_name = fp.file_name()
                                                                    .map(|n| assets_skins_dir.join(n).exists())
                                                                    .unwrap_or(false);
                                                                if absolute_exists || exists_in_champions_rel || exists_in_champions_name || exists_in_assets_rel || exists_in_assets_name {
                                                                    true
                                                                } else {
                                                                    println!("[Party Mode] Skipping skin (missing skin_file): champ={} skin={} path={}", s.champion_id, s.skin_id, fp_str);
                                                                    false
                                                                }
                                                            } else {
                                                                // If no path at all, skip to avoid aborting the whole injection
                                                                println!("[Party Mode] Skipping skin (no skin_file path): champ={} skin={} ", s.champion_id, s.skin_id);
                                                                false
                                                            }
                                                        })
                                                        .collect();
                            if filtered_skins.len() < original_len {
                              println!(
                              "[Party Mode] Filtered out {} skins without available skin_file files",
                              original_len - filtered_skins.len()
                            );
                            }

                            // Get selected misc items
                            let misc_items =
                              get_selected_misc_items(&app_handle).unwrap_or_else(|err| {
                                println!("Failed to get selected misc items: {}", err);
                                Vec::new()
                              });

                            match inject_skins_and_misc(
                              &app_handle,
                              &league_path_clone,
                              &filtered_skins,
                              &misc_items,
                              &champions_dir,
                            ) {
                              Ok(_) => {
                                let _ = app_handle.emit("injection-status", "success");
                                println!("[Enhanced] Successfully injected {} skins and {} misc items for champion {}",
                                                                     filtered_skins.len(), misc_items.len(), champ_id);
                              }
                              Err(e) => {
                                let _ = app_handle.emit(
                                  "skin-injection-error",
                                  format!(
                                    "Failed to inject skins and misc items for champion {}: {}",
                                    champ_id, e
                                  ),
                                );
                                let _ = app_handle.emit("injection-status", "error");
                              }
                            }

                            last_selected_skins.insert(champ_id, skin.clone());
                            skin_changes = true;
                          }
                        }
                      }

                      if skin_changes {
                        emit_terminal_log(
                          &app_handle,
                          "[LCU Watcher] Updated skin selection tracking",
                        );
                      }
                    }
                  }
                }
              }

              if phase != "ChampSelect" && phase != "None" && last_phase == "ChampSelect" {
                // Cleanup is now handled automatically by the phase change detection above
                // This removes redundant cleanup and improves performance
                let log_msg = "[LCU Watcher] Left ChampSelect phase - cleanup handled by phase change detection";
                println!("{}", log_msg);
                emit_terminal_log(&app_handle, log_msg);
                clear_received_skins();
                println!("[Party Mode] Cleared in-memory received skins after leaving ChampSelect");
                clear_sent_shares();
                // Reset dedup signature so the next phase can inject again
                if let Ok(mut g) = LAST_PARTY_INJECTION_SIGNATURE.lock() {
                  *g = None;
                }
                // Reset hard gate
                PARTY_INJECTION_DONE_THIS_PHASE.store(false, Ordering::Relaxed);
              }

              sleep_duration = Duration::from_secs(1);
            } else if phase == "InProgress" {
              // Keep existing in-game phase behavior
            }

            // Manual mode: also handle Lobby -> Matchmaking transition (no champ select modes)
            if last_phase == "Lobby"
              && phase == "Matchmaking"
              && crate::commands::skin_injection::is_manual_injection_active()
            {
              // Trigger manual injection using stored selections
              emit_terminal_log(&app_handle, "[LCU Watcher] Lobby->Matchmaking transition detected; manual injection mode active - triggering manual injection");
              let app_clone = app_handle.clone();
              std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                  match crate::commands::skin_injection::trigger_manual_injection(&app_clone).await
                  {
                    Ok(_) => {
                      println!(
                        "[LCU Watcher] Manual injection (instant-assign) completed successfully"
                      );
                    }
                    Err(e) => {
                      eprintln!(
                        "[LCU Watcher] Manual injection (instant-assign) failed: {}",
                        e
                      );
                    }
                  }
                });
              });
            }

            // Handle Lobby -> Matchmaking transition: try to resolve champion selections from
            // session and lobby endpoints and inject, regardless of injection_mode. This makes

            // instant-assign and similar lobby-selection modes more reliable without needing map codes.

            // Skip instant-assign injection if manual injection mode is active

            if last_phase == "Lobby"
              && phase == "Matchmaking"
              && !crate::commands::skin_injection::is_manual_injection_active()
            {
              // Check if we already handled this instant-assign phase to prevent duplicate injections
              let already_done = PARTY_INJECTION_DONE_THIS_PHASE.load(Ordering::Relaxed);
              if already_done {
                println!("[LCU Watcher][instant-assign] Injection already completed for this phase; skipping duplicate Lobby->Matchmaking detection");
              } else {
                // Mark as done immediately to prevent race conditions with rapid polling
                PARTY_INJECTION_DONE_THIS_PHASE.store(true, Ordering::Relaxed);

                // Reset outbound send dedup at the start of a new instant-assign phase
                clear_sent_shares();
                emit_terminal_log(&app_handle, "[LCU Watcher] Lobby->Matchmaking transition detected; resolving lobby-selected champions...");

                let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));
                let mut resolved_champions: Vec<i64> = Vec::new();

                // 1) Try gameflow session endpoint (may include gameData.playerChampionSelections)
                let session_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", port);
                if let Ok(resp) = client
                  .get(&session_url)
                  .header("Authorization", format!("Basic {}", auth))
                  .send()
                {
                  if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>() {
                      // Preferred: playerChampionSelections or selectedChampions
                      resolved_champions.extend(get_swift_play_champion_selections(&json));

                      if let Some(game_data) = json.get("gameData") {
                        if let Some(selected) = game_data
                          .get("selectedChampions")
                          .and_then(|s| s.as_array())
                        {
                          for sel in selected {
                            if let Some(cid) = sel.get("championId").and_then(|v| v.as_i64()) {
                              if cid > 0 && !resolved_champions.contains(&cid) {
                                resolved_champions.push(cid);
                              }
                            }
                          }
                        }
                        // Some responses include playerChampionSelections under gameData
                        if let Some(pcs) = game_data
                          .get("playerChampionSelections")
                          .and_then(|p| p.as_array())
                        {
                          for item in pcs {
                            if let Some(champs) = item.get("championIds").and_then(|c| c.as_array())
                            {
                              for c in champs {
                                if let Some(cid) = c.as_i64() {
                                  if cid > 0 && !resolved_champions.contains(&cid) {
                                    resolved_champions.push(cid);
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }

                // 2) If still empty, try the lobby endpoint which often contains localMember/playerSlots
                if resolved_champions.is_empty() {
                  let lobby_url_v2 = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
                  if let Ok(resp) = client
                    .get(&lobby_url_v2)
                    .header("Authorization", format!("Basic {}", auth))
                    .send()
                  {
                    if resp.status().is_success() {
                      if let Ok(lobby_json) = resp.json::<serde_json::Value>() {
                        let lobby_ids = extract_swift_play_champions_from_lobby(&lobby_json);
                        for id in lobby_ids {
                          if !resolved_champions.contains(&id) {
                            resolved_champions.push(id);
                          }
                        }
                      }
                    }
                  }
                }

                // 3) As an additional fallback, try /lol-lobby/v1/lobby and /lol-lobby/v1/members
                if resolved_champions.is_empty() {
                  let lobby_url_v1 = format!("https://127.0.0.1:{}/lol-lobby/v1/lobby", port);
                  if let Ok(resp) = client
                    .get(&lobby_url_v1)
                    .header("Authorization", format!("Basic {}", auth))
                    .send()
                  {
                    if resp.status().is_success() {
                      if let Ok(lobby_json) = resp.json::<serde_json::Value>() {
                        let ids = extract_swift_play_champions_from_lobby(&lobby_json);
                        for id in ids {
                          if !resolved_champions.contains(&id) {
                            resolved_champions.push(id);
                          }
                        }
                      }
                    }
                  }
                }

                // If we resolved any champions, trigger party-mode injection flow (which includes local skins)
                if !resolved_champions.is_empty() {
                  emit_terminal_log(
                    &app_handle,
                    &format!(
                      "[LCU Watcher] Resolved {} champion(s) from lobby/session: {:?}",
                      resolved_champions.len(),
                      resolved_champions
                    ),
                  );

                  // Party Mode: on instant-assign (Lobby->Matchmaking), send our shares now and inject
                  // friend + local skins for the resolved champions without waiting for ChampSelect.
                  // NOTE: We use ONLY party mode injection here (not solo) to avoid duplicate injections
                  let app_for_async = app_handle.clone();
                  let champs_u32: Vec<u32> = resolved_champions.iter().map(|c| *c as u32).collect();
                  std::thread::spawn(move || {
                    let rt =
                      tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                    rt.block_on(async move {
                                        // Send a share for each resolved champion if we have a local selection
                                        // Prefer official skin from config; fall back to custom skin file if any
                                        let config_dir = app_for_async.path().app_data_dir()
                                            .unwrap_or_else(|_| PathBuf::from("."))
                                            .join("config");
                                        let cfg_file = config_dir.join("config.json");
                                        if let Ok(data) = std::fs::read_to_string(&cfg_file) {
                                            if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
                                                for cid in &champs_u32 {
                                                    let mut sent_share = false;
                                                    if let Some(skin) = config.skins.iter().find(|s| s.champion_id == *cid) {
                                                        let _ = crate::commands::party_mode::send_skin_share_to_paired_friends(
                                                            &app_for_async,
                                                            skin.champion_id,
                                                            skin.skin_id,
                                                            skin.chroma_id,
                                                            skin.skin_file.clone(),
                                                        ).await.map(|_| { sent_share = true; () });
                                                    } else if let Some(custom) = config.custom_skins.iter().find(|s| s.champion_id == *cid) {
                                                        let _ = crate::commands::party_mode::send_skin_share_to_paired_friends(
                                                            &app_for_async,
                                                            custom.champion_id,
                                                            0,
                                                            None,
                                                            Some(custom.file_path.clone()),
                                                        ).await.map(|_| { sent_share = true; () });
                                                    }
                                                    if sent_share {
                                                        println!("[Party Mode][instant-assign] Sent skin_share for champion {} on Matchmaking", cid);
                                                    } else {
                                                        println!("[Party Mode][instant-assign] No local skin found to share for champion {}", cid);
                                                    }
                                                }
                                            }
                                        }

                                        // Wait briefly for friends to share before injecting (up to ~8s)
                                        let mut ready = false;
                                        let start = std::time::Instant::now();
                                        while start.elapsed() < std::time::Duration::from_secs(8) {
                                            match crate::commands::party_mode::should_inject_now(&app_for_async, *champs_u32.get(0).unwrap_or(&0)).await {
                                                Ok(true) => { ready = true; break; }
                                                Ok(false) => {
                                                    println!("[Party Mode][instant-assign] Waiting for friends to share before injection...");
                                                }
                                                Err(e) => {
                                                    println!("[Party Mode][instant-assign] should_inject_now error (proceeding): {}", e);
                                                    break;
                                                }
                                            }
                                            std::thread::sleep(std::time::Duration::from_millis(750));
                                        }
                                        if !ready { println!("[Party Mode][instant-assign] Proceeding without all shares after timeout"); }

                                        // Now try to inject using party-mode logic (includes local + any received friend skins)
                                        if let Err(e) = trigger_party_mode_injection_for_champions(&app_for_async, &champs_u32).await {
                                            eprintln!("[Party Mode][instant-assign] Injection failed: {}", e);
                                        }
                                    });
                  });
                } else {
                  emit_terminal_log(
                    &app_handle,
                    "[LCU Watcher] Could not resolve any champion selections from lobby/session",
                  );
                }
              } // Close the already_done else block
            }

            last_phase = phase.to_string();
            was_reconnecting = phase == "Reconnect";
            was_in_game = phase == "InProgress" || was_reconnecting;
            if phase == "ChampSelect" {
              PHASE_STATE.store(1, Ordering::Relaxed);
            } else {
              PHASE_STATE.store(2, Ordering::Relaxed);
            }
          }
          Err(e) => println!("Failed to build HTTP client: {}", e),
        }

        // Smart waiting: check file watcher channel or timeout
        // This provides instant response to lockfile changes while still doing periodic checks
        match lockfile_rx.recv_timeout(sleep_duration) {
          Ok(_) => {
            // File system event detected - drain any additional queued events to debounce
            while lockfile_rx.try_recv().is_ok() {
              // Discard rapid-fire events
            }
            // Small delay to let file operations complete
            thread::sleep(Duration::from_millis(100));
          }
          Err(mpsc::RecvTimeoutError::Timeout) => {
            // Normal timeout - continue with periodic check
          }
          Err(mpsc::RecvTimeoutError::Disconnected) => {
            // Watcher died - fall back to periodic checks only
            eprintln!("[LCU Watcher] File watcher disconnected, using polling only");
            thread::sleep(sleep_duration);
          }
        }
      }
    }
  });

  println!("LCU status watcher thread started");
  Ok(())
}
