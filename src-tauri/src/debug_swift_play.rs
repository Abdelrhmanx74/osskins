// Debug utilities for Swift Play specific issues

use serde_json::Value;

/// Utility for analyzing and recording Swift Play specific JSON data for debugging
pub fn analyze_swift_play_data(json: &Value, phase: &str) {
    // Only log relevant Swift Play data
    if let Some(game_data) = json.get("gameData") {
        let queue_id = game_data.get("queue")
            .and_then(|q| q.get("id"))
            .and_then(|id| id.as_i64())
            .unwrap_or(0);
            
        // Only log for Swift Play queue IDs
        if queue_id == 480 || queue_id == 1700 {
            println!("[SWIFT PLAY DEBUG] =======================");
            println!("[SWIFT PLAY DEBUG] Queue ID: {}, Phase: {}", queue_id, phase);
            
            // Extract and log the state
            let state = json.get("state").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
            println!("[SWIFT PLAY DEBUG] State: {}", state);
            
            // Check if champions are in playerSlots
            if let Some(local_member) = json.get("localMember") {
                if let Some(player_slots) = local_member.get("playerSlots").and_then(|ps| ps.as_array()) {
                    println!("[SWIFT PLAY DEBUG] Found {} player slots", player_slots.len());
                    
                    for (i, slot) in player_slots.iter().enumerate() {
                        let champion_id = slot.get("championId")
                            .and_then(|id| id.as_i64())
                            .unwrap_or(0);
                            
                        let status = slot.get("selectionStatus")
                            .and_then(|s| s.as_str())
                            .unwrap_or("UNKNOWN");
                            
                        println!("[SWIFT PLAY DEBUG] Slot {}: Champion ID={}, Status={}", i, champion_id, status);
                    }
                } else {
                    println!("[SWIFT PLAY DEBUG] No player slots found in localMember");
                }
                
                // Check if the ready flag is set
                let is_ready = local_member.get("ready")
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);
                
                println!("[SWIFT PLAY DEBUG] Ready flag: {}", is_ready);
            } else {
                println!("[SWIFT PLAY DEBUG] No localMember found");
            }
            
            // Check if playerChampionSelections exists
            if let Some(selections) = game_data.get("playerChampionSelections").and_then(|s| s.as_array()) {
                println!("[SWIFT PLAY DEBUG] Found playerChampionSelections: {} entries", selections.len());
                
                for (i, selection) in selections.iter().enumerate() {
                    if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                        println!("[SWIFT PLAY DEBUG] Selection {}: Champion ID={}", i, champion_id);
                    }
                }
            }
            
            println!("[SWIFT PLAY DEBUG] =======================");
        }
    }
}