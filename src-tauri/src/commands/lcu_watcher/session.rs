// Session and champion selection utilities

use serde_json;

// Helper function to get selected champion ID from session JSON
pub fn get_selected_champion_id(session_json: &serde_json::Value) -> Option<i64> {
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

// Helper function to get instant-assign champion selections from session JSON
pub fn get_swift_play_champion_selections(json: &serde_json::Value) -> Vec<i64> {
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

// Extract instant-assign champion IDs from the lobby data directly
pub fn extract_swift_play_champions_from_lobby(json: &serde_json::Value) -> Vec<i64> {
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
