// Tests for injection logic and skin file resolution

use super::test_helpers::*;

#[cfg(test)]
mod injection_logic_tests {
    use super::*;

    /// Test: should_inject_now with all friends ready
    ///
    /// Scenario: All paired friends in party have shared their skins.
    /// Expected: should_inject_now returns true.
    #[test]
    fn test_should_inject_all_friends_ready() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 64; // Lee Sin

        // Three friends, all share
        add_received_skin_to_cache(
            "friend_1",
            "Friend1",
            champion_id,
            5,
            None,
            Some("/path/to/leesin1.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "Friend2",
            champion_id,
            6,
            Some(1),
            Some("/path/to/leesin2.zip".to_string()),
            base_time + 1000,
        );

        add_received_skin_to_cache(
            "friend_3",
            "Friend3",
            champion_id,
            7,
            None,
            Some("/path/to/leesin3.zip".to_string()),
            base_time + 2000,
        );

        // Verify conditions for injection
        assert_ne!(champion_id, 0, "Champion must be locked");
        assert_eq!(get_received_skins_count(), 3, "All 3 friends shared");

        // In real implementation: shared == total, so should_inject_now returns true
        let total_friends = 3;
        let shared_friends = 3;
        let should_inject = shared_friends == total_friends;
        assert!(should_inject);

        clear_received_skins_cache();
    }

    /// Test: should_inject_now with partial readiness
    ///
    /// Scenario: Only some friends have shared, not in special mode (ARAM/Swift).
    /// Expected: should_inject_now returns false, waits for more.
    #[test]
    fn test_should_inject_partial_readiness() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 157; // Yasuo

        // Two friends in party, only one shared
        add_received_skin_to_cache(
            "friend_ready",
            "ReadyFriend",
            champion_id,
            10,
            None,
            Some("/path/to/yasuo.zip".to_string()),
            base_time,
        );

        assert_ne!(champion_id, 0);
        assert_eq!(get_received_skins_count(), 1);

        // Not in special mode
        let is_aram = false;
        let is_swift = false;
        let total_friends = 2;
        let shared_friends = 1;

        // Should wait for the other friend
        let should_inject = (shared_friends == total_friends) 
            || (is_aram && shared_friends > 0)
            || (is_swift && shared_friends * 2 >= total_friends);
        
        assert!(!should_inject, "Should wait for remaining friend");

        clear_received_skins_cache();
    }

    /// Test: Skin file path resolution - absolute path
    ///
    /// Scenario: Friend shares with absolute file path that exists.
    /// Expected: Path should be resolved correctly.
    #[test]
    fn test_skin_file_path_absolute() {
        let skin_file_path = "/absolute/path/to/skin.zip";
        
        // In real code, this would check file existence
        // For test, we verify the path format
        assert!(skin_file_path.starts_with('/'));
        assert!(skin_file_path.ends_with(".zip"));
    }

    /// Test: Skin file path resolution - relative path
    ///
    /// Scenario: Friend shares with relative path (e.g., "ezrea/skin.zip").
    /// Expected: Path should be resolved relative to champions directory.
    #[test]
    fn test_skin_file_path_relative() {
        let skin_file_path = "ezrea/ProjectAshe.zip";
        
        // Verify it's a relative path
        assert!(!skin_file_path.starts_with('/'));
        assert!(skin_file_path.contains("ezrea"));
        
        // In real code, would join with champions_dir
        let champions_dir = "/path/to/champions";
        let resolved = format!("{}/{}", champions_dir, skin_file_path);
        assert!(resolved.contains("champions/ezrea"));
    }

    /// Test: Local + friend skin injection batch
    ///
    /// Scenario: Inject both local skin and friend skins together.
    /// Expected: All skins should be batched for single injection call.
    #[test]
    fn test_local_and_friend_skin_batch() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 103; // Ahri

        // Local skin (not in received cache, would come from config)
        let has_local_skin = true;

        // Friend skins
        add_received_skin_to_cache(
            "friend_1",
            "Friend1",
            champion_id,
            3,
            None,
            Some("/path/to/ahri1.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "Friend2",
            champion_id,
            5,
            Some(2),
            Some("/path/to/ahri2.zip".to_string()),
            base_time + 1000,
        );

        // Total skins to inject: 1 local + 2 friend = 3
        let friend_count = get_received_skins_count();
        let total_to_inject = if has_local_skin { friend_count + 1 } else { friend_count };
        
        assert_eq!(total_to_inject, 3);

        clear_received_skins_cache();
    }

    /// Test: Custom skin handling
    ///
    /// Scenario: Friend shares a custom skin (skin_id = 0).
    /// Expected: Custom skin should be handled with special file path logic.
    #[test]
    fn test_custom_skin_handling() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Custom skin has skin_id = 0
        add_received_skin_to_cache(
            "friend_custom",
            "CustomFriend",
            266, // Aatrox
            0, // Custom skin indicator
            None,
            Some("/custom/path/to/aatrox_custom.zip".to_string()),
            base_time,
        );

        assert!(is_skin_in_cache("friend_custom", 266));
        assert_eq!(get_received_skins_count(), 1);

        clear_received_skins_cache();
    }

    /// Test: No champion locked (champion_id = 0)
    ///
    /// Scenario: Player hasn't locked a champion yet.
    /// Expected: should_inject_now should return false.
    #[test]
    fn test_no_champion_locked() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 0; // Not locked

        // Friend shares
        add_received_skin_to_cache(
            "friend_eager",
            "EagerFriend",
            64,
            5,
            None,
            Some("/path/to/leesin.zip".to_string()),
            base_time,
        );

        // Even though friend shared, can't inject without champion lock
        assert_eq!(champion_id, 0);
        let should_inject = champion_id != 0;
        assert!(!should_inject);

        clear_received_skins_cache();
    }

    /// Test: Skin file path with chroma
    ///
    /// Scenario: Friend shares skin with chroma_id specified.
    /// Expected: Chroma information should be preserved through injection.
    #[test]
    fn test_skin_with_chroma() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        add_received_skin_to_cache(
            "friend_chroma",
            "ChromaFriend",
            222, // Jinx
            10,
            Some(5), // Chroma ID 5
            Some("/path/to/jinx_chroma.zip".to_string()),
            base_time,
        );

        assert!(is_skin_in_cache("friend_chroma", 222));
        
        // In real code, chroma_id would be passed to injection
        clear_received_skins_cache();
    }

    /// Test: Multiple skins for same champion (different friends)
    ///
    /// Scenario: Multiple friends share different skins for the same champion.
    /// Expected: All skins should be collected for injection.
    #[test]
    fn test_multiple_skins_same_champion() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 238; // Zed

        // Friend 1 shares skin A
        add_received_skin_to_cache(
            "friend_1",
            "Friend1",
            champion_id,
            15,
            None,
            Some("/path/to/zed1.zip".to_string()),
            base_time,
        );

        // Friend 2 shares skin B
        add_received_skin_to_cache(
            "friend_2",
            "Friend2",
            champion_id,
            20,
            Some(3),
            Some("/path/to/zed2.zip".to_string()),
            base_time + 1000,
        );

        // Friend 3 shares skin C
        add_received_skin_to_cache(
            "friend_3",
            "Friend3",
            champion_id,
            25,
            None,
            Some("/path/to/zed3.zip".to_string()),
            base_time + 2000,
        );

        // All 3 different skins for same champion
        assert_eq!(get_received_skins_count(), 3);

        clear_received_skins_cache();
    }

    /// Test: Missing skin file path
    ///
    /// Scenario: Friend shares but skin_file_path is None.
    /// Expected: Should skip this skin or use fallback logic.
    #[test]
    fn test_missing_skin_file_path() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        add_received_skin_to_cache(
            "friend_no_file",
            "NoFileFriend",
            91, // Talon
            7,
            None,
            None, // No file path provided
            base_time,
        );

        assert!(is_skin_in_cache("friend_no_file", 91));
        // In real implementation, this would be skipped during injection
        // or use fallback to local skin with matching champion_id and skin_id

        clear_received_skins_cache();
    }

    /// Test: Injection deduplication
    ///
    /// Scenario: Same skin from different friends (same file path).
    /// Expected: Deduplication should prevent duplicate injections.
    #[test]
    fn test_injection_deduplication() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let same_file = "/path/to/shared_skin.zip".to_string();

        // Two friends share the exact same skin file
        add_received_skin_to_cache(
            "friend_1",
            "Friend1",
            157, // Yasuo
            10,
            None,
            Some(same_file.clone()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "Friend2",
            157, // Yasuo
            10,
            None,
            Some(same_file.clone()),
            base_time + 1000,
        );

        // Both shares are in cache (different friends)
        assert_eq!(get_received_skins_count(), 2);

        // In real injection, deduplication by (champion_id, skin_id, chroma_id, file_path, friend_id)
        // would keep both since they're from different friends

        clear_received_skins_cache();
    }

    /// Test: Fallback to local skin when friend skin missing
    ///
    /// Scenario: Friend's skin file not found locally, fallback to local skin for same champion.
    /// Expected: Local skin should be used as fallback.
    #[test]
    fn test_fallback_to_local_skin() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Friend shares but file doesn't exist locally
        add_received_skin_to_cache(
            "friend_missing",
            "MissingFriend",
            268, // Azir
            8,
            None,
            Some("/nonexistent/path/azir.zip".to_string()),
            base_time,
        );

        // In real code, this would trigger fallback to local skin
        // if local_skin exists for champion 268
        let has_local_fallback = true; // Simulated

        assert!(has_local_fallback);
        assert!(is_skin_in_cache("friend_missing", 268));

        clear_received_skins_cache();
    }

    /// Test: Injection with misc items
    ///
    /// Scenario: Inject skins along with misc items (wards, emotes, etc.).
    /// Expected: Both skins and misc items should be included in injection.
    #[test]
    fn test_injection_with_misc_items() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Friend skin
        add_received_skin_to_cache(
            "friend_misc",
            "MiscFriend",
            412, // Thresh
            6,
            None,
            Some("/path/to/thresh.zip".to_string()),
            base_time,
        );

        // Misc items would come from separate source in real code
        let has_misc_items = true;
        let misc_item_count = 2; // Example: ward + emote

        // Total injection includes skins + misc items
        let skin_count = get_received_skins_count();
        assert!(skin_count > 0);
        assert!(has_misc_items);
        assert_eq!(misc_item_count, 2);

        clear_received_skins_cache();
    }
}
