// Debug utilities for matchmaking phase detection

use serde_json::Value;

/// Utility for analyzing and recording matchmaking data for debugging
pub fn analyze_matchmaking_data(json: &Value, phase: &str) {
    // Only analyze matchmaking-related data
    let state = json.get("state").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
    
    if state == "MATCHMAKING" || state == "GAMESTARTING" || state == "PREPARING" || phase == "Matchmaking" {
        println!("[MATCHMAKING DEBUG] =======================");
        println!("[MATCHMAKING DEBUG] State: {}, Phase: {}", state, phase);
        
        // Get queue ID for additional context
        let queue_id = json.get("gameData")
            .and_then(|d| d.get("queue"))
            .and_then(|q| q.get("id"))
            .and_then(|id| id.as_i64())
            .unwrap_or_else(|| {
                json.get("gameConfig")
                    .and_then(|c| c.get("queueId"))
                    .and_then(|id| id.as_i64())
                    .unwrap_or(0)
            });
            
        println!("[MATCHMAKING DEBUG] Queue ID: {}", queue_id);
        
        // Check for available champion data
        println!("[MATCHMAKING DEBUG] Looking for champion data...");
        
        // Check localMember and playerSlots
        if let Some(local_member) = json.get("localMember") {
            println!("[MATCHMAKING DEBUG] Found localMember");
            
            // Check ready status
            let ready = local_member.get("ready")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);
                
            println!("[MATCHMAKING DEBUG] Ready status: {}", ready);
            
            // Check player slots
            if let Some(slots) = local_member.get("playerSlots").and_then(|s| s.as_array()) {
                println!("[MATCHMAKING DEBUG] Found {} player slots", slots.len());
                
                for (i, slot) in slots.iter().enumerate() {
                    let champion_id = slot.get("championId")
                        .and_then(|id| id.as_i64())
                        .unwrap_or(0);
                        
                    let selection_status = slot.get("selectionStatus")
                        .and_then(|s| s.as_str())
                        .unwrap_or("UNKNOWN");
                        
                    println!("[MATCHMAKING DEBUG] Slot {}: Champion ID={}, Status={}", 
                             i, champion_id, selection_status);
                }
            } else {
                println!("[MATCHMAKING DEBUG] No playerSlots found");
            }
        } else {
            println!("[MATCHMAKING DEBUG] No localMember found");
        }
        
        // Check for gameData.playerChampionSelections
        if let Some(game_data) = json.get("gameData") {
            if let Some(selections) = game_data.get("playerChampionSelections").and_then(|s| s.as_array()) {
                println!("[MATCHMAKING DEBUG] Found {} playerChampionSelections", selections.len());
                
                for (i, selection) in selections.iter().enumerate() {
                    if let Some(champion_id) = selection.get("championId").and_then(|id| id.as_i64()) {
                        println!("[MATCHMAKING DEBUG] Selection {}: Champion ID={}", i, champion_id);
                    }
                }
            } else {
                println!("[MATCHMAKING DEBUG] No playerChampionSelections found");
            }
            
            // Check additional game data
            if let Some(game_mode) = game_data.get("queue")
                .and_then(|q| q.get("gameMode"))
                .and_then(|m| m.as_str()) 
            {
                println!("[MATCHMAKING DEBUG] Game mode: {}", game_mode);
            }
        }
        
        println!("[MATCHMAKING DEBUG] =======================");
    }
}
