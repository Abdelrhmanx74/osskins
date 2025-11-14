// Injection logic for skins and misc items

use std::collections::HashSet;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Emitter};
use serde_json;

use crate::commands::misc_items::get_selected_misc_items;
use crate::commands::types::SavedConfig;
use crate::commands::party_mode::RECEIVED_SKINS;
use crate::injection::Skin;
use super::utils::is_in_champ_select;

// Helper function to inject skins for multiple champions (used in instant-assign)
// Kept for backward compatibility and manual calls (not referenced by watcher now).
#[allow(dead_code)]
pub fn inject_skins_for_champions(app: &AppHandle, league_path: &str, champion_ids: &[i64]) {
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

        // Emit start event once for the whole batch
        let _ = app.emit("injection-status", "injecting");

        match crate::injection::inject_skins_and_misc_no_events(
          app,
          league_path,
          &skins_to_inject,
          &misc_items,
          &champions_dir,
        ) {
          Ok(_) => {
            // Emit success once for the whole batch
            let _ = app.emit("injection-status", "completed");
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
  // Emit start event once for the whole batch
  let _ = app.emit("injection-status", "injecting");

  match crate::injection::inject_skins_and_misc_no_events(
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

      // Emit success once for the whole batch
      let _ = app.emit("injection-status", "completed");
      Ok(())
    }
    Err(e) => {
      println!("[Party Mode] ❌ Failed to inject friend skins: {}", e);
      let _ = app.emit("injection-status", "error");
      Err(format!("Failed to inject friend skins: {}", e))
    }
  }
}

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

  // Emit start event once for the whole batch
  let _ = app.emit("injection-status", "injecting");

  match crate::injection::inject_skins_and_misc_no_events(app, league_path, &unique_skins, &misc_items, &champions_dir) {
    Ok(_) => {
      let total = unique_skins.len();
      let friend_count = total.saturating_sub(local_added_count);
      println!(
        "[Party Mode] ✅ Multi-champion injection complete: {} skins ({} friend, {} local)",
        total, friend_count, local_added_count
      );
      // Emit success once for the whole batch
      let _ = app.emit("injection-status", "completed");
      Ok(())
    }
    Err(e) => {
      eprintln!("[Party Mode] ❌ Multi-champion injection failed: {}", e);
      let _ = app.emit("injection-status", "error");
      Err(format!("Multi-champion injection failed: {}", e))
    }
  }
}
