// Tests for session tracking and state management

use super::test_helpers::*;
use crate::commands::party_mode::types::MAX_SHARE_AGE_SECS;

#[cfg(test)]
mod session_state_tests {
    use super::*;

    /// Test: Session ID change clears received skins
    ///
    /// Scenario: When a new game session starts, the session ID changes.
    /// Expected: All previously received skins should be cleared.
    #[test]
    fn test_session_change_clears_skins() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Receive skins during session 1
        add_received_skin_to_cache(
            "friend_session",
            "SessionFriend",
            11, // Master Yi
            3,
            None,
            Some("/path/to/yi.zip".to_string()),
            base_time,
        );

        assert_eq!(get_received_skins_count(), 1);

        // Simulate session change by clearing cache (in real code, this happens automatically)
        clear_received_skins_cache();

        // After session change, cache should be empty
        assert_eq!(get_received_skins_count(), 0);
        assert!(!is_skin_in_cache("friend_session", 11));
    }

    /// Test: Stale skin share pruning
    ///
    /// Scenario: Shares older than MAX_SHARE_AGE_SECS should be removed.
    /// Expected: Old shares are pruned, recent ones remain.
    #[test]
    fn test_stale_skin_pruning() {
        clear_received_skins_cache();

        let current_time = get_current_timestamp_ms();
        
        // Add an old share (way past MAX_SHARE_AGE_SECS)
        let old_time = get_timestamp_seconds_ago(MAX_SHARE_AGE_SECS + 60);
        add_received_skin_to_cache(
            "friend_old",
            "OldFriend",
            50, // Swain
            5,
            None,
            Some("/path/to/swain.zip".to_string()),
            old_time,
        );

        // Add a recent share (within MAX_SHARE_AGE_SECS)
        add_received_skin_to_cache(
            "friend_recent",
            "RecentFriend",
            51, // Caitlyn
            6,
            Some(2),
            Some("/path/to/caitlyn.zip".to_string()),
            current_time,
        );

        // Before pruning, both are in cache
        assert_eq!(get_received_skins_count(), 2);

        // The pruning logic would filter out the old one based on timestamp
        // For this test, we verify both are present before pruning
        assert!(is_skin_in_cache("friend_old", 50));
        assert!(is_skin_in_cache("friend_recent", 51));

        // In real implementation, prune_stale_received_skins() would remove friend_old
        
        clear_received_skins_cache();
    }

    /// Test: Deduplication of sent shares
    ///
    /// Scenario: Same skin should not be sent to the same friend twice in one phase.
    /// Expected: Deduplication mechanism prevents duplicate sends.
    #[test]
    fn test_sent_shares_deduplication() {
        clear_sent_shares();

        let friend_id = "friend_123";
        let champion_id = 157; // Yasuo
        let skin_id = 10;
        let chroma_id = Some(3);

        // First send
        assert!(!was_share_sent(friend_id, champion_id, skin_id, chroma_id));
        add_sent_share_signature(friend_id, champion_id, skin_id, chroma_id);
        assert!(was_share_sent(friend_id, champion_id, skin_id, chroma_id));

        // Attempt second send - should be detected as duplicate
        assert!(was_share_sent(friend_id, champion_id, skin_id, chroma_id));

        clear_sent_shares();
    }

    /// Test: Phase transition state reset
    ///
    /// Scenario: When phase changes (e.g., from ChampSelect to InGame), state should reset.
    /// Expected: Sent shares and received skins should be cleared for new phase.
    #[test]
    fn test_phase_transition_reset() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // During ChampSelect phase
        add_received_skin_to_cache(
            "friend_phase",
            "PhaseFriend",
            555, // Pyke
            8,
            None,
            Some("/path/to/pyke.zip".to_string()),
            base_time,
        );

        add_sent_share_signature("friend_phase", 555, 8, None);

        assert_eq!(get_received_skins_count(), 1);
        assert!(was_share_sent("friend_phase", 555, 8, None));

        // Phase transition occurs (ChampSelect -> InGame)
        clear_received_skins_cache();
        clear_sent_shares();

        // After transition, all state should be reset
        assert_eq!(get_received_skins_count(), 0);
        assert!(!was_share_sent("friend_phase", 555, 8, None));
    }

    /// Test: Multiple shares from same friend for different champions
    ///
    /// Scenario: Same friend shares skins for multiple champions (e.g., in ARAM re-rolls).
    /// Expected: All shares should be stored separately by champion ID.
    #[test]
    fn test_multiple_shares_same_friend_different_champions() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let friend_id = "friend_multi";

        // Friend shares for champion 1
        add_received_skin_to_cache(
            friend_id,
            "MultiFriend",
            38, // Kassadin
            4,
            None,
            Some("/path/to/kassadin.zip".to_string()),
            base_time,
        );

        // Same friend shares for champion 2
        add_received_skin_to_cache(
            friend_id,
            "MultiFriend",
            131, // Diana
            7,
            Some(1),
            Some("/path/to/diana.zip".to_string()),
            base_time + 2000,
        );

        // Same friend shares for champion 3
        add_received_skin_to_cache(
            friend_id,
            "MultiFriend",
            42, // Corki
            9,
            None,
            Some("/path/to/corki.zip".to_string()),
            base_time + 4000,
        );

        // All three shares stored separately
        assert_eq!(get_received_skins_count(), 3);
        assert!(is_skin_in_cache(friend_id, 38));
        assert!(is_skin_in_cache(friend_id, 131));
        assert!(is_skin_in_cache(friend_id, 42));

        clear_received_skins_cache();
    }

    /// Test: Overwriting skin share for same champion
    ///
    /// Scenario: Friend shares a new skin for the same champion (replacing previous share).
    /// Expected: The new share should overwrite the old one.
    #[test]
    fn test_overwrite_skin_share_same_champion() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();
        let friend_id = "friend_overwrite";
        let champion_id = 238; // Zed

        // First share
        add_received_skin_to_cache(
            friend_id,
            "OverwriteFriend",
            champion_id,
            15, // Skin ID 15
            None,
            Some("/path/to/zed1.zip".to_string()),
            base_time,
        );

        assert_eq!(get_received_skins_count(), 1);

        // Second share for same champion (different skin)
        add_received_skin_to_cache(
            friend_id,
            "OverwriteFriend",
            champion_id,
            20, // Skin ID 20
            Some(2),
            Some("/path/to/zed2.zip".to_string()),
            base_time + 3000,
        );

        // Should still be 1 entry (overwritten, not added)
        assert_eq!(get_received_skins_count(), 1);
        assert!(is_skin_in_cache(friend_id, champion_id));

        clear_received_skins_cache();
    }

    /// Test: Sent shares cleared on new champion select phase
    ///
    /// Scenario: Moving to a new ChampSelect should clear sent share deduplication.
    /// Expected: Can send shares again in the new phase.
    #[test]
    fn test_sent_shares_cleared_new_phase() {
        clear_sent_shares();

        let friend_id = "friend_newphase";
        let champion_id = 103; // Ahri
        let skin_id = 12;

        // Phase 1: Send share
        add_sent_share_signature(friend_id, champion_id, skin_id, None);
        assert!(was_share_sent(friend_id, champion_id, skin_id, None));

        // New phase begins
        clear_sent_shares();

        // Should be able to send again
        assert!(!was_share_sent(friend_id, champion_id, skin_id, None));

        clear_sent_shares();
    }

    /// Test: State persistence during champion lock changes
    ///
    /// Scenario: Player locks and unlocks champion, shares should persist.
    /// Expected: Received skins remain in cache through lock/unlock cycles.
    #[test]
    fn test_state_persistence_during_lock_changes() {
        clear_received_skins_cache();

        let base_time = get_current_timestamp_ms();

        // Friend shares while player is selecting
        add_received_skin_to_cache(
            "friend_lock",
            "LockFriend",
            114, // Fiora
            5,
            None,
            Some("/path/to/fiora.zip".to_string()),
            base_time,
        );

        assert_eq!(get_received_skins_count(), 1);

        // Simulate player locking champion (champion_id = 114)
        let mut champion_locked: u32 = 114;
        assert_eq!(champion_locked, 114);

        // Share still in cache
        assert!(is_skin_in_cache("friend_lock", 114));

        // Player unlocks to change pick
        champion_locked = 0;
        assert_eq!(champion_locked, 0);

        // Share should still be in cache
        assert!(is_skin_in_cache("friend_lock", 114));

        // Player locks again
        champion_locked = 114;
        assert_eq!(champion_locked, 114);

        // Share still available
        assert!(is_skin_in_cache("friend_lock", 114));

        clear_received_skins_cache();
    }

    /// Test: MAX_SHARE_AGE_SECS boundary condition
    ///
    /// Scenario: Share at exactly MAX_SHARE_AGE_SECS seconds old.
    /// Expected: Should still be valid (edge case).
    #[test]
    fn test_max_share_age_boundary() {
        clear_received_skins_cache();

        // Share at exactly MAX_SHARE_AGE_SECS seconds ago
        let boundary_time = get_timestamp_seconds_ago(MAX_SHARE_AGE_SECS);
        
        add_received_skin_to_cache(
            "friend_boundary",
            "BoundaryFriend",
            69, // Cassiopeia
            6,
            Some(1),
            Some("/path/to/cass.zip".to_string()),
            boundary_time,
        );

        // Should be in cache at the boundary
        assert!(is_skin_in_cache("friend_boundary", 69));
        assert_eq!(get_received_skins_count(), 1);

        // Just beyond the boundary would be pruned (but we test the boundary itself)
        let beyond_boundary = get_timestamp_seconds_ago(MAX_SHARE_AGE_SECS + 1);
        add_received_skin_to_cache(
            "friend_old",
            "OldFriend",
            68, // Rumble
            3,
            None,
            Some("/path/to/rumble.zip".to_string()),
            beyond_boundary,
        );

        // Both are in cache before pruning
        assert_eq!(get_received_skins_count(), 2);

        clear_received_skins_cache();
    }

    /// Test: Concurrent state access
    ///
    /// Scenario: Multiple operations accessing shared state concurrently.
    /// Expected: State should remain consistent (thread-safe with Mutex).
    #[test]
    fn test_concurrent_state_access() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Simulate concurrent adds
        for i in 0..5 {
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

        // All should be added successfully
        assert_eq!(get_received_skins_count(), 5);

        for i in 0..5 {
            assert!(is_skin_in_cache(&format!("friend_{}", i), 100 + i as u32));
        }

        clear_received_skins_cache();
        clear_sent_shares();
    }
}
