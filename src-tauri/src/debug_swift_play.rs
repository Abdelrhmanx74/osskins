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
            // All debug println! removed for cleaner logs
        }
    }
}