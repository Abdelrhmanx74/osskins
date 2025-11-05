use crate::commands::misc_items::get_selected_misc_items;
use crate::commands::party_mode::{
  clear_received_skins, clear_sent_shares, PARTY_MODE_VERBOSE, RECEIVED_SKINS,
};
use crate::commands::types::{SavedConfig, SkinData};
use crate::injection::{inject_skins_and_misc, Skin};
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use copypasta::{ClipboardContext, ClipboardProvider};
use once_cell::sync::Lazy;
use serde_json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager};

// Injection mode selection – stored in config.json under "injection_mode"
#[derive(PartialEq, Eq)]
enum InjectionMode {
  ChampSelect,
  Lobby,
}

fn read_injection_mode(app: &AppHandle) -> InjectionMode {
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let cfg_file = config_dir.join("config.json");
  if let Ok(data) = std::fs::read_to_string(&cfg_file) {
    if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
      if let Some(mode_val) = cfg.get("injection_mode").and_then(|v| v.as_str()) {
        match mode_val.to_lowercase().as_str() {
          "lobby" | "lobby_mode" => return InjectionMode::Lobby,
          "champselect" | "champ_select" | "champselect_mode" | "champ" => {
            return InjectionMode::ChampSelect
          }
          _ => {}
        }
      }
    }
  }
  // Default to ChampSelect mode
  InjectionMode::ChampSelect
}

// LCU (League Client) watcher and communication

// 0 = Unknown, 1 = ChampSelect, 2 = Other
pub static PHASE_STATE: Lazy<AtomicU8> = Lazy::new(|| AtomicU8::new(0));

// Prevent repeated injections in the same ChampSelect phase
static LAST_PARTY_INJECTION_SIGNATURE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

// Hard gate: inject at most once per ChampSelect phase (prevents thrash when champion_id flips)
static PARTY_INJECTION_DONE_THIS_PHASE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Track last champion share time to debounce rapid ARAM rerolls
static LAST_CHAMPION_SHARE_TIME: Lazy<Mutex<HashMap<u32, std::time::Instant>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

fn compute_party_injection_signature(current_champion_id: u32) -> String {
  // Build a stable signature that includes the local locked champion id plus the
  // currently received friend skins. This ensures a champion change forces
  // injection even if the set of received friend skins didn't change.
  let map = RECEIVED_SKINS.lock().unwrap();
  let mut parts: Vec<String> = map
    .values()
    .map(|s| {
      format!(
        "{}:{}:{}:{}",
        s.from_summoner_id,
        s.champion_id,
        s.skin_id,
        s.chroma_id.unwrap_or(0)
      )
    })
    .collect();
  parts.sort();
  // Prefix with champion id so local champion selection influences the signature
  if parts.is_empty() {
    format!("champion:{}", current_champion_id)
  } else {
    format!("champion:{}|{}", current_champion_id, parts.join("|"))
  }
}

pub fn is_in_champ_select() -> bool {
  PHASE_STATE.load(Ordering::Relaxed) == 1
}

#[tauri::command]
pub fn start_lcu_watcher(app: AppHandle, league_path: String) -> Result<(), String> {
  println!("Starting LCU status watcher for path: {}", league_path);
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

    loop {
      let mut sleep_duration = Duration::from_secs(5);

      let log_msg = format!("[LCU Watcher] Monitoring directory: {}", league_path_clone);
      println!("{}", log_msg);
      emit_terminal_log(&app_handle, &log_msg);

      // Only check the configured League directory for lockfile
      let search_dirs = [PathBuf::from(&league_path_clone)];
      let mut port = None;
      let mut token = None;
      let mut found_any_lockfile = false;
      let mut lockfile_path = None;

      // Rest of the lockfile detection code remains the same
      for dir in &search_dirs {
        let log_msg = format!("[LCU Watcher] Looking for lockfiles in: {}", dir.display());
        println!("{}", log_msg);
        emit_terminal_log(&app_handle, &log_msg);

        // Check each possible lockfile name
        for name in [
          "lockfile",
          "LeagueClientUx.lockfile",
          "LeagueClient.lockfile",
        ] {
          let path = dir.join(name);
          if path.exists() {
            found_any_lockfile = true;
            lockfile_path = Some(path.clone());
            println!("[LCU Watcher] Found lockfile: {}", path.display());
            emit_terminal_log(
              &app_handle,
              &format!("[LCU Watcher] Found lockfile: {}", path.display()),
            );
          }
          if let Ok(content) = fs::read_to_string(&path) {
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

            // Check for party mode messages continuously (every 3 seconds) regardless of phase
            // This ensures we don't miss connection requests when not in champion select
            if last_party_mode_check.elapsed().as_secs() >= 3 {
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

            // Check if party mode injection should be triggered (every 2 seconds)
            // This runs independently of champion changes to handle the timing properly
            // Skip party mode injection if manual injection mode is active
            if phase == "ChampSelect"
              && last_party_injection_check.elapsed().as_secs() >= 2
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
                          if !last_selected_skins.contains_key(&champ_id)
                            || last_selected_skins.get(&champ_id).map_or(true, |old_skin| {
                              old_skin.skin_id != skin.skin_id
                                || old_skin.chroma_id != skin.chroma_id
                                || old_skin.skin_file != skin.skin_file
                            })
                          {
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

                            // Check if injection was already done this phase (prevents ARAM loops)
                            let already_done =
                              PARTY_INJECTION_DONE_THIS_PHASE.load(Ordering::Relaxed);
                            if already_done {
                              println!("[Party Mode] Injection already completed for this phase; skipping champion {}", champ_id);
                              continue;
                            }

                            // Check if we should inject now (wait for friends to share their skins)
                            let should_inject = {
                              // Create a runtime to handle the async call
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
                                    println!("[Party Mode] Error checking injection timing: {}", e);
                                    true // Default to inject if there's an error
                                  }
                                }
                              })
                            };

                            if !should_inject {
                              println!("[Party Mode] Delaying injection for champion {}, waiting for more friends to share", champ_id);
                              continue;
                            }

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

                // If we resolved any champions, inject (solo flow) and also trigger party-mode flow
                if !resolved_champions.is_empty() {
                  emit_terminal_log(
                    &app_handle,
                    &format!(
                      "[LCU Watcher] Resolved {} champion(s) from lobby/session: {:?}",
                      resolved_champions.len(),
                      resolved_champions
                    ),
                  );
                  // Solo instant-assign injection (kept as-is)
                  inject_skins_for_champions(&app_handle, &league_path_clone, &resolved_champions);

                  // Party Mode: on instant-assign (Lobby->Matchmaking), send our shares now and try to inject
                  // friend + local skins for the resolved champions without waiting for ChampSelect.
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

        thread::sleep(sleep_duration);
      }
    }
  });

  println!("LCU status watcher thread started");
  Ok(())
}

fn emit_terminal_log(_app: &AppHandle, message: &str) {
  // Buffer logs in memory for later printing. Keep emitting to stdout for backwards visibility.
  println!("{}", message);
  if let Ok(mut buf) = LOG_BUFFER.lock() {
    buf.push(message.to_string());
    // Keep buffer bounded to last 2000 lines
    if buf.len() > 2000 {
      let excess = buf.len() - 2000;
      buf.drain(0..excess);
    }
  }

  // Also append to an on-disk live log so we can export the full runtime log later.
  if let Ok(app_dir) = _app.path().app_data_dir() {
    let logs_dir = app_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
      // Non-fatal: we still keep logs in memory
      println!("[LCU Watcher] Failed to ensure logs dir exists: {}", e);
      return;
    }

    let live_log = logs_dir.join("osskins-live.log");
    // Try to append; ignore failures to avoid crashing the watcher thread
    if let Ok(mut f) = File::options().create(true).append(true).open(&live_log) {
      if let Err(e) = writeln!(f, "{}", message) {
        println!("[LCU Watcher] Failed to write to live log: {}", e);
      }
    } else {
      // Could not open the file - non-fatal
      // Keep in-memory buffer as fallback
    }
  }
}

// Global in-memory log buffer
static LOG_BUFFER: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Append a message to the global in-memory buffer and on-disk live log.
/// This is public so other backend modules can call it to ensure consistent logging.
pub fn append_global_log(message: &str) {
  // Print to stdout for normal visibility
  println!("{}", message);

  // Append to in-memory buffer (bounded)
  if let Ok(mut buf) = LOG_BUFFER.lock() {
    buf.push(message.to_string());
    if buf.len() > 2000 {
      let excess = buf.len() - 2000;
      buf.drain(0..excess);
    }
  }

  // Append to on-disk live log using APPDATA fallback if necessary
  let logs_dir = std::env::var("APPDATA")
    .map(|ap| PathBuf::from(ap).join("com.osskins.app").join("logs"))
    .unwrap_or_else(|_| PathBuf::from(".").join("logs"));

  if let Err(e) = std::fs::create_dir_all(&logs_dir) {
    // Non-fatal
    eprintln!("[LCU Watcher] Failed to ensure logs dir exists: {}", e);
    return;
  }

  let live_log = logs_dir.join("osskins-live.log");
  if let Ok(mut f) = File::options().create(true).append(true).open(&live_log) {
    if let Err(e) = writeln!(f, "{}", message) {
      eprintln!("[LCU Watcher] Failed to write to live log: {}", e);
    }
  }
}

/// Write buffered logs to a timestamped temp file and copy contents to clipboard.
#[tauri::command]
pub fn print_logs(app: AppHandle) -> Result<String, String> {
  // Prefer the on-disk live log (captures everything appended since app start).
  let app_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."));
  let out_dir = app_dir.join("logs");
  if let Err(e) = std::fs::create_dir_all(&out_dir) {
    return Err(format!("Failed to create log dir: {}", e));
  }

  let live_log = out_dir.join("osskins-live.log");

  // Read from the live log file if it exists and is readable. Otherwise fall back to in-memory buffer.
  let full_contents = if live_log.exists() {
    match std::fs::read_to_string(&live_log) {
      Ok(s) if !s.is_empty() => s,
      _ => {
        // Fall back to buffer
        let buf = LOG_BUFFER
          .lock()
          .map_err(|e| format!("Lock error: {}", e))?;
        if buf.is_empty() {
          return Err("No logs available".to_string());
        }
        buf.join("\n")
      }
    }
  } else {
    let buf = LOG_BUFFER
      .lock()
      .map_err(|e| format!("Lock error: {}", e))?;
    if buf.is_empty() {
      return Err("No logs available".to_string());
    }
    buf.join("\n")
  };

  // Write exported timestamped copy
  let filename = format!("osskins-logs-{}.txt", Utc::now().format("%Y%m%d-%H%M%S"));
  let out_path = out_dir.join(&filename);
  let mut file = File::create(&out_path).map_err(|e| format!("Failed to create file: {}", e))?;
  if let Err(e) = write!(file, "{}", full_contents) {
    return Err(format!("Failed to write logs: {}", e));
  }

  // Copy to clipboard
  let mut ctx = ClipboardContext::new().map_err(|e| format!("Clipboard init failed: {}", e))?;
  ctx
    .set_contents(full_contents)
    .map_err(|e| format!("Clipboard write failed: {}", e))?;

  Ok(out_path.to_string_lossy().to_string())
}

// Add helper function for cleaner log messages
#[allow(dead_code)]
fn format_json_summary(json: &serde_json::Value) -> String {
  let mut summary = String::new();

  if let Some(phase) = json.get("phase") {
    summary.push_str(&format!("phase: {}, ", phase.as_str().unwrap_or("unknown")));
  }

  if let Some(_game_data) = json.get("gameData") {
    summary.push_str("gameData: {...}, ");
  }

  if let Some(actions) = json.get("actions") {
    summary.push_str(&format!(
      "actions: [{} items], ",
      actions.as_array().map_or(0, |a| a.len())
    ));
  }

  if summary.is_empty() {
    summary = "[Response summary unavailable]".to_string();
  }

  summary
}

// Helper function for delayed logging
#[allow(dead_code)]
fn delayed_log(app: &AppHandle, message: &str) {
  emit_terminal_log(app, message);
  thread::sleep(Duration::from_millis(100)); // Small delay for better readability
}

// Helper function to get selected champion ID from session JSON
fn get_selected_champion_id(session_json: &serde_json::Value) -> Option<i64> {
  // Get local player cell ID
  if let Some(local_player_cell_id) = session_json
    .get("localPlayerCellId")
    .and_then(|v| v.as_i64())
  {
    // First, find our current active action
    if let Some(actions) = session_json.get("actions").and_then(|v| v.as_array()) {
      // Track if we found any pick in progress
      let mut has_pick_in_progress = false;

      // First pass: check if we have any pick in progress
      for action_group in actions.iter() {
        if let Some(actions) = action_group.as_array() {
          for action in actions {
            if let Some(actor_cell_id) = action.get("actorCellId").and_then(|v| v.as_i64()) {
              if actor_cell_id == local_player_cell_id {
                let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let is_in_progress = action
                  .get("isInProgress")
                  .and_then(|v| v.as_bool())
                  .unwrap_or(false);

                if action_type == "pick" && is_in_progress {
                  has_pick_in_progress = true;
                  break;
                }
              }
            }
          }
        }
      }

      // If we have a pick in progress, don't return any champion ID
      if has_pick_in_progress {
        println!(
          "[LCU Watcher][DEBUG] Local pick is in progress; deferring champion ID resolution"
        );
        return None;
      }

      // Second pass: look for completed pick
      for action_group in actions {
        if let Some(actions) = action_group.as_array() {
          for action in actions {
            if let Some(actor_cell_id) = action.get("actorCellId").and_then(|v| v.as_i64()) {
              if actor_cell_id == local_player_cell_id {
                let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let is_completed = action
                  .get("completed")
                  .and_then(|v| v.as_bool())
                  .unwrap_or(false);
                let champion_id = action
                  .get("championId")
                  .and_then(|v| v.as_i64())
                  .unwrap_or(0);

                // Only return champion ID if:
                // 1. It's a pick action (not ban)
                // 2. Action is completed
                // 3. Valid champion ID
                if action_type == "pick" && is_completed && champion_id > 0 {
                  println!(
                    "[LCU Watcher][DEBUG] Found completed pick for local player: champion_id={}",
                    champion_id
                  );
                  return Some(champion_id);
                }
              }
            }
          }
        }
      }
    }

    // As a backup, check myTeam data: treat a valid championId as assigned (covers ARAM/instant-assign modes).
    if let Some(my_team) = session_json.get("myTeam").and_then(|v| v.as_array()) {
      for player in my_team {
        if let Some(cell_id) = player.get("cellId").and_then(|v| v.as_i64()) {
          if cell_id == local_player_cell_id {
            let champion_id = player
              .get("championId")
              .and_then(|v| v.as_i64())
              .unwrap_or(0);
            // Consider selected if we have a valid champion id (even if intent is set).
            // We already checked that no pick is in progress above, so this is safe and
            // lets ARAM/instant-assign modes share immediately upon assignment.
            if champion_id > 0 {
              println!("[LCU Watcher][DEBUG] myTeam shows championId={}; treating as assigned (ARAM/instant)", champion_id);
              return Some(champion_id);
            }
          }
        }
      }
    }
  }
  println!("[LCU Watcher][DEBUG] No completed pick found for local player yet");
  None
}

// Create a persistent HTTP client to avoid recreating it every time
fn get_lcu_client() -> reqwest::blocking::Client {
  static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

  CLIENT
    .get_or_init(|| {
      reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
    .clone()
}

// Helper function to get instant-assign champion selections from session JSON
fn get_swift_play_champion_selections(json: &serde_json::Value) -> Vec<i64> {
  let mut champion_ids = Vec::new();

  // Method 1: Look in gameData -> playerChampionSelections
  if let Some(game_data) = json.get("gameData") {
    if let Some(selections) = game_data
      .get("playerChampionSelections")
      .and_then(|p| p.as_array())
    {
      // Get local player's summoner ID first
      let local_summoner_id = json
        .get("localPlayerSelection")
        .and_then(|lp| lp.get("summonerId"))
        .and_then(|id| id.as_i64());

      if let Some(local_id) = local_summoner_id {
        for selection in selections {
          // Check if this is the local player
          if let Some(player_id) = selection.get("summonerId").and_then(|id| id.as_i64()) {
            if player_id == local_id {
              // Extract champion IDs
              if let Some(champs) = selection.get("championIds").and_then(|ids| ids.as_array()) {
                for champ in champs {
                  if let Some(id) = champ.as_i64() {
                    if id > 0 && !champion_ids.contains(&id) {
                      champion_ids.push(id);
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

  // Method 2: Look in gameData -> selectedChampions
  if champion_ids.is_empty() {
    if let Some(game_data) = json.get("gameData") {
      if let Some(selected_champions) = game_data
        .get("selectedChampions")
        .and_then(|sc| sc.as_array())
      {
        for selection in selected_champions {
          if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
            if champion_id > 0 && !champion_ids.contains(&champion_id) {
              champion_ids.push(champion_id);
            }
          }
        }
      }
    }
  }

  // Method 3: Look in the player's team data
  if champion_ids.is_empty() {
    if let Some(team) = json.get("myTeam").and_then(|t| t.as_array()) {
      let player_name = json
        .get("playerName")
        .and_then(|p| p.as_str())
        .unwrap_or("");

      for player in team {
        let is_local_player = player
          .get("summonerName")
          .and_then(|n| n.as_str())
          .map_or(false, |name| name == player_name);

        if is_local_player {
          // Primary champion
          if let Some(champion_id) = player.get("championId").and_then(|id| id.as_i64()) {
            if champion_id > 0 && !champion_ids.contains(&champion_id) {
              champion_ids.push(champion_id);
            }
          }

          // Secondary champion
          if let Some(secondary_id) = player.get("secondaryChampionId").and_then(|id| id.as_i64()) {
            if secondary_id > 0 && !champion_ids.contains(&secondary_id) {
              champion_ids.push(secondary_id);
            }
          }
        }
      }
    }
  }

  // Try one more method for instant-assign
  if champion_ids.is_empty() {
    if let Some(roles) = json.get("roleAssignments").and_then(|r| r.as_array()) {
      for role in roles {
        if let Some(champion_id) = role.get("championId").and_then(|id| id.as_i64()) {
          if champion_id > 0 && !champion_ids.contains(&champion_id) {
            champion_ids.push(champion_id);
          }
        }
      }
    }
  }

  // Method 4: Check lobby data playerSlots for instant-assign
  if champion_ids.is_empty() {
    // Try to find champions in localMember.playerSlots (common in instant-assign)
    if let Some(local_member) = json.get("localMember") {
      if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
        for slot in player_slots {
          if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
            if champion_id > 0 && !champion_ids.contains(&champion_id) {
              champion_ids.push(champion_id);
            }
          }
        }
      }
    }
  }

  champion_ids
}

// Helper function to inject skins for multiple champions (used in instant-assign)
fn inject_skins_for_champions(app: &AppHandle, league_path: &str, champion_ids: &[i64]) {
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let cfg_file = config_dir.join("config.json");

  // Check if we have config with skin selections
  if let Ok(data) = std::fs::read_to_string(&cfg_file) {
    if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
      // Get all skins for the selected champions (both official and custom)
      let mut skins_to_inject = Vec::new();

      for champ_id in champion_ids {
        let champ_id_u32 = *champ_id as u32;

        // Check for official skin first
        if let Some(skin) = config.skins.iter().find(|s| s.champion_id == champ_id_u32) {
          skins_to_inject.push(Skin {
            champion_id: skin.champion_id,
            skin_id: skin.skin_id,
            chroma_id: skin.chroma_id,
            skin_file_path: skin.skin_file.clone(),
          });
        }
        // If no official skin, check for custom skin
        else if let Some(custom_skin) = config
          .custom_skins
          .iter()
          .find(|s| s.champion_id == champ_id_u32)
        {
          // For custom skins, use skin_id = 0 and the custom file path as skin_file_path
          skins_to_inject.push(Skin {
            champion_id: custom_skin.champion_id,
            skin_id: 0,      // Custom skins use skin_id 0
            chroma_id: None, // Custom skins don't have chromas
            skin_file_path: Some(custom_skin.file_path.clone()),
          });
        }
      }

      // Get selected misc items
      let misc_items = get_selected_misc_items(app).unwrap_or_else(|err| {
        println!("Failed to get selected misc items: {}", err);
        Vec::new()
      });

      // If we found skins to inject or misc items, do it
      if !skins_to_inject.is_empty() || !misc_items.is_empty() {
        let champions_dir = app
          .path()
          .app_data_dir()
          .unwrap_or_else(|_| PathBuf::from("."))
          .join("champions");

        match inject_skins_and_misc(
          app,
          league_path,
          &skins_to_inject,
          &misc_items,
          &champions_dir,
        ) {
          Ok(_) => {
            let _ = app.emit("injection-status", "success");
            if !skins_to_inject.is_empty() {
              println!(
                "[Enhanced] Successfully injected {} skins and {} misc items",
                skins_to_inject.len(),
                misc_items.len()
              );
            }
          }
          Err(e) => {
            let _ = app.emit(
              "skin-injection-error",
              format!(
                "Failed to inject instant-assign skins and misc items: {}",
                e
              ),
            );
            let _ = app.emit("injection-status", "error");
          }
        }
      }
    }
  }
}

// Extract instant-assign champion IDs from the lobby data directly
fn extract_swift_play_champions_from_lobby(json: &serde_json::Value) -> Vec<i64> {
  let mut champion_ids = Vec::new();

  if let Some(local_member) = json.get("localMember") {
    if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
      for slot in player_slots {
        if let Some(champion_id) = slot.get("championId").and_then(|id| id.as_i64()) {
          if champion_id > 0 && !champion_ids.contains(&champion_id) {
            champion_ids.push(champion_id);
          }
        }
      }
    }
  }

  champion_ids
}

// Start monitoring LCU chat messages for party mode
#[tauri::command]
pub fn start_party_mode_chat_monitor(_app: AppHandle) -> Result<(), String> {
  // Party mode monitoring is now integrated into the main LCU watcher
  // This command is kept for backward compatibility but doesn't start a separate thread
  println!("Party mode chat monitoring is integrated into the main LCU watcher");
  Ok(())
}

// Check for party mode messages using existing connection info
fn check_for_party_mode_messages_with_connection(
  app: &AppHandle,
  port: &str,
  token: &str,
  processed_message_ids: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
  let client = get_lcu_client();
  let url = format!("https://127.0.0.1:{}/lol-chat/v1/conversations", port);
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

  println!(
    "[Party Mode][DEBUG] Fetching conversations for OSS scan: {}",
    url
  );
  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .map_err(|e| format!("Failed to get conversations: {}", e))?;

  if !response.status().is_success() {
    return Ok(());
  }

  let conversations: serde_json::Value = response
    .json()
    .map_err(|e| format!("Failed to parse conversations: {}", e))?;

  if let Some(conversations_array) = conversations.as_array() {
    println!(
      "[Party Mode][DEBUG] Conversations found: {}",
      conversations_array.len()
    );
    for conversation in conversations_array {
      if let Some(conversation_id) = conversation.get("id").and_then(|id| id.as_str()) {
        let pid = conversation
          .get("pid")
          .and_then(|v| v.as_str())
          .unwrap_or("");
        println!(
          "[Party Mode][DEBUG] Scanning conversation id={} pid={}",
          conversation_id, pid
        );
        if let Err(e) = check_conversation_for_party_messages(
          app,
          &client,
          port,
          token,
          conversation_id,
          processed_message_ids,
        ) {
          eprintln!("Error checking conversation {}: {}", conversation_id, e);
        }
      }
    }
  }

  Ok(())
}

// Check a specific conversation for party mode messages
fn check_conversation_for_party_messages(
  app: &AppHandle,
  client: &reqwest::blocking::Client,
  port: &str,
  token: &str,
  conversation_id: &str,
  processed_message_ids: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
  let url = format!(
    "https://127.0.0.1:{}/lol-chat/v1/conversations/{}/messages",
    port, conversation_id
  );
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

  println!(
    "[Party Mode][DEBUG] Fetching messages for conversation: {}",
    url
  );
  let response = client
    .get(&url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
    .map_err(|e| format!("Failed to get messages: {}", e))?;

  if !response.status().is_success() {
    return Ok(());
  }

  let messages: serde_json::Value = response
    .json()
    .map_err(|e| format!("Failed to parse messages: {}", e))?;

  if let Some(messages_array) = messages.as_array() {
    println!(
      "[Party Mode][DEBUG] Messages count in conversation: {}",
      messages_array.len()
    );
    // Check all messages, not just recent ones, but skip already processed messages
    for message in messages_array.iter() {
      // Get message ID to track processed messages (support string or numeric IDs)
      let message_id = message
        .get("id")
        .and_then(|id| {
          id.as_str()
            .map(|s| s.to_string())
            .or_else(|| id.as_u64().map(|n| n.to_string()))
        })
        .unwrap_or_else(|| "unknown".to_string());

      // Skip if we've already processed this message
      if processed_message_ids.contains(&message_id) {
        // skip silently, but we can log verbose when debugging
        continue;
      }

      let body = message.get("body").and_then(|b| b.as_str());
      let from_summoner_id = message
        .get("fromSummonerId")
        .and_then(|id| id.as_str())
        .or_else(|| message.get("fromId").and_then(|id| id.as_str()))
        .or_else(|| message.get("senderId").and_then(|id| id.as_str()));

      if let (Some(body), Some(from_summoner_id)) = (body, from_summoner_id) {
        // Only print debug info for OSS messages to reduce noise
        if body.starts_with("OSS:") {
          println!(
            "[Party Mode] Found OSS message from {}: {}",
            from_summoner_id, body
          );
          println!(
            "[Party Mode][DEBUG] Marking message id={} as processed",
            message_id
          );

          // Mark this message as processed before handling it
          processed_message_ids.insert(message_id);

          let rt = tokio::runtime::Runtime::new().unwrap();
          if let Err(e) = rt.block_on(crate::commands::party_mode::handle_party_mode_message(
            app,
            body,
            from_summoner_id,
          )) {
            eprintln!("Error handling party mode message: {}", e);
          }
        }
      } else {
        // Debug: Check what fields are available in the message
        if message.as_object().is_some() {
          let available_fields: Vec<String> =
            message.as_object().unwrap().keys().cloned().collect();
          println!(
            "[Party Mode] Debug: Message has fields: {:?}",
            available_fields
          );
        }
      }
    }

    // Clean up old message IDs to prevent memory growth
    // Keep only the last 100 message IDs
    if processed_message_ids.len() > 100 {
      let mut ids_vec: Vec<String> = processed_message_ids.iter().cloned().collect();
      ids_vec.sort(); // Not perfect ordering, but good enough for cleanup
      let keep_count = 50;
      processed_message_ids.clear();
      for id in ids_vec.into_iter().rev().take(keep_count) {
        processed_message_ids.insert(id);
      }
      println!(
        "[Party Mode][DEBUG] Processed IDs trimmed to {}",
        processed_message_ids.len()
      );
    }
  } else {
    println!(
      "[Party Mode] No messages array found in response for conversation {}",
      conversation_id
    );
  }

  Ok(())
}

// Function to trigger immediate injection of all friend skins (called from party mode)
pub async fn trigger_party_mode_injection(app: &AppHandle, champion_id: u32) -> Result<(), String> {
  // There is a small race between the phase update and the party-mode trigger where
  // the global PHASE_STATE may not yet have been updated to ChampSelect. To avoid
  // missing injections because of this, log a warning but allow the injection to
  // proceed. The surrounding logic already ensures we only attempt injection when
  // a locked champion and friend-sharing conditions are met.
  if !is_in_champ_select() {
    println!("[Party Mode] Warning: trigger_party_mode_injection called but PHASE_STATE is not ChampSelect; proceeding due to possible race");
  }

  println!("[Party Mode] Triggering immediate injection of all friend skins...");

  // Get config and misc items (same as the main injection logic)
  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");

  if !config_file.exists() {
    return Err("Config file not found".to_string());
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;

  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  let misc_items = get_selected_misc_items(app)?;
  let league_path = config
    .league_path
    .as_ref()
    .ok_or("League path not configured".to_string())?;

  // Prefer the app data champions directory as the canonical source of skin_file files
  let champions_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("champions");
  if !champions_dir.exists() {
    if let Err(e) = std::fs::create_dir_all(&champions_dir) {
      println!("[Party Mode] Failed to create champions directory: {}", e);
    }
  }
  // Keep League's ASSETS/Skins as a last-resort fallback only
  let assets_skins_dir = PathBuf::from(league_path).join("ASSETS/Skins");

  // Collect friend skins from received skins; keep friend identity alongside skin
  let mut skins_with_source: Vec<(Skin, Option<String>)> = Vec::new();
  let mut local_skin_added = false;
  let received_skins_map = RECEIVED_SKINS.lock().unwrap();

  // Add the local selected skin for the locked champion so we always inject our own selection
  if champion_id != 0 {
    if let Some(local_skin) = config.skins.iter().find(|s| s.champion_id == champion_id) {
      skins_with_source.push((
        Skin {
          champion_id: local_skin.champion_id,
          skin_id: local_skin.skin_id,
          chroma_id: local_skin.chroma_id,
          skin_file_path: local_skin.skin_file.clone(),
        },
        None,
      ));
      local_skin_added = true;
      println!(
        "[Party Mode][DEBUG] Added local skin for champion {} to injection list",
        champion_id
      );
    } else {
      println!(
        "[Party Mode][DEBUG] No local skin selected for champion {}",
        champion_id
      );
    }
  }

  for received_skin in received_skins_map.values() {
    if let Some(skin_file_path) = &received_skin.skin_file_path {
      // Debug: show what we received from friend
      println!(
        "[Party Mode][DEBUG] Received skin_file_path='{}' from {}",
        skin_file_path, received_skin.from_summoner_name
      );

      // Normalize the path string for easier matching (use forward slashes)
      let fp_raw = skin_file_path.clone();
      let fp_norm = fp_raw.replace('\\', "/");
      // Create a PathBuf for the original string and a "relative-friendly" one without leading slash
      let fp = PathBuf::from(&fp_raw);
      let fp_rel = PathBuf::from(fp_norm.trim_start_matches('/'));

      let mut found_path: Option<PathBuf> = None;

      // 1) Direct absolute exists
      if fp.is_absolute() && fp.exists() {
        println!(
          "[Party Mode][DEBUG] Using absolute skin_file from friend: {}",
          fp.display()
        );
        found_path = Some(fp.clone());
      }

      // 2) Map known portable prefixes (e.g., /ezrea/) under champions_dir even if not treated as absolute on Windows
      if found_path.is_none() {
        let fp_str = fp_norm.as_str();
        // Support both '/ezrea/...' and 'ezrea/...'
        if fp_str.starts_with("/ezrea/") || fp_str.starts_with("ezrea/") {
          let tail = if fp_str.starts_with("/ezrea/") {
            &fp_str["/ezrea/".len()..]
          } else {
            &fp_str["ezrea/".len()..]
          };
          let mapped = champions_dir.join(tail);
          if mapped.exists() {
            println!(
              "[Party Mode][DEBUG] Mapped /ezrea path to champions dir: {}",
              mapped.display()
            );
            found_path = Some(mapped);
          } else {
            if let Some(base) = PathBuf::from(tail).file_name() {
              let by_name = champions_dir.join(base);
              if by_name.exists() {
                println!(
                  "[Party Mode][DEBUG] Fallback by basename in champions dir: {}",
                  by_name.display()
                );
                found_path = Some(by_name);
              }
            }
          }
        } else if fp_str.starts_with('/') {
          // Generic: drop the leading root and try under champions_dir
          let rel_parts: PathBuf = fp_rel.iter().collect();
          let mapped = champions_dir.join(&rel_parts);
          if mapped.exists() {
            println!(
              "[Party Mode][DEBUG] Generic mapped absolute to champions dir: {}",
              mapped.display()
            );
            found_path = Some(mapped);
          }
        }
      }

      // Helper: try variants in a directory - try both original and relative-friendly src
      let try_in_dir = |dir: &PathBuf, src: &PathBuf| -> Option<PathBuf> {
        // If relative src, try full relative join
        if !src.is_absolute() {
          let rel = dir.join(src);
          if rel.exists() {
            return Some(rel);
          }
        }
        // Try by basename
        if let Some(name) = src.file_name() {
          let by_name = dir.join(name);
          if by_name.exists() {
            return Some(by_name);
          }
          if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
            for ext in ["zip", "skin_file"] {
              let cand = dir.join(format!("{}.{}", stem, ext));
              if cand.exists() {
                return Some(cand);
              }
            }
          }
        }
        None
      };

      // 3) Check champions and assets dirs with variants (try both fp and fp_rel)
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&champions_dir, &fp_rel) {
          println!(
            "[Party Mode][DEBUG] Found in champions dir variants: {}",
            p.display()
          );
          found_path = Some(p);
        }
      }
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&champions_dir, &fp) {
          println!(
            "[Party Mode][DEBUG] Found in champions dir variants: {}",
            p.display()
          );
          found_path = Some(p);
        }
      }
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&assets_skins_dir, &fp_rel) {
          println!(
            "[Party Mode][DEBUG] Found in ASSETS/Skins variants: {}",
            p.display()
          );
          found_path = Some(p);
        }
      }
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&assets_skins_dir, &fp) {
          println!(
            "[Party Mode][DEBUG] Found in ASSETS/Skins variants: {}",
            p.display()
          );
          found_path = Some(p);
        }
      }

      // 4) Shallow recursive search by stem
      if found_path.is_none() {
        if let Some(stem) = fp.file_stem().and_then(|s| s.to_str()) {
          for dir in [&champions_dir, &assets_skins_dir] {
            if let Ok(mut rd) = std::fs::read_dir(dir) {
              while let Some(Ok(entry)) = rd.next() {
                let path = entry.path();
                if path.is_dir() {
                  if let Ok(mut inner) = std::fs::read_dir(&path) {
                    while let Some(Ok(e2)) = inner.next() {
                      let p2 = e2.path();
                      if p2.is_file() {
                        if let Some(s) = p2.file_stem().and_then(|x| x.to_str()) {
                          if s.eq_ignore_ascii_case(stem) {
                            println!(
                              "[Party Mode][DEBUG] Found by shallow scan in {}: {}",
                              dir.display(),
                              p2.display()
                            );
                            found_path = Some(p2.clone());
                            break;
                          }
                        }
                      }
                    }
                  }
                } else if path.is_file() {
                  if let Some(s) = path.file_stem().and_then(|x| x.to_str()) {
                    if s.eq_ignore_ascii_case(stem) {
                      println!(
                        "[Party Mode][DEBUG] Found by file match in {}: {}",
                        dir.display(),
                        path.display()
                      );
                      found_path = Some(path.clone());
                      break;
                    }
                  }
                }
                if found_path.is_some() {
                  break;
                }
              }
            }
            if found_path.is_some() {
              break;
            }
          }
        }
      }

      // 5) Fallback: use local config skin mapping for same champion_id and skin_id
      if found_path.is_none() {
        if let Some(local_skin) = config.skins.iter().find(|s| {
          s.champion_id == received_skin.champion_id && s.skin_id == received_skin.skin_id
        }) {
          if let Some(ref f) = local_skin.skin_file {
            let cand_abs = PathBuf::from(f);
            let mut candidates: Vec<PathBuf> = Vec::new();
            if cand_abs.is_absolute() {
              candidates.push(cand_abs.clone());
            }
            candidates.push(champions_dir.join(&cand_abs));
            if let Some(name) = cand_abs.file_name() {
              candidates.push(champions_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                candidates.push(champions_dir.join(format!("{}.zip", stem)));
                candidates.push(champions_dir.join(format!("{}.skin_file", stem)));
              }
            }
            candidates.push(assets_skins_dir.join(&cand_abs));
            if let Some(name) = cand_abs.file_name() {
              candidates.push(assets_skins_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                candidates.push(assets_skins_dir.join(format!("{}.zip", stem)));
                candidates.push(assets_skins_dir.join(format!("{}.skin_file", stem)));
              }
            }
            if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
              println!(
                "[Party Mode][DEBUG] Fallback matched local config skin: {}",
                found.display()
              );
              found_path = Some(found);
            }
          }
        }
      }

      // 6) Last-resort fallback: if still not found, use ANY local skin selected for the same champion
      // This ensures we still inject something visually for that friend/champion even if their exact skin isn't available locally
      if found_path.is_none() {
        // Prefer official selection, else custom skin
        if let Some(local_skin_any) = config
          .skins
          .iter()
          .find(|s| s.champion_id == received_skin.champion_id)
        {
          if let Some(ref f) = local_skin_any.skin_file {
            let cand_abs = PathBuf::from(f);
            let mut candidates: Vec<PathBuf> = Vec::new();
            if cand_abs.is_absolute() {
              candidates.push(cand_abs.clone());
            }
            candidates.push(champions_dir.join(&cand_abs));
            if let Some(name) = cand_abs.file_name() {
              candidates.push(champions_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                candidates.push(champions_dir.join(format!("{}.zip", stem)));
                candidates.push(champions_dir.join(format!("{}.skin_file", stem)));
              }
            }
            candidates.push(assets_skins_dir.join(&cand_abs));
            if let Some(name) = cand_abs.file_name() {
              candidates.push(assets_skins_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                candidates.push(assets_skins_dir.join(format!("{}.zip", stem)));
                candidates.push(assets_skins_dir.join(format!("{}.skin_file", stem)));
              }
            }
            if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
              println!(
                "[Party Mode][DEBUG] Fallback mapped by champion {} to local selection: {}",
                received_skin.champion_id,
                found.display()
              );
              found_path = Some(found);
            }
          }
        } else if let Some(custom_any) = config
          .custom_skins
          .iter()
          .find(|s| s.champion_id == received_skin.champion_id)
        {
          let cand_abs = PathBuf::from(&custom_any.file_path);
          if cand_abs.exists() {
            println!(
              "[Party Mode][DEBUG] Fallback mapped by champion {} to local custom skin: {}",
              received_skin.champion_id,
              cand_abs.display()
            );
            found_path = Some(cand_abs);
          } else {
            let cand_rel = champions_dir.join(&cand_abs);
            if cand_rel.exists() {
              found_path = Some(cand_rel);
            }
          }
        }
      }

      if let Some(resolved) = found_path {
        skins_with_source.push((
          Skin {
            champion_id: received_skin.champion_id,
            skin_id: received_skin.skin_id,
            chroma_id: received_skin.chroma_id,
            // Use resolved local path for reliability
            skin_file_path: Some(resolved.to_string_lossy().to_string()),
          },
          Some(received_skin.from_summoner_id.clone()),
        ));
        println!(
          "[Party Mode] Adding friend skin from {} for injection: Champion {}, Skin {}",
          received_skin.from_summoner_name, received_skin.champion_id, received_skin.skin_id
        );
      } else {
        println!(
          "[Party Mode] Skipping friend skin from {} (missing skin_file: {})",
          received_skin.from_summoner_name, skin_file_path
        );
        println!(
          "[Party Mode][DEBUG] Champions dir: {}",
          champions_dir.display()
        );
        println!(
          "[Party Mode][DEBUG] Assets skins dir: {}",
          assets_skins_dir.display()
        );
        println!(
          "[Party Mode][DEBUG] Tried to resolve from: {}",
          fp.display()
        );
      }
    }
  }

  // Release the lock
  drop(received_skins_map);

  if skins_with_source.is_empty() {
    println!("[Party Mode] No skins (local or friend) with available files to inject");
    return Ok(());
  }

  // Deduplicate, but keep distinct entries per friend. Include friend_id in key.
  let mut seen: HashSet<(u32, u32, Option<u32>, Option<String>, Option<String>)> = HashSet::new();
  let mut skins_to_inject: Vec<Skin> = Vec::new();
  for (s, friend_id_opt) in skins_with_source.into_iter() {
    let key = (
      s.champion_id,
      s.skin_id,
      s.chroma_id,
      s.skin_file_path.clone(),
      friend_id_opt.clone(),
    );
    if seen.insert(key) {
      skins_to_inject.push(s);
    }
  }

  // Inject friend skins using the same logic as the main injection
  match inject_skins_and_misc(
    app,
    league_path,
    &skins_to_inject,
    &misc_items,
    &champions_dir,
  ) {
    Ok(_) => {
      let total = skins_to_inject.len();
      let friend_count = if local_skin_added {
        total.saturating_sub(1)
      } else {
        total
      };
      println!(
        "[Party Mode] ✅ Successfully injected {} skins ({} friend skins, {} local)",
        total,
        friend_count,
        if local_skin_added { 1 } else { 0 }
      );

      // Log details of each injected skin
      for skin in skins_to_inject.iter() {
        println!(
          "[Party Mode] 🎨 Injected skin: Champion {}, Skin {}",
          skin.champion_id, skin.skin_id
        );
      }

      let _ = app.emit(
        "injection-status",
        format!(
          "Successfully injected {} skins ({} friend)",
          total, friend_count
        ),
      );
      Ok(())
    }
    Err(e) => {
      println!("[Party Mode] ❌ Failed to inject friend skins: {}", e);
      let _ = app.emit(
        "injection-status",
        format!("Failed to inject friend skins: {}", e),
      );
      Err(format!("Failed to inject friend skins: {}", e))
    }
  }
}

/// Trigger party-mode injection for multiple champions (instant-assign and similar modes)
pub async fn trigger_party_mode_injection_for_champions(
  app: &AppHandle,
  champion_ids: &[u32],
) -> Result<(), String> {
  if champion_ids.is_empty() {
    return Ok(());
  }

  println!(
    "[Party Mode] Triggering multi-champion injection for {:?}",
    champion_ids
  );

  let config_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("config");
  let config_file = config_dir.join("config.json");
  if !config_file.exists() {
    return Err("Config file not found".to_string());
  }

  let config_data =
    std::fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config: {}", e))?;
  let config: SavedConfig =
    serde_json::from_str(&config_data).map_err(|e| format!("Failed to parse config: {}", e))?;

  let misc_items = get_selected_misc_items(app)?;
  let league_path = config
    .league_path
    .as_ref()
    .ok_or("League path not configured".to_string())?;
  let champions_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("champions");
  if !champions_dir.exists() {
    let _ = std::fs::create_dir_all(&champions_dir);
  }
  let assets_skins_dir = PathBuf::from(league_path).join("ASSETS/Skins");

  // Keep friend identity so we don't collapse multiple friends sharing the same skin/path
  let mut skins_with_source: Vec<(Skin, Option<String>)> = Vec::new();
  let mut local_added_count = 0usize;

  // Add local skins for each champion (official preferred, fallback to custom)
  for cid in champion_ids {
    if let Some(local_skin) = config.skins.iter().find(|s| s.champion_id == *cid) {
      skins_with_source.push((
        Skin {
          champion_id: local_skin.champion_id,
          skin_id: local_skin.skin_id,
          chroma_id: local_skin.chroma_id,
          skin_file_path: local_skin.skin_file.clone(),
        },
        None,
      ));
      local_added_count += 1;
    } else if let Some(custom) = config.custom_skins.iter().find(|s| s.champion_id == *cid) {
      skins_with_source.push((
        Skin {
          champion_id: custom.champion_id,
          skin_id: 0,
          chroma_id: None,
          skin_file_path: Some(custom.file_path.clone()),
        },
        None,
      ));
      local_added_count += 1;
    }
  }

  // Add friend skins collected so far (resolve skin_file paths like in single-champion flow)
  let received_skins_map = RECEIVED_SKINS.lock().unwrap();
  for received_skin in received_skins_map.values() {
    if let Some(skin_file_path) = &received_skin.skin_file_path {
      let fp_raw = skin_file_path.clone();
      let fp_norm = fp_raw.replace('\\', "/");
      let fp = PathBuf::from(&fp_raw);
      let fp_rel = PathBuf::from(fp_norm.trim_start_matches('/'));
      let mut found_path: Option<PathBuf> = None;

      if fp.is_absolute() && fp.exists() {
        found_path = Some(fp.clone());
      }

      if found_path.is_none() {
        let fp_str = fp_norm.as_str();
        if fp_str.starts_with("/ezrea/") || fp_str.starts_with("ezrea/") {
          let tail = if fp_str.starts_with("/ezrea/") {
            &fp_str["/ezrea/".len()..]
          } else {
            &fp_str["ezrea/".len()..]
          };
          let mapped = champions_dir.join(tail);
          if mapped.exists() {
            found_path = Some(mapped);
          } else if let Some(base) = PathBuf::from(tail).file_name() {
            let by_name = champions_dir.join(base);
            if by_name.exists() {
              found_path = Some(by_name);
            }
          }
        } else if fp_str.starts_with('/') {
          let mapped = champions_dir.join(&fp_rel);
          if mapped.exists() {
            found_path = Some(mapped);
          }
        }
      }

      let try_in_dir = |dir: &PathBuf, src: &PathBuf| -> Option<PathBuf> {
        if !src.is_absolute() {
          let rel = dir.join(src);
          if rel.exists() {
            return Some(rel);
          }
        }
        if let Some(name) = src.file_name() {
          let by_name = dir.join(name);
          if by_name.exists() {
            return Some(by_name);
          }
          if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
            for ext in ["zip", "skin_file"] {
              let cand = dir.join(format!("{}.{}", stem, ext));
              if cand.exists() {
                return Some(cand);
              }
            }
          }
        }
        None
      };

      if found_path.is_none() {
        if let Some(p) = try_in_dir(&champions_dir, &fp_rel) {
          found_path = Some(p);
        }
      }
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&champions_dir, &fp) {
          found_path = Some(p);
        }
      }
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&assets_skins_dir, &fp_rel) {
          found_path = Some(p);
        }
      }
      if found_path.is_none() {
        if let Some(p) = try_in_dir(&assets_skins_dir, &fp) {
          found_path = Some(p);
        }
      }

      if found_path.is_none() {
        if let Some(local_skin) = config.skins.iter().find(|s| {
          s.champion_id == received_skin.champion_id && s.skin_id == received_skin.skin_id
        }) {
          if let Some(ref f) = local_skin.skin_file {
            let cand = PathBuf::from(f);
            let mut cands = vec![];
            if cand.is_absolute() {
              cands.push(cand.clone());
            }
            cands.push(champions_dir.join(&cand));
            if let Some(name) = cand.file_name() {
              cands.push(champions_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                cands.push(champions_dir.join(format!("{}.zip", stem)));
                cands.push(champions_dir.join(format!("{}.skin_file", stem)));
              }
            }
            cands.push(assets_skins_dir.join(&cand));
            if let Some(name) = cand.file_name() {
              cands.push(assets_skins_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                cands.push(assets_skins_dir.join(format!("{}.zip", stem)));
                cands.push(assets_skins_dir.join(format!("{}.skin_file", stem)));
              }
            }
            if let Some(found) = cands.into_iter().find(|p| p.exists()) {
              found_path = Some(found);
            }
          }
        }
      }

      // 6) Last-resort fallback by champion: use any local selection (official or custom) for this champion
      if found_path.is_none() {
        if let Some(local_any) = config
          .skins
          .iter()
          .find(|s| s.champion_id == received_skin.champion_id)
        {
          if let Some(ref f) = local_any.skin_file {
            let cand = PathBuf::from(f);
            let mut cands = vec![];
            if cand.is_absolute() {
              cands.push(cand.clone());
            }
            cands.push(champions_dir.join(&cand));
            if let Some(name) = cand.file_name() {
              cands.push(champions_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                cands.push(champions_dir.join(format!("{}.zip", stem)));
                cands.push(champions_dir.join(format!("{}.skin_file", stem)));
              }
            }
            cands.push(assets_skins_dir.join(&cand));
            if let Some(name) = cand.file_name() {
              cands.push(assets_skins_dir.join(name));
              if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
                cands.push(assets_skins_dir.join(format!("{}.zip", stem)));
                cands.push(assets_skins_dir.join(format!("{}.skin_file", stem)));
              }
            }
            if let Some(found) = cands.into_iter().find(|p| p.exists()) {
              found_path = Some(found);
            }
          }
        } else if let Some(custom_any) = config
          .custom_skins
          .iter()
          .find(|s| s.champion_id == received_skin.champion_id)
        {
          let cand = PathBuf::from(&custom_any.file_path);
          if cand.exists() {
            found_path = Some(cand);
          } else {
            let rel = champions_dir.join(&cand);
            if rel.exists() {
              found_path = Some(rel);
            }
          }
        }
      }

      if let Some(resolved) = found_path {
        skins_with_source.push((
          Skin {
            champion_id: received_skin.champion_id,
            skin_id: received_skin.skin_id,
            chroma_id: received_skin.chroma_id,
            skin_file_path: Some(resolved.to_string_lossy().to_string()),
          },
          Some(received_skin.from_summoner_id.clone()),
        ));
      } else {
        println!(
          "[Party Mode] Skipping friend skin from {} (missing skin_file: {})",
          received_skin.from_summoner_name, skin_file_path
        );
      }
    }
  }
  drop(received_skins_map);

  if skins_with_source.is_empty() && misc_items.is_empty() {
    return Ok(());
  }
  // Dedup but keep per-friend duplicates by including friend id in key
  let mut seen: HashSet<(u32, u32, Option<u32>, Option<String>, Option<String>)> = HashSet::new();
  let mut unique_skins: Vec<Skin> = Vec::new();
  for (s, friend_id_opt) in skins_with_source.into_iter() {
    let key = (
      s.champion_id,
      s.skin_id,
      s.chroma_id,
      s.skin_file_path.clone(),
      friend_id_opt.clone(),
    );
    if seen.insert(key) {
      unique_skins.push(s);
    }
  }

  match inject_skins_and_misc(app, league_path, &unique_skins, &misc_items, &champions_dir) {
    Ok(_) => {
      let total = unique_skins.len();
      let friend_count = total.saturating_sub(local_added_count);
      println!(
        "[Party Mode] ✅ Multi-champion injection complete: {} skins ({} friend, {} local)",
        total, friend_count, local_added_count
      );
      let _ = app.emit(
        "injection-status",
        format!(
          "instant-assign injected {} skins ({} friend)",
          total, friend_count
        ),
      );
      Ok(())
    }
    Err(e) => {
      eprintln!("[Party Mode] ❌ Multi-champion injection failed: {}", e);
      Err(format!("Multi-champion injection failed: {}", e))
    }
  }
}
