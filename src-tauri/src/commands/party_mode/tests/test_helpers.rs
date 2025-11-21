// Test helpers and mock utilities for party mode tests

use std::collections::{HashMap, HashSet};
use super::super::types::{InMemoryReceivedSkin, RECEIVED_SKINS, SENT_SKIN_SHARES};

// Mock types that mirror the real types
pub struct SkinShare {
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub champion_id: u32,
    pub skin_id: u32,
    pub skin_name: String,
    pub chroma_id: Option<u32>,
    pub skin_file_path: Option<String>,
    pub timestamp: u64,
}

pub struct PairedFriend {
    pub summoner_id: String,
    pub summoner_name: String,
    pub display_name: String,
    pub share_enabled: bool,
}

/// Mock LCU connection data
pub struct MockLcuConnection {
    pub port: String,
    pub token: String,
}

impl Default for MockLcuConnection {
    fn default() -> Self {
        Self {
            port: "8080".to_string(),
            token: "test_token".to_string(),
        }
    }
}

/// Create a mock skin share message
pub fn create_mock_skin_share(
    from_summoner_id: &str,
    from_summoner_name: &str,
    champion_id: u32,
    skin_id: u32,
    chroma_id: Option<u32>,
    skin_file_path: Option<String>,
    timestamp: u64,
) -> SkinShare {
    SkinShare {
        from_summoner_id: from_summoner_id.to_string(),
        from_summoner_name: from_summoner_name.to_string(),
        champion_id,
        skin_id,
        skin_name: format!("Test Skin {}", skin_id),
        chroma_id,
        skin_file_path,
        timestamp,
    }
}

/// Create a mock paired friend
pub fn create_mock_paired_friend(
    summoner_id: &str,
    summoner_name: &str,
    display_name: &str,
    share_enabled: bool,
) -> PairedFriend {
    PairedFriend {
        summoner_id: summoner_id.to_string(),
        summoner_name: summoner_name.to_string(),
        display_name: display_name.to_string(),
        share_enabled,
    }
}

/// Add a received skin to the global cache
pub fn add_received_skin_to_cache(
    from_summoner_id: &str,
    from_summoner_name: &str,
    champion_id: u32,
    skin_id: u32,
    chroma_id: Option<u32>,
    skin_file_path: Option<String>,
    received_at: u64,
) {
    let key = format!("{}_{}", from_summoner_id, champion_id);
    let mut map = RECEIVED_SKINS.lock().unwrap();
    map.insert(
        key,
        InMemoryReceivedSkin {
            from_summoner_id: from_summoner_id.to_string(),
            from_summoner_name: from_summoner_name.to_string(),
            champion_id,
            skin_id,
            chroma_id,
            skin_file_path,
            received_at,
        },
    );
}

/// Clear all received skins from cache
pub fn clear_received_skins_cache() {
    let mut map = RECEIVED_SKINS.lock().unwrap();
    map.clear();
}

/// Get count of received skins in cache
pub fn get_received_skins_count() -> usize {
    let map = RECEIVED_SKINS.lock().unwrap();
    map.len()
}

/// Check if a specific skin is in cache
pub fn is_skin_in_cache(from_summoner_id: &str, champion_id: u32) -> bool {
    let key = format!("{}_{}", from_summoner_id, champion_id);
    let map = RECEIVED_SKINS.lock().unwrap();
    map.contains_key(&key)
}

/// Add a sent share signature to deduplication set
pub fn add_sent_share_signature(
    friend_summoner_id: &str,
    champion_id: u32,
    skin_id: u32,
    chroma_id: Option<u32>,
) {
    let chroma_str = chroma_id.map(|c| c.to_string()).unwrap_or_else(|| "none".to_string());
    let key = format!("{}:{}:{}:{}", friend_summoner_id, champion_id, skin_id, chroma_str);
    let mut sent = SENT_SKIN_SHARES.lock().unwrap();
    sent.insert(key);
}

/// Clear sent shares deduplication set
pub fn clear_sent_shares() {
    let mut sent = SENT_SKIN_SHARES.lock().unwrap();
    sent.clear();
}

/// Check if a share was already sent
pub fn was_share_sent(
    friend_summoner_id: &str,
    champion_id: u32,
    skin_id: u32,
    chroma_id: Option<u32>,
) -> bool {
    let chroma_str = chroma_id.map(|c| c.to_string()).unwrap_or_else(|| "none".to_string());
    let key = format!("{}:{}:{}:{}", friend_summoner_id, champion_id, skin_id, chroma_str);
    let sent = SENT_SKIN_SHARES.lock().unwrap();
    sent.contains(&key)
}

/// Get current timestamp in milliseconds
pub fn get_current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Get a timestamp from N seconds ago
pub fn get_timestamp_seconds_ago(seconds: u64) -> u64 {
    let current = get_current_timestamp_ms();
    current.saturating_sub(seconds * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_skin_share_creation() {
        let share = create_mock_skin_share(
            "123",
            "TestPlayer",
            266,
            1,
            Some(2),
            Some("/path/to/skin.zip".to_string()),
            1234567890,
        );
        
        assert_eq!(share.from_summoner_id, "123");
        assert_eq!(share.from_summoner_name, "TestPlayer");
        assert_eq!(share.champion_id, 266);
        assert_eq!(share.skin_id, 1);
        assert_eq!(share.chroma_id, Some(2));
        assert_eq!(share.skin_file_path, Some("/path/to/skin.zip".to_string()));
    }

    #[test]
    fn test_mock_paired_friend_creation() {
        let friend = create_mock_paired_friend(
            "456",
            "FriendName",
            "Friend Display",
            true,
        );
        
        assert_eq!(friend.summoner_id, "456");
        assert_eq!(friend.summoner_name, "FriendName");
        assert_eq!(friend.display_name, "Friend Display");
        assert!(friend.share_enabled);
    }

    #[test]
    fn test_received_skins_cache_operations() {
        clear_received_skins_cache();
        
        assert_eq!(get_received_skins_count(), 0);
        
        add_received_skin_to_cache(
            "999",
            "TestFriend",
            266,
            1,
            None,
            Some("/test/skin.zip".to_string()),
            1234567890,
        );
        
        assert_eq!(get_received_skins_count(), 1);
        assert!(is_skin_in_cache("999", 266));
        assert!(!is_skin_in_cache("999", 267));
        
        clear_received_skins_cache();
        assert_eq!(get_received_skins_count(), 0);
    }

    #[test]
    fn test_sent_shares_deduplication() {
        clear_sent_shares();
        
        assert!(!was_share_sent("123", 266, 1, None));
        
        add_sent_share_signature("123", 266, 1, None);
        assert!(was_share_sent("123", 266, 1, None));
        assert!(!was_share_sent("123", 266, 2, None));
        
        clear_sent_shares();
        assert!(!was_share_sent("123", 266, 1, None));
    }

    #[test]
    fn test_timestamp_helpers() {
        let current = get_current_timestamp_ms();
        assert!(current > 0);
        
        let past = get_timestamp_seconds_ago(300);
        assert!(past < current);
        assert!(current - past >= 300000); // At least 300 seconds (in ms)
    }
}
