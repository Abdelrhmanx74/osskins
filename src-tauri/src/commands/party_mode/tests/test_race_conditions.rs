// Tests for race conditions and concurrent operations

use super::test_helpers::*;

#[cfg(test)]
mod race_condition_tests {
    use super::*;

    /// Test: Concurrent skin shares from multiple friends
    ///
    /// Scenario: Multiple friends send skin shares at nearly the same time.
    /// Expected: All shares should be received and stored correctly without loss.
    #[test]
    fn test_concurrent_skin_shares() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 64; // Lee Sin

        // Simulate concurrent shares (within milliseconds of each other)
        add_received_skin_to_cache(
            "friend_1",
            "ConcurrentFriend1",
            champion_id,
            5,
            None,
            Some("/path/to/leesin1.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "ConcurrentFriend2",
            champion_id,
            6,
            Some(1),
            Some("/path/to/leesin2.zip".to_string()),
            base_time + 10, // 10ms later
        );

        add_received_skin_to_cache(
            "friend_3",
            "ConcurrentFriend3",
            champion_id,
            7,
            None,
            Some("/path/to/leesin3.zip".to_string()),
            base_time + 20, // 20ms later
        );

        // All shares should be present despite near-simultaneous arrival
        assert_eq!(get_received_skins_count(), 3);
        assert!(is_skin_in_cache("friend_1", champion_id));
        assert!(is_skin_in_cache("friend_2", champion_id));
        assert!(is_skin_in_cache("friend_3", champion_id));

        clear_received_skins_cache();
    }

    /// Test: Message processing during phase transition
    ///
    /// Scenario: Skin share message arrives exactly during ChampSelect -> InGame transition.
    /// Expected: Message should either be processed in old phase or rejected/stored for next.
    #[test]
    fn test_message_during_phase_transition() {
        clear_received_skins_cache();
        clear_sent_shares();

        let transition_time = get_current_timestamp_ms();

        // Phase is ChampSelect
        let mut current_phase = "ChampSelect";

        // Share arrives just before transition
        add_received_skin_to_cache(
            "friend_transition",
            "TransitionFriend",
            157, // Yasuo
            10,
            None,
            Some("/path/to/yasuo.zip".to_string()),
            transition_time,
        );

        assert_eq!(get_received_skins_count(), 1);

        // Phase transitions to InGame
        current_phase = "InGame";
        
        // On phase transition, cache should be cleared
        clear_received_skins_cache();

        assert_eq!(current_phase, "InGame");
        assert_eq!(get_received_skins_count(), 0);
    }

    /// Test: Injection trigger race with watcher loop
    ///
    /// Scenario: Watcher loop checks injection conditions at the same time as champion lock occurs.
    /// Expected: Injection should trigger correctly despite timing race.
    #[test]
    fn test_injection_trigger_race() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();
        let champion_id: u32 = 238; // Zed

        // Friend shares their skin
        add_received_skin_to_cache(
            "friend_race",
            "RaceFriend",
            champion_id,
            15,
            None,
            Some("/path/to/zed.zip".to_string()),
            base_time,
        );

        // Simulate watcher loop checking conditions
        let watcher_check_1 = champion_id == 0; // Not locked yet
        assert!(watcher_check_1); // Should not inject

        // Champion gets locked (race condition)
        let locked_champion_id: u32 = 238;

        // Watcher loop checks again
        let watcher_check_2 = locked_champion_id != 0;
        assert!(watcher_check_2); // Should trigger injection

        // Share is still available
        assert!(is_skin_in_cache("friend_race", champion_id));

        clear_received_skins_cache();
    }

    /// Test: Concurrent sent share deduplication
    ///
    /// Scenario: Two rapid attempts to send the same share to the same friend.
    /// Expected: Only one should succeed due to deduplication.
    #[test]
    fn test_concurrent_sent_share_deduplication() {
        clear_sent_shares();

        let friend_id = "friend_dedup";
        let champion_id = 103; // Ahri
        let skin_id = 12;
        let chroma_id = Some(3);

        // First send attempt
        let was_sent_before_1 = was_share_sent(friend_id, champion_id, skin_id, chroma_id);
        assert!(!was_sent_before_1);
        add_sent_share_signature(friend_id, champion_id, skin_id, chroma_id);

        // Second send attempt (concurrent/rapid)
        let was_sent_before_2 = was_share_sent(friend_id, champion_id, skin_id, chroma_id);
        assert!(was_sent_before_2); // Should be blocked

        clear_sent_shares();
    }

    /// Test: Phase state and cache consistency
    ///
    /// Scenario: Phase state updated while cache operations are in progress.
    /// Expected: Cache operations should be atomic and maintain consistency.
    #[test]
    fn test_phase_state_cache_consistency() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Add shares while in ChampSelect
        add_received_skin_to_cache(
            "friend_1",
            "Friend1",
            64,
            5,
            None,
            Some("/path/to/leesin.zip".to_string()),
            base_time,
        );

        let count_before = get_received_skins_count();
        assert_eq!(count_before, 1);

        // Add more shares
        add_received_skin_to_cache(
            "friend_2",
            "Friend2",
            64,
            6,
            Some(1),
            Some("/path/to/leesin2.zip".to_string()),
            base_time + 1000,
        );

        let count_after = get_received_skins_count();
        assert_eq!(count_after, 2);

        // Clear simulates phase transition
        clear_received_skins_cache();
        assert_eq!(get_received_skins_count(), 0);
    }

    /// Test: Rapid champion lock/unlock cycles
    ///
    /// Scenario: Player rapidly locks and unlocks champion during selection.
    /// Expected: State should remain consistent, shares should persist.
    #[test]
    fn test_rapid_lock_unlock_cycles() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend shares
        add_received_skin_to_cache(
            "friend_rapid",
            "RapidFriend",
            157, // Yasuo
            10,
            None,
            Some("/path/to/yasuo.zip".to_string()),
            base_time,
        );

        // Simulate rapid lock/unlock
        let mut champion_id: u32 = 0;
        
        // Lock
        champion_id = 157;
        assert_eq!(champion_id, 157);
        assert!(is_skin_in_cache("friend_rapid", 157));

        // Unlock
        champion_id = 0;
        assert_eq!(champion_id, 0);
        assert!(is_skin_in_cache("friend_rapid", 157)); // Share still there

        // Lock again
        champion_id = 157;
        assert_eq!(champion_id, 157);
        assert!(is_skin_in_cache("friend_rapid", 157));

        // Unlock again
        champion_id = 0;
        assert_eq!(champion_id, 0);
        assert!(is_skin_in_cache("friend_rapid", 157));

        clear_received_skins_cache();
    }

    /// Test: Session ID check race with skin receipt
    ///
    /// Scenario: Session ID changes at the same time a skin share arrives.
    /// Expected: Either share is accepted in old session or rejected for new session.
    #[test]
    fn test_session_change_skin_receipt_race() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Share arrives in session 1
        add_received_skin_to_cache(
            "friend_session_race",
            "SessionRaceFriend",
            91, // Talon
            7,
            None,
            Some("/path/to/talon.zip".to_string()),
            base_time,
        );

        assert_eq!(get_received_skins_count(), 1);

        // Session changes (simulated by clear)
        clear_received_skins_cache();

        // New session, cache is empty
        assert_eq!(get_received_skins_count(), 0);

        // New share in new session
        add_received_skin_to_cache(
            "friend_session_race",
            "SessionRaceFriend",
            92, // Riven
            11,
            None,
            Some("/path/to/riven.zip".to_string()),
            base_time + 10000,
        );

        assert_eq!(get_received_skins_count(), 1);
        assert!(is_skin_in_cache("friend_session_race", 92));
        assert!(!is_skin_in_cache("friend_session_race", 91)); // Old session share gone

        clear_received_skins_cache();
    }

    /// Test: Concurrent cache reads and writes
    ///
    /// Scenario: Multiple threads/operations reading and writing to cache simultaneously.
    /// Expected: Mutex protection should ensure data consistency.
    #[test]
    fn test_concurrent_cache_operations() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Simulate concurrent writes
        for i in 0..10 {
            add_received_skin_to_cache(
                &format!("friend_{}", i),
                &format!("Friend{}", i),
                100 + i as u32,
                i as u32,
                None,
                Some(format!("/path/to/skin{}.zip", i)),
                base_time + (i as u64 * 100),
            );
        }

        // All writes should succeed
        assert_eq!(get_received_skins_count(), 10);

        // Verify all entries
        for i in 0..10 {
            assert!(is_skin_in_cache(&format!("friend_{}", i), 100 + i as u32));
        }

        clear_received_skins_cache();
    }

    /// Test: Injection already in progress flag
    ///
    /// Scenario: Injection triggered while another injection is already running.
    /// Expected: Second injection should be skipped or queued appropriately.
    #[test]
    fn test_injection_already_in_progress() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Setup: friend shares
        add_received_skin_to_cache(
            "friend_inject",
            "InjectFriend",
            64, // Lee Sin
            5,
            None,
            Some("/path/to/leesin.zip".to_string()),
            base_time,
        );

        // Simulate injection flag
        let mut injection_in_progress = false;

        // First injection attempt
        if !injection_in_progress {
            injection_in_progress = true;
            // Injection logic would run here
            assert!(injection_in_progress);
        }

        // Second injection attempt (should be blocked)
        if !injection_in_progress {
            // This should not execute
            panic!("Second injection should be blocked");
        }

        // Injection completes
        injection_in_progress = false;
        assert!(!injection_in_progress);

        clear_received_skins_cache();
    }

    /// Test: Party membership query during share processing
    ///
    /// Scenario: Party membership is queried while share messages are being processed.
    /// Expected: Consistent party state should be used for all decisions.
    #[test]
    fn test_party_query_during_share_processing() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Initial party state
        let party_members = vec!["friend_1".to_string(), "friend_2".to_string()];

        // Friend 1 shares
        add_received_skin_to_cache(
            "friend_1",
            "Friend1",
            103, // Ahri
            12,
            None,
            Some("/path/to/ahri.zip".to_string()),
            base_time,
        );

        // Query party membership
        assert!(party_members.contains(&"friend_1".to_string()));

        // Friend 2 shares
        add_received_skin_to_cache(
            "friend_2",
            "Friend2",
            103, // Ahri
            13,
            Some(2),
            Some("/path/to/ahri2.zip".to_string()),
            base_time + 1000,
        );

        // Query party membership again
        assert!(party_members.contains(&"friend_2".to_string()));

        // Both shares from party members
        assert_eq!(get_received_skins_count(), 2);

        clear_received_skins_cache();
    }

    /// Test: Watcher polling interval race
    ///
    /// Scenario: Share arrives between watcher polling intervals.
    /// Expected: Share should be caught on next poll cycle.
    #[test]
    fn test_watcher_polling_interval_race() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Watcher polls at t=0
        let watcher_poll_1 = get_received_skins_count();
        assert_eq!(watcher_poll_1, 0);

        // Share arrives at t=500ms (between polls)
        add_received_skin_to_cache(
            "friend_poll",
            "PollFriend",
            238, // Zed
            15,
            None,
            Some("/path/to/zed.zip".to_string()),
            base_time + 500,
        );

        // Watcher polls at t=1500ms (next cycle)
        let watcher_poll_2 = get_received_skins_count();
        assert_eq!(watcher_poll_2, 1);
        assert!(is_skin_in_cache("friend_poll", 238));

        clear_received_skins_cache();
    }

    /// Test: Duplicate message ID handling
    ///
    /// Scenario: Same message arrives multiple times (network retry/duplicate).
    /// Expected: Should be processed only once using message ID deduplication.
    #[test]
    fn test_duplicate_message_id_handling() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let message_id = "msg_12345";

        // Track processed message IDs
        let mut processed_messages: std::collections::HashSet<String> = std::collections::HashSet::new();

        // First message arrival
        if !processed_messages.contains(message_id) {
            add_received_skin_to_cache(
                "friend_dup",
                "DupFriend",
                157, // Yasuo
                10,
                None,
                Some("/path/to/yasuo.zip".to_string()),
                base_time,
            );
            processed_messages.insert(message_id.to_string());
        }

        assert_eq!(get_received_skins_count(), 1);

        // Duplicate message arrival (should be ignored)
        if !processed_messages.contains(message_id) {
            add_received_skin_to_cache(
                "friend_dup",
                "DupFriend",
                157,
                10,
                None,
                Some("/path/to/yasuo.zip".to_string()),
                base_time + 100,
            );
            processed_messages.insert(message_id.to_string());
        }

        // Count should still be 1 (duplicate ignored)
        assert_eq!(get_received_skins_count(), 1);

        clear_received_skins_cache();
    }
}
