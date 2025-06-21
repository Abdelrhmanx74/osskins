// Debug utilities for matchmaking phase detection

use serde_json::Value;

/// Utility for analyzing and recording matchmaking data for debugging
pub fn analyze_matchmaking_data(json: &Value, phase: &str) {
    // Only analyze matchmaking-related data
    let state = json.get("state").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
    
    if state == "MATCHMAKING" || state == "GAMESTARTING" || state == "PREPARING" || phase == "Matchmaking" {
        // All debug println! removed for cleaner logs
    }
}
