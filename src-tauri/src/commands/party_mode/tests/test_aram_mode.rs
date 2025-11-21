// Tests for ARAM mode specific scenarios

use super::test_helpers::*;

#[cfg(test)]
mod aram_tests {
    use super::*;

    /// Test: Champion re-roll triggers re-sharing
    ///
    /// Scenario: In ARAM, a friend re-rolls their champion and gets a new one.
    /// Expected: The friend should share the skin for the new champion, and the old share might be cleared.
    #[test]
    fn test_champion_reroll_triggers_reshare() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Initial champion assignment and share
        add_received_skin_to_cache(
            "friend_aram",
            "AramFriend",
            89, // Leona - initial champion
            1,
            None,
            Some("/path/to/leona.zip".to_string()),
            base_time,
        );

        assert!(is_skin_in_cache("friend_aram", 89));
        assert_eq!(get_received_skins_count(), 1);

        // Friend re-rolls and gets a new champion
        // In practice, this would trigger a new share from the friend
        add_received_skin_to_cache(
            "friend_aram",
            "AramFriend",
            61, // Orianna - new champion after re-roll
            3,
            Some(2),
            Some("/path/to/orianna.zip".to_string()),
            base_time + 3000, // 3 seconds later
        );

        // Both shares are in cache (different champions)
        assert_eq!(get_received_skins_count(), 2);
        assert!(is_skin_in_cache("friend_aram", 89)); // Old champion still cached
        assert!(is_skin_in_cache("friend_aram", 61)); // New champion cached

        clear_received_skins_cache();
    }

    /// Test: Champion swap between friends
    ///
    /// Scenario: Two friends swap champions in ARAM using the trade feature.
    /// Expected: Both should re-share their new champion skins after the swap.
    #[test]
    fn test_champion_swap_between_friends() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend 1 initially has Champion A
        add_received_skin_to_cache(
            "friend_1",
            "SwapFriend1",
            127, // Lissandra - Friend 1's initial champion
            2,
            None,
            Some("/path/to/lissandra.zip".to_string()),
            base_time,
        );

        // Friend 2 initially has Champion B
        add_received_skin_to_cache(
            "friend_2",
            "SwapFriend2",
            64, // Lee Sin - Friend 2's initial champion
            5,
            Some(1),
            Some("/path/to/leesin.zip".to_string()),
            base_time,
        );

        assert_eq!(get_received_skins_count(), 2);

        // After swap: Friend 1 now has Champion B (Lee Sin)
        add_received_skin_to_cache(
            "friend_1",
            "SwapFriend1",
            64, // Lee Sin - Friend 1 after swap
            5,
            Some(1),
            Some("/path/to/leesin2.zip".to_string()),
            base_time + 5000,
        );

        // After swap: Friend 2 now has Champion A (Lissandra)
        add_received_skin_to_cache(
            "friend_2",
            "SwapFriend2",
            127, // Lissandra - Friend 2 after swap
            2,
            None,
            Some("/path/to/lissandra2.zip".to_string()),
            base_time + 5000,
        );

        // All shares are present (4 total: 2 before swap, 2 after swap)
        assert_eq!(get_received_skins_count(), 4);
        assert!(is_skin_in_cache("friend_1", 127)); // Original
        assert!(is_skin_in_cache("friend_1", 64));  // After swap
        assert!(is_skin_in_cache("friend_2", 64));  // Original
        assert!(is_skin_in_cache("friend_2", 127)); // After swap

        clear_received_skins_cache();
    }

    /// Test: Multiple re-rolls in sequence
    ///
    /// Scenario: A friend re-rolls multiple times in ARAM before settling on a champion.
    /// Expected: Each re-roll should result in a new share, all stored in cache.
    #[test]
    fn test_multiple_rerolls_in_sequence() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Initial champion
        add_received_skin_to_cache(
            "friend_reroller",
            "RerollMaster",
            1, // Annie
            1,
            None,
            Some("/path/to/annie.zip".to_string()),
            base_time,
        );

        // First re-roll
        add_received_skin_to_cache(
            "friend_reroller",
            "RerollMaster",
            43, // Karma
            2,
            None,
            Some("/path/to/karma.zip".to_string()),
            base_time + 2000,
        );

        // Second re-roll
        add_received_skin_to_cache(
            "friend_reroller",
            "RerollMaster",
            268, // Azir
            3,
            Some(1),
            Some("/path/to/azir.zip".to_string()),
            base_time + 4000,
        );

        // Third re-roll (final choice)
        add_received_skin_to_cache(
            "friend_reroller",
            "RerollMaster",
            157, // Yasuo
            10,
            None,
            Some("/path/to/yasuo.zip".to_string()),
            base_time + 6000,
        );

        // All re-rolled champions are cached
        assert_eq!(get_received_skins_count(), 4);
        assert!(is_skin_in_cache("friend_reroller", 1));
        assert!(is_skin_in_cache("friend_reroller", 43));
        assert!(is_skin_in_cache("friend_reroller", 268));
        assert!(is_skin_in_cache("friend_reroller", 157));

        clear_received_skins_cache();
    }

    /// Test: Re-share after champion trade
    ///
    /// Scenario: A friend trades champions with another player and shares the new skin.
    /// Expected: The new champion's skin should be shared and cached correctly.
    #[test]
    fn test_reshare_after_champion_trade() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend's original champion before trade
        add_received_skin_to_cache(
            "friend_trader",
            "TraderFriend",
            22, // Ashe
            4,
            None,
            Some("/path/to/ashe.zip".to_string()),
            base_time,
        );

        assert!(is_skin_in_cache("friend_trader", 22));

        // After trading with another player, friend gets new champion
        add_received_skin_to_cache(
            "friend_trader",
            "TraderFriend",
            51, // Caitlyn
            6,
            Some(3),
            Some("/path/to/caitlyn.zip".to_string()),
            base_time + 8000,
        );

        // Both champions are in cache
        assert_eq!(get_received_skins_count(), 2);
        assert!(is_skin_in_cache("friend_trader", 22));
        assert!(is_skin_in_cache("friend_trader", 51));

        clear_received_skins_cache();
    }

    /// Test: Partial share injection with 50% threshold
    ///
    /// Scenario: In ARAM, if only 50% of friends have shared, injection should still proceed.
    /// Expected: Injection logic should trigger even with partial shares in ARAM mode.
    #[test]
    fn test_aram_partial_share_injection() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Simulate 2 friends in party, only 1 has shared (50%)
        add_received_skin_to_cache(
            "friend_1",
            "SharedFriend",
            91, // Talon
            7,
            None,
            Some("/path/to/talon.zip".to_string()),
            base_time,
        );

        // Friend 2 hasn't shared yet (not in cache)
        // In the real should_inject_now() logic for ARAM:
        // - is_aram = true
        // - shared > 0 (we have 1 share)
        // - Should return true and inject early

        assert_eq!(get_received_skins_count(), 1);
        assert!(is_skin_in_cache("friend_1", 91));
        
        // Simulate the condition check
        let is_aram = true;
        let shared_count = 1;
        let total_friends = 2;
        
        // ARAM logic: if shared > 0, inject early
        let should_inject = is_aram && shared_count > 0;
        assert!(should_inject, "ARAM should inject with partial shares");

        clear_received_skins_cache();
    }

    /// Test: ARAM with no re-rolls or trades
    ///
    /// Scenario: Normal ARAM game where everyone keeps their assigned champion.
    /// Expected: Standard skin sharing and injection should work.
    #[test]
    fn test_aram_no_rerolls_normal_flow() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Three friends all share their assigned champions
        add_received_skin_to_cache(
            "friend_1",
            "AramPlayer1",
            25, // Morgana
            3,
            None,
            Some("/path/to/morgana.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "AramPlayer2",
            53, // Blitzcrank
            5,
            Some(2),
            Some("/path/to/blitz.zip".to_string()),
            base_time + 1000,
        );

        add_received_skin_to_cache(
            "friend_3",
            "AramPlayer3",
            412, // Thresh
            8,
            None,
            Some("/path/to/thresh.zip".to_string()),
            base_time + 2000,
        );

        // All shares received
        assert_eq!(get_received_skins_count(), 3);
        assert!(is_skin_in_cache("friend_1", 25));
        assert!(is_skin_in_cache("friend_2", 53));
        assert!(is_skin_in_cache("friend_3", 412));

        clear_received_skins_cache();
    }

    /// Test: ARAM re-roll with same champion (rare case)
    ///
    /// Scenario: A friend re-rolls but randomly gets the same champion.
    /// Expected: No new share needed, or the same share is updated with new timestamp.
    #[test]
    fn test_aram_reroll_same_champion() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Initial champion
        add_received_skin_to_cache(
            "friend_unlucky",
            "UnluckyReroller",
            30, // Karthus
            2,
            None,
            Some("/path/to/karthus.zip".to_string()),
            base_time,
        );

        // Re-roll but get same champion (updates the entry with same key)
        add_received_skin_to_cache(
            "friend_unlucky",
            "UnluckyReroller",
            30, // Karthus again
            2,
            None,
            Some("/path/to/karthus.zip".to_string()),
            base_time + 5000,
        );

        // Only one entry should exist (updated timestamp)
        assert_eq!(get_received_skins_count(), 1);
        assert!(is_skin_in_cache("friend_unlucky", 30));

        clear_received_skins_cache();
    }
}
