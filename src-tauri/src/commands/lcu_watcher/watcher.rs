// Main LCU watcher using WebSocket event stream

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use base64::{engine::general_purpose, Engine};

use crate::commands::party_mode::{clear_received_skins, clear_sent_shares, PARTY_MODE_VERBOSE, RECEIVED_SKINS};
use crate::commands::types::{SavedConfig, SkinData};
use crate::commands::misc_items::get_selected_misc_items;
use crate::injection::{inject_skins_and_misc, Skin};
use super::types::{InjectionMode, LAST_CHAMPION_SHARE_TIME, LAST_PARTY_INJECTION_SIGNATURE, PARTY_INJECTION_DONE_THIS_PHASE, PHASE_STATE};
use super::utils::{compute_party_injection_signature, read_injection_mode};
use super::logging::emit_terminal_log;
use super::session::{extract_swift_play_champions_from_lobby, get_selected_champion_id, get_swift_play_champion_selections};
use super::injection::{trigger_party_mode_injection, trigger_party_mode_injection_for_champions};
use super::party_mode::check_for_party_mode_messages_with_connection;

use native_tls::TlsConnector;
use tokio_tungstenite::Connector;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};

#[tauri::command]
pub fn start_lcu_watcher(app: AppHandle, league_path: String) -> Result<(), String> {
  println!("Starting LCU watcher (WebSocket-based) for path: {}", league_path);
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
    let mut last_phase = String::from("None");
    let mut was_in_game = false;
    let mut was_reconnecting = false;
    let _ = app_handle.emit("lcu-status", "None".to_string());

    // Track last seen selections to detect changes
    let mut last_selected_skins: std::collections::HashMap<u32, SkinData> =
      std::collections::HashMap::new();
    let mut last_champion_id: Option<u32> = None;
    let mut last_party_mode_check = Instant::now();
    let mut last_party_injection_check = Instant::now();
    let mut processed_message_ids: std::collections::HashSet<String> =
      std::collections::HashSet::new();
    let mut last_party_injection_time: Instant = Instant::now() - Duration::from_secs(60);

    // Reuse a Tokio runtime for async WebSocket operations
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    loop {
      // 1) Read lockfile to get port/token
      let (port, token, lockfile_path) = match read_lockfile_once(&league_path_clone) {
        Some(t) => t,
        None => {
          let log_msg = format!(
            "[LCU Watcher] No valid lockfile found. Is League running? The lockfile should be at: {}",
            league_path_clone
          );
          println!("{}", log_msg);
          emit_terminal_log(&app_handle, &log_msg);
          thread::sleep(Duration::from_secs(3));
          continue;
        }
      };

      // 2) Build WebSocket connection to LCU
      let ws_url = format!("wss://127.0.0.1:{}/", port);
      let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));

      let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build TLS connector");

      println!("[LCU Watcher] Connecting to LCU WebSocket at {}", ws_url);
      
      // Use tokio_tungstenite's IntoClientRequest trait for proper WebSocket handshake
      use tokio_tungstenite::tungstenite::client::IntoClientRequest;
      let mut request = ws_url.clone().into_client_request().expect("Invalid URL");
      request.headers_mut().insert("Authorization", format!("Basic {}", auth).parse().unwrap());
      request.headers_mut().insert("Sec-WebSocket-Protocol", "wamp".parse().unwrap());

      let connect_res = rt.block_on(async {
        tokio_tungstenite::connect_async_tls_with_config(
          request,
          None,
          false,
          Some(Connector::NativeTls(tls))
        ).await
      });

      let (mut socket, _response) = match connect_res {
        Ok(ok) => ok,
        Err(e) => {
          eprintln!("[LCU Watcher] WebSocket connect failed: {}", e);
          thread::sleep(Duration::from_secs(2));
          continue;
        }
      };

      // Subscribe to all JSON API events
      let subscribe_msg = Message::Text("[5,\"OnJsonApiEvent\"]".into());
      let _ = rt.block_on(async { socket.send(subscribe_msg).await });
      let _ = app_handle.emit("lcu-status", "Connected");

      // Async HTTP client for occasional REST lookups
      // Keep this client handy if we later need REST in the async loop
      let _http_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("http client");

      // 3) Event loop while lockfile exists and socket is alive
      let mut socket_alive = true;
      let mut last_lobby_check = Instant::now();
      while lockfile_path.exists() && socket_alive {
        // Periodically check party-mode inbox via REST (WebSocket doesn't cover all chat events)
        if last_party_mode_check.elapsed().as_millis() >= 1500 {
          last_party_mode_check = Instant::now();
          let app_clone = app_handle.clone();
          let port_clone = port.clone();
          let token_clone = token.clone();
          let mut ids_ref = processed_message_ids.clone();
          // Run check synchronously to keep ordering
          if let Err(e) = check_for_party_mode_messages_with_connection(
            &app_clone,
            &port_clone,
            &token_clone,
            &mut ids_ref,
          ) {
            eprintln!("Error checking party mode messages: {}", e);
          } else {
            processed_message_ids = ids_ref;
          }
        }

        // Periodically poll lobby state for Swift Play (Lobby injection mode)
        // WebSocket doesn't reliably emit lobby selection events
        if injection_mode == InjectionMode::Lobby && last_phase == "Lobby" && last_lobby_check.elapsed().as_millis() >= 1000 {
          last_lobby_check = Instant::now();
          let port_clone = port.clone();
          let token_clone = token.clone();
          let auth = general_purpose::STANDARD.encode(format!("riot:{}", token_clone));
          
          // Check lobby state via REST
          let http_client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .build();
          if let Ok(client) = http_client {
            let session_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", port_clone);
            if let Ok(resp) = client
              .get(&session_url)
              .header("Authorization", format!("Basic {}", auth))
              .send()
            {
              if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                  // Check for Swift Play champion selections in lobby
                  let selections = get_swift_play_champion_selections(&json);
                  if !selections.is_empty() {
                    // Log for debugging
                    println!("[LCU Watcher][Lobby] Detected {} Swift Play champion selections", selections.len());
                  }
                }
              }
            }
          }
        }

        // Read next event
        let next_msg = rt.block_on(async { socket.next().await });
        match next_msg {
          Some(Ok(msg)) => {
            if let Some(evt) = parse_lcu_ws_event(msg) {
              match evt.uri.as_str() {
                "/lol-gameflow/v1/gameflow-phase" => {
                  let new_phase = evt
                    .data
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "None".to_string());
                  
                  // Check transitions BEFORE updating last_phase
                  let old_phase = last_phase.clone();
                  
                  // Handle Lobby->Matchmaking instant-assign via REST resolution (Swift Play/lobby injection)
                  // This fires when you click "Find Match" with champions already selected
                  if old_phase == "Lobby" && new_phase == "Matchmaking" && !crate::commands::skin_injection::is_manual_injection_active() {
                    println!("[LCU Watcher] Lobby->Matchmaking transition - triggering instant-assign injection");
                    handle_instant_assign_injection(&app_handle, &league_path_clone, &port, &token);
                  }
                  
                  // Now update the phase
                  handle_phase_change(
                    &app_handle,
                    &league_path_clone,
                    &mut last_phase,
                    &new_phase,
                    &mut was_in_game,
                    &mut was_reconnecting,
                  );
                }
                "/lol-champ-select/v1/session" => {
                  if injection_mode == InjectionMode::ChampSelect
                    && last_phase == "ChampSelect"
                    && !crate::commands::skin_injection::is_manual_injection_active()
                  {
                    handle_champ_select_event_data(
                      &app_handle,
                      &league_path_clone,
                      &evt.data,
                      &mut last_selected_skins,
                      &mut last_champion_id,
                      &mut last_party_injection_check,
                      &mut last_party_injection_time,
                    );
                  }
                }
                "/lol-lobby/v2/lobby" | "/lol-gameflow/v1/session" => {
                  // Monitor lobby for Swift Play selections (Lobby injection mode)
                  if injection_mode == InjectionMode::Lobby && last_phase == "Lobby" {
                    let selections = if evt.uri == "/lol-gameflow/v1/session" {
                      get_swift_play_champion_selections(&evt.data)
                    } else {
                      extract_swift_play_champions_from_lobby(&evt.data)
                    };
                    if !selections.is_empty() {
                      println!("[LCU Watcher][WS] Detected {} Swift Play selections in lobby", selections.len());
                    }
                  }
                }
                _ => {}
              }
            }
          }
          Some(Err(e)) => {
            eprintln!("[LCU Watcher] WebSocket read error: {}", e);
            socket_alive = false;
          }
          None => {
            eprintln!("[LCU Watcher] WebSocket stream ended");
            socket_alive = false;
          }
        }
      }

      // Socket done or lockfile gone => loop will retry
      let _ = app_handle.emit("lcu-status", "Disconnected");

      // Only run polling fallback if lockfile still exists (WS failed but LCU is running)
      // Don't run it if LCU closed (lockfile gone) - just wait and retry WS connection
      if lockfile_path.exists() {
        println!("[LCU Watcher] WebSocket disconnected but lockfile exists, trying polling fallback");
        run_polling_loop(
          &app_handle,
          &league_path_clone,
          &port,
          &token,
          &mut last_phase,
          &mut was_in_game,
          &mut was_reconnecting,
          &mut last_selected_skins,
          &mut last_champion_id,
          &mut last_party_mode_check,
          &mut processed_message_ids,
          &mut last_party_injection_check,
          &mut last_party_injection_time,
        );
      }

      thread::sleep(Duration::from_secs(2));
    }
  });

  println!("LCU status watcher thread started");
  Ok(())
}

fn read_lockfile_once(league_path: &str) -> Option<(String, String, PathBuf)> {
  let dir = PathBuf::from(league_path);
  for name in ["lockfile", "LeagueClientUx.lockfile", "LeagueClient.lockfile"] {
    let path = dir.join(name);
    if let Ok(content) = fs::read_to_string(&path) {
      let parts: Vec<&str> = content.split(':').collect();
      if parts.len() >= 5 {
        let port = parts[2].to_string();
        let token = parts[3].to_string();
        return Some((port, token, path));
      }
    }
  }
  None
}

#[derive(Debug, Clone)]
struct LcuEvent {
  uri: String,
  data: serde_json::Value,
}

fn parse_lcu_ws_event(msg: Message) -> Option<LcuEvent> {
  if !msg.is_text() {
    return None;
  }
  let txt = msg.into_text().ok()?;
  // Expect an array like [8, "OnJsonApiEvent", { uri, eventType, data }]
  let val: serde_json::Value = serde_json::from_str(&txt).ok()?;
  if let serde_json::Value::Array(arr) = val {
    if arr.len() >= 3 {
      if let Some(obj) = arr[2].as_object() {
        let uri = obj
          .get("uri")
          .and_then(|v| v.as_str())
          .unwrap_or("")
          .to_string();
        let data = obj.get("data").cloned().unwrap_or(serde_json::Value::Null);
        return Some(LcuEvent { uri, data });
      }
    }
  }
  None
}

fn handle_phase_change(
  app_handle: &AppHandle,
  league_path: &str,
  last_phase: &mut String,
  new_phase: &str,
  was_in_game: &mut bool,
  was_reconnecting: &mut bool,
) {
  if new_phase == *last_phase {
    return;
  }
  println!("[LCU Watcher] LCU status changed: {} -> {}", last_phase, new_phase);
  emit_terminal_log(
    app_handle,
    &format!("[LCU Watcher] LCU status changed: {} -> {}", last_phase, new_phase),
  );

  let should_cleanup = match (&**last_phase, new_phase) {
    ("InProgress", "None") => true,
    ("InProgress", "Lobby") => true,
    ("InProgress", "Matchmaking") => true,
    // Don't cleanup on Matchmaking->Lobby (Swift Play flow when match found)
    ("Matchmaking", "Lobby") => false,
    ("Reconnect", "None") => true,
    ("Reconnect", "Lobby") => true,
    ("Reconnect", "Matchmaking") => true,
    ("ChampSelect", "None") => true,
    ("ChampSelect", "Lobby") => true,
    ("ChampSelect", "Matchmaking") => true,
    (_, "None") if last_phase != "None" => true,
    ("ChampSelect", "InProgress") => false,
    ("ChampSelect", "Reconnect") => false,
    ("InProgress", "Reconnect") => false,
    ("Reconnect", "InProgress") => false,
    _ => false,
  };

  if should_cleanup {
    PARTY_INJECTION_DONE_THIS_PHASE.store(false, Ordering::Relaxed);
    let _ = app_handle.emit("injection-status", "idle");
    match crate::injection::needs_injection_cleanup(app_handle, league_path) {
      Ok(needs_cleanup) => {
        if needs_cleanup {
          let log_msg = format!(
            "[LCU Watcher] Injection cleanup needed for phase transition {} -> {}, cleaning up...",
            last_phase, new_phase
          );
          println!("{}", log_msg);
          emit_terminal_log(app_handle, &log_msg);
          if let Err(e) = crate::injection::cleanup_injection(app_handle, league_path) {
            let error_msg = format!(
              "[LCU Watcher] Error cleaning up injection on phase change: {}",
              e
            );
            println!("{}", error_msg);
            emit_terminal_log(app_handle, &error_msg);
          } else {
            let success_msg = "[LCU Watcher] ✅ Injection cleanup completed successfully";
            println!("{}", success_msg);
            emit_terminal_log(app_handle, success_msg);
          }
        }
      }
      Err(e) => {
        let error_msg = format!("[LCU Watcher] Error checking if cleanup is needed: {}", e);
        println!("{}", error_msg);
        emit_terminal_log(app_handle, &error_msg);
      }
    }
  } else {
    let log_msg = format!(
      "[LCU Watcher] Phase transition {} -> {} does not require cleanup, keeping injection active",
      last_phase, new_phase
    );
    println!("{}", log_msg);
    emit_terminal_log(app_handle, &log_msg);
  }

  if new_phase == "ChampSelect" {
    if let Ok(mut g) = LAST_PARTY_INJECTION_SIGNATURE.lock() {
      *g = None;
    }
    clear_received_skins();
    clear_sent_shares();
    if let Ok(mut times) = LAST_CHAMPION_SHARE_TIME.lock() {
      times.clear();
    }
    PARTY_INJECTION_DONE_THIS_PHASE.store(false, Ordering::Relaxed);
    println!("[LCU Watcher][DEBUG] Reset party-mode state for new ChampSelect");

    let champions_dir = app_handle
      .path()
      .app_data_dir()
      .unwrap_or_else(|_| PathBuf::from("."))
      .join("champions");
    if !champions_dir.exists() {
      let _ = fs::create_dir_all(&champions_dir);
    }
    let overlay_dir = app_handle
      .path()
      .app_data_dir()
      .unwrap_or_else(|_| PathBuf::from("."))
      .join("overlay");
    if overlay_dir.exists() {
      let _ = fs::remove_dir_all(&overlay_dir);
    }

    if crate::commands::skin_injection::is_manual_injection_active() {
      println!("[LCU Watcher] Manual injection mode active - triggering injection");
      let app_clone = app_handle.clone();
      std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async move {
          let _ = crate::commands::skin_injection::trigger_manual_injection(&app_clone).await;
        });
      });
    }
  }

  // Manual mode: Lobby -> Matchmaking transition injection
  if *last_phase == "Lobby"
    && new_phase == "Matchmaking"
    && crate::commands::skin_injection::is_manual_injection_active()
  {
    emit_terminal_log(
      app_handle,
      "[LCU Watcher] Lobby->Matchmaking detected; manual injection active - triggering",
    );
    let app_clone = app_handle.clone();
    std::thread::spawn(move || {
      let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
      rt.block_on(async move {
        let _ = crate::commands::skin_injection::trigger_manual_injection(&app_clone).await;
      });
    });
  }

  *last_phase = new_phase.to_string();
  *was_reconnecting = new_phase == "Reconnect";
  *was_in_game = new_phase == "InProgress" || *was_reconnecting;
  if new_phase == "ChampSelect" {
    PHASE_STATE.store(1, Ordering::Relaxed);
  } else {
    PHASE_STATE.store(2, Ordering::Relaxed);
  }
}

fn handle_champ_select_event_data(
  app_handle: &AppHandle,
  league_path: &str,
  data: &serde_json::Value,
  last_selected_skins: &mut std::collections::HashMap<u32, SkinData>,
  last_champion_id: &mut Option<u32>,
  last_party_injection_check: &mut Instant,
  last_party_injection_time: &mut Instant,
) {
  if let Some(selected_champ_id) = get_selected_champion_id(data) {
    let current_champion_id = selected_champ_id as u32;
    let champion_changed = if let Some(last_champ) = *last_champion_id {
      last_champ != current_champion_id
    } else {
      true
    };
    *last_champion_id = Some(current_champion_id);

    if champion_changed {
      let config_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
      let cfg_file = config_dir.join("config.json");
      if let Ok(data) = std::fs::read_to_string(&cfg_file) {
        if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
          if let Some(skin) = config
            .skins
            .iter()
            .find(|s| s.champion_id == current_champion_id)
          {
            let should_share = {
              let mut last_shares = LAST_CHAMPION_SHARE_TIME.lock().unwrap();
              let mut can_share = true;
              let now = Instant::now();
              for (_champ_id, last_time) in last_shares.iter() {
                if last_time.elapsed().as_millis() < 1000 {
                  can_share = false;
                  break;
                }
              }
              if can_share {
                last_shares.insert(current_champion_id, now);
                true
              } else {
                false
              }
            };
            if should_share {
              let app_handle_clone = app_handle.clone();
              let skin_clone = skin.clone();
              std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                  let _ = crate::commands::party_mode::send_skin_share_to_paired_friends(
                    &app_handle_clone,
                    skin_clone.champion_id,
                    skin_clone.skin_id,
                    skin_clone.chroma_id,
                    skin_clone.skin_file.clone(),
                  )
                  .await;
                });
              });
            }
            last_selected_skins.insert(current_champion_id, skin.clone());
          }
        }
      }
    }

    // Party mode trigger check
    if last_party_injection_check.elapsed().as_millis() >= 1000
      && !crate::commands::skin_injection::is_manual_injection_active()
    {
      *last_party_injection_check = Instant::now();
      let should_inject = {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
          crate::commands::party_mode::should_inject_now(app_handle, current_champion_id)
            .await
            .unwrap_or(false)
        })
      };
      let already_done = PARTY_INJECTION_DONE_THIS_PHASE.load(Ordering::Relaxed);
      if should_inject && !already_done && last_party_injection_time.elapsed().as_secs() >= 5 {
        let current_sig = compute_party_injection_signature(current_champion_id);
        let mut guard = LAST_PARTY_INJECTION_SIGNATURE.lock().unwrap();
        if guard.as_ref() != Some(&current_sig) {
          PARTY_INJECTION_DONE_THIS_PHASE.store(true, Ordering::Relaxed);
          *guard = Some(current_sig);
          drop(guard);
          let app_handle_clone = app_handle.clone();
          std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
              let _ = trigger_party_mode_injection(&app_handle_clone, current_champion_id).await;
            });
          });
          *last_party_injection_time = Instant::now();
        }
      }
    }

    // Auto injection on skin change
    if !crate::commands::skin_injection::is_manual_injection_active() {
      let config_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config");
      let cfg_file = config_dir.join("config.json");
      if let Ok(data) = std::fs::read_to_string(&cfg_file) {
        if let Ok(config) = serde_json::from_str::<SavedConfig>(&data) {
          for skin in &config.skins {
            let champ_id = skin.champion_id;
            if *last_champion_id == Some(champ_id) {
              let skin_has_changed = !last_selected_skins.contains_key(&champ_id)
                || last_selected_skins.get(&champ_id).map_or(true, |old_skin| {
                  old_skin.skin_id != skin.skin_id
                    || old_skin.chroma_id != skin.chroma_id
                    || old_skin.skin_file != skin.skin_file
                });
              if skin_has_changed {
                println!(
                  "[Auto Injection] Skin change detected for champion {}, triggering re-injection",
                  champ_id
                );
                let mut skins_to_inject = vec![Skin {
                  champion_id: skin.champion_id,
                  skin_id: skin.skin_id,
                  chroma_id: skin.chroma_id,
                  skin_file_path: skin.skin_file.clone(),
                }];
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
                let assets_skins_dir = PathBuf::from(league_path).join("ASSETS/Skins");
                let original_len = skins_to_inject.len();
                let filtered_skins: Vec<Skin> = skins_to_inject
                  .into_iter()
                  .filter(|s| {
                    if let Some(ref fp_str) = s.skin_file_path {
                      let fp = PathBuf::from(fp_str);
                      let absolute_exists = fp.is_absolute() && fp.exists();
                      let exists_in_champions_rel = if fp.is_absolute() {
                        false
                      } else {
                        champions_dir.join(&fp).exists()
                      };
                      let exists_in_champions_name = fp
                        .file_name()
                        .map(|n| champions_dir.join(n).exists())
                        .unwrap_or(false);
                      let exists_in_assets_rel = if fp.is_absolute() {
                        false
                      } else {
                        assets_skins_dir.join(&fp).exists()
                      };
                      let exists_in_assets_name = fp
                        .file_name()
                        .map(|n| assets_skins_dir.join(n).exists())
                        .unwrap_or(false);
                      absolute_exists
                        || exists_in_champions_rel
                        || exists_in_champions_name
                        || exists_in_assets_rel
                        || exists_in_assets_name
                    } else {
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
                let misc_items = get_selected_misc_items(app_handle).unwrap_or_default();
                match inject_skins_and_misc(
                  app_handle,
                  league_path,
                  &filtered_skins,
                  &misc_items,
                  &champions_dir,
                ) {
                  Ok(_) => {
                    let _ = app_handle.emit("injection-status", "success");
                    println!(
                      "[Enhanced] Successfully injected {} skins and {} misc items for champion {}",
                      filtered_skins.len(),
                      misc_items.len(),
                      champ_id
                    );
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
              }
            }
          }
        }
      }
    }
  }
}

fn handle_instant_assign_injection(
  app_handle: &AppHandle,
  league_path: &str,
  port: &str,
  token: &str,
) {
  let already_done = PARTY_INJECTION_DONE_THIS_PHASE.load(Ordering::Relaxed);
  if already_done {
    println!("[LCU Watcher][instant-assign] Injection already completed for this phase; skipping");
    return;
  }
  PARTY_INJECTION_DONE_THIS_PHASE.store(true, Ordering::Relaxed);
  clear_sent_shares();
  emit_terminal_log(app_handle, "[LCU Watcher] Lobby->Matchmaking detected; resolving lobby-selected champions...");

  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));
  let mut resolved_champions: Vec<i64> = Vec::new();

  // Build blocking client for synchronous resolution
  let http_client = reqwest::blocking::Client::builder()
    .danger_accept_invalid_certs(true)
    .build();
  let http_client = match http_client { Ok(c) => c, Err(_) => return };

  // Try gameflow session
  let session_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/session", port);
  if let Ok(resp) = http_client
    .get(&session_url)
    .header("Authorization", format!("Basic {}", auth))
    .send()
  {
    if resp.status().is_success() {
      if let Ok(json) = resp.json::<serde_json::Value>() {
      resolved_champions.extend(get_swift_play_champion_selections(&json));
      if let Some(game_data) = json.get("gameData") {
        if let Some(selected) = game_data.get("selectedChampions").and_then(|s| s.as_array()) {
          for sel in selected {
            if let Some(cid) = sel.get("championId").and_then(|v| v.as_i64()) {
              if cid > 0 && !resolved_champions.contains(&cid) {
                resolved_champions.push(cid);
              }
            }
          }
        }
        if let Some(pcs) = game_data
          .get("playerChampionSelections")
          .and_then(|p| p.as_array())
        {
          for item in pcs {
            if let Some(champs) = item.get("championIds").and_then(|c| c.as_array()) {
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

  // Try lobby v2
  if resolved_champions.is_empty() {
    let lobby_url_v2 = format!("https://127.0.0.1:{}/lol-lobby/v2/lobby", port);
    if let Ok(resp) = http_client
      .get(&lobby_url_v2)
      .header("Authorization", format!("Basic {}", auth))
      .send()
    {
      if resp.status().is_success() {
        if let Ok(json) = resp.json::<serde_json::Value>() {
        let lobby_ids = extract_swift_play_champions_from_lobby(&json);
        for id in lobby_ids {
          if !resolved_champions.contains(&id) {
            resolved_champions.push(id);
          }
        }
        }
      }
    }
  }

  // Try lobby v1
  if resolved_champions.is_empty() {
    let lobby_url_v1 = format!("https://127.0.0.1:{}/lol-lobby/v1/lobby", port);
    if let Ok(resp) = http_client
      .get(&lobby_url_v1)
      .header("Authorization", format!("Basic {}", auth))
      .send()
    {
      if resp.status().is_success() {
        if let Ok(json) = resp.json::<serde_json::Value>() {
        let ids = extract_swift_play_champions_from_lobby(&json);
        for id in ids {
          if !resolved_champions.contains(&id) {
            resolved_champions.push(id);
          }
        }
        }
      }
    }
  }

  if !resolved_champions.is_empty() {
    emit_terminal_log(
      app_handle,
      &format!(
        "[LCU Watcher] Resolved {} champion(s) from lobby/session: {:?}",
        resolved_champions.len(),
        resolved_champions
      ),
    );
    // Send shares then inject (party mode)
    let app_for_async = app_handle.clone();
    let champs_u32: Vec<u32> = resolved_champions.iter().map(|c| *c as u32).collect();
    std::thread::spawn(move || {
      let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
      rt.block_on(async move {
        let config_dir = app_for_async
          .path()
          .app_data_dir()
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
                )
                .await
                .map(|_| {
                  sent_share = true;
                });
              } else if let Some(custom) =
                config.custom_skins.iter().find(|s| s.champion_id == *cid)
              {
                let _ = crate::commands::party_mode::send_skin_share_to_paired_friends(
                  &app_for_async,
                  custom.champion_id,
                  0,
                  None,
                  Some(custom.file_path.clone()),
                )
                .await
                .map(|_| {
                  sent_share = true;
                });
              }
              if sent_share {
                println!(
                  "[Party Mode][instant-assign] Sent skin_share for champion {} on Matchmaking",
                  cid
                );
              }
            }
          }
        }

        // Wait briefly
        let mut ready = false;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(6) {
          match crate::commands::party_mode::should_inject_now(&app_for_async, *champs_u32
            .get(0)
            .unwrap_or(&0))
          .await
          {
            Ok(true) => {
              ready = true;
              break;
            }
            Ok(false) => {
              println!(
                "[Party Mode][instant-assign] Waiting for friends to share before injection..."
              );
            }
            Err(e) => {
              println!(
                "[Party Mode][instant-assign] should_inject_now error (proceeding): {}",
                e
              );
              break;
            }
          }
          std::thread::sleep(Duration::from_millis(500));
        }
        if !ready {
          println!(
            "[Party Mode][instant-assign] Proceeding after {}s wait (friends may share shortly)",
            start.elapsed().as_secs_f32()
          );
        }
        if let Err(e) = trigger_party_mode_injection_for_champions(&app_for_async, &champs_u32)
          .await
        {
          eprintln!("[Party Mode][instant-assign] Injection failed: {}", e);
        }
      });
    });
  } else {
    emit_terminal_log(
      app_handle,
      "[LCU Watcher] Could not resolve any champion selections from lobby/session",
    );
  }
}

fn run_polling_loop(
  app_handle: &AppHandle,
  league_path: &str,
  port: &str,
  token: &str,
  last_phase: &mut String,
  was_in_game: &mut bool,
  was_reconnecting: &mut bool,
  last_selected_skins: &mut std::collections::HashMap<u32, SkinData>,
  last_champion_id: &mut Option<u32>,
  last_party_mode_check: &mut Instant,
  processed_message_ids: &mut std::collections::HashSet<String>,
  last_party_injection_check: &mut Instant,
  last_party_injection_time: &mut Instant,
) {
  let auth = general_purpose::STANDARD.encode(format!("riot:{}", token));
  let client = match reqwest::blocking::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()
  {
    Ok(c) => c,
    Err(e) => {
      eprintln!("[LCU Watcher][poll] HTTP client build failed: {}", e);
      return;
    }
  };

  let mut last_phase_seen = last_phase.clone();

  // Poll for a bounded time or until things change
  for _ in 0..1200 { // ~20 minutes max as a safety guard
    // Party-mode inbox polling
    if last_party_mode_check.elapsed().as_millis() >= 1500 {
      *last_party_mode_check = Instant::now();
      if let Err(e) = check_for_party_mode_messages_with_connection(
        app_handle,
        port,
        token,
        processed_message_ids,
      ) {
        eprintln!("[LCU Watcher][poll] party-mode check error: {}", e);
      }
    }

    // Phase polling
    let phase_url = format!("https://127.0.0.1:{}/lol-gameflow/v1/gameflow-phase", port);
    let phase = match client
      .get(&phase_url)
      .header("Authorization", format!("Basic {}", auth))
      .send()
    {
      Ok(resp) if resp.status().is_success() => resp
        .text()
        .unwrap_or_else(|_| "None".into())
        .trim_matches('"')
        .to_string(),
      _ => {
        std::thread::sleep(Duration::from_millis(500));
        continue;
      }
    };

    if &phase != &last_phase_seen {
      handle_phase_change(
        app_handle,
        league_path,
        last_phase,
        &phase,
        was_in_game,
        was_reconnecting,
      );
      last_phase_seen = phase.clone();

      // Manual mode Lobby->Matchmaking handling (parity with WS path)
      if *last_phase == "Lobby"
        && phase == "Matchmaking"
        && crate::commands::skin_injection::is_manual_injection_active()
      {
        let app_clone = app_handle.clone();
        std::thread::spawn(move || {
          let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
          rt.block_on(async move {
            let _ = crate::commands::skin_injection::trigger_manual_injection(&app_clone).await;
          });
        });
      }

      // Auto/party instant-assign on Lobby->Matchmaking
      if last_phase == "Lobby" && phase == "Matchmaking" && !crate::commands::skin_injection::is_manual_injection_active() {
        handle_instant_assign_injection(app_handle, league_path, port, token);
      }
    }

    // Champ Select polling only when needed
    if phase == "ChampSelect" && !crate::commands::skin_injection::is_manual_injection_active() {
      let session_url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
      if let Ok(resp) = client
        .get(&session_url)
        .header("Authorization", format!("Basic {}", auth))
        .send()
      {
        if resp.status().is_success() {
          if let Ok(json) = resp.json::<serde_json::Value>() {
            handle_champ_select_event_data(
              app_handle,
              league_path,
              &json,
              last_selected_skins,
              last_champion_id,
              last_party_injection_check,
              last_party_injection_time,
            );
          }
        }
      }
    }

    std::thread::sleep(Duration::from_millis(500));
  }
}
