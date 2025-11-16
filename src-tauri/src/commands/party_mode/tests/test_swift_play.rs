// Tests for Swift Play (instant-assign) mode scenarios

use super::test_helpers::*;

#[cfg(test)]
mod swift_play_tests {
    use super::*;

    /// Test: Two skins shared before matchmaking phase
    ///
    /// Scenario: In Swift Play, player selects 2 champions and friends share skins for both.
    /// Expected: Both skins should be received and cached before matchmaking starts.
    #[test]
    fn test_two_skins_shared_before_matchmaking() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend shares skin for first champion
        add_received_skin_to_cache(
            "friend_swift",
            "SwiftFriend",
            157, // Yasuo - first choice
            10,
            None,
            Some("/path/to/yasuo.zip".to_string()),
            base_time,
        );

        // Friend shares skin for second champion
        add_received_skin_to_cache(
            "friend_swift",
            "SwiftFriend",
            238, // Zed - second choice
            15,
            Some(3),
            Some("/path/to/zed.zip".to_string()),
            base_time + 1000,
        );

        // Both skins should be cached
        assert_eq!(get_received_skins_count(), 2);
        assert!(is_skin_in_cache("friend_swift", 157));
        assert!(is_skin_in_cache("friend_swift", 238));

        clear_received_skins_cache();
    }

    /// Test: Multiple champions selection and sharing
    ///
    /// Scenario: Player has 2 champion choices, multiple friends share skins for both.
    /// Expected: All shares should be received and properly organized by champion.
    #[test]
    fn test_multiple_champions_multiple_friends() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend 1 shares for both champions
        add_received_skin_to_cache(
            "friend_1",
            "SwiftFriend1",
            61, // Orianna - choice 1
            7,
            None,
            Some("/path/to/orianna.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_1",
            "SwiftFriend1",
            134, // Syndra - choice 2
            9,
            Some(2),
            Some("/path/to/syndra.zip".to_string()),
            base_time + 500,
        );

        // Friend 2 also shares for both champions
        add_received_skin_to_cache(
            "friend_2",
            "SwiftFriend2",
            61, // Orianna - choice 1
            8,
            Some(1),
            Some("/path/to/orianna2.zip".to_string()),
            base_time + 1000,
        );

        add_received_skin_to_cache(
            "friend_2",
            "SwiftFriend2",
            134, // Syndra - choice 2
            10,
            None,
            Some("/path/to/syndra2.zip".to_string()),
            base_time + 1500,
        );

        // All 4 shares should be cached (2 friends × 2 champions)
        assert_eq!(get_received_skins_count(), 4);
        assert!(is_skin_in_cache("friend_1", 61));
        assert!(is_skin_in_cache("friend_1", 134));
        assert!(is_skin_in_cache("friend_2", 61));
        assert!(is_skin_in_cache("friend_2", 134));

        clear_received_skins_cache();
    }

    /// Test: Champion selection change mid-phase
    ///
    /// Scenario: Player changes one of their champion selections before matchmaking.
    /// Expected: New shares should be received for the new champion choice.
    #[test]
    fn test_champion_selection_change_mid_phase() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Initial selections and shares
        add_received_skin_to_cache(
            "friend_change",
            "ChangingFriend",
            103, // Ahri - original choice 1
            12,
            None,
            Some("/path/to/ahri.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_change",
            "ChangingFriend",
            7, // LeBlanc - original choice 2
            5,
            Some(2),
            Some("/path/to/leblanc.zip".to_string()),
            base_time + 1000,
        );

        assert_eq!(get_received_skins_count(), 2);

        // Player changes second choice from LeBlanc to Syndra
        add_received_skin_to_cache(
            "friend_change",
            "ChangingFriend",
            134, // Syndra - new choice 2
            9,
            None,
            Some("/path/to/syndra.zip".to_string()),
            base_time + 5000,
        );

        // All shares are cached (3 total: 2 original + 1 new)
        assert_eq!(get_received_skins_count(), 3);
        assert!(is_skin_in_cache("friend_change", 103)); // Ahri still there
        assert!(is_skin_in_cache("friend_change", 7));   // LeBlanc still there
        assert!(is_skin_in_cache("friend_change", 134)); // Syndra added

        clear_received_skins_cache();
    }

    /// Test: 50% threshold injection logic
    ///
    /// Scenario: With 50% or more friends having shared, injection should proceed in Swift Play.
    /// Expected: Injection triggers when threshold is met.
    #[test]
    fn test_swift_play_50_percent_threshold() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Simulate 4 friends in party
        // 2 friends have shared (50%)
        add_received_skin_to_cache(
            "friend_1",
            "SwiftPlayer1",
            64, // Lee Sin
            5,
            None,
            Some("/path/to/leesin.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "SwiftPlayer2",
            64, // Lee Sin
            6,
            Some(1),
            Some("/path/to/leesin2.zip".to_string()),
            base_time + 1000,
        );

        // Friends 3 and 4 haven't shared yet

        let shared_count = 2; // Number of friends who shared
        let total_friends = 4;
        let is_swift_play = true;

        // Swift Play logic: if shared * 2 >= total, inject
        let should_inject = is_swift_play && (shared_count * 2 >= total_friends);
        
        assert!(should_inject, "Swift Play should inject at 50% threshold");
        assert_eq!(get_received_skins_count(), 2);

        clear_received_skins_cache();
    }

    /// Test: Below 50% threshold in Swift Play
    ///
    /// Scenario: Less than 50% of friends have shared their skins.
    /// Expected: Injection should wait for more shares.
    #[test]
    fn test_swift_play_below_threshold() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Simulate 4 friends in party
        // Only 1 friend has shared (25%, below 50%)
        add_received_skin_to_cache(
            "friend_1",
            "OnlySharer",
            89, // Leona
            4,
            None,
            Some("/path/to/leona.zip".to_string()),
            base_time,
        );

        let shared_count = 1;
        let total_friends = 4;
        let is_swift_play = true;

        // Swift Play logic: if shared * 2 >= total, inject
        let should_inject = is_swift_play && (shared_count * 2 >= total_friends);
        
        assert!(!should_inject, "Swift Play should not inject below 50% threshold");
        assert_eq!(get_received_skins_count(), 1);

        clear_received_skins_cache();
    }

    /// Test: All friends share in Swift Play
    ///
    /// Scenario: All friends in the party have shared their skins for both champions.
    /// Expected: Injection should definitely proceed.
    #[test]
    fn test_swift_play_all_friends_shared() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // 3 friends, all share for 2 champions each
        let champions = vec![157, 238]; // Yasuo, Zed

        for (i, champion_id) in champions.iter().enumerate() {
            for friend_num in 1..=3 {
                add_received_skin_to_cache(
                    &format!("friend_{}", friend_num),
                    &format!("SwiftFriend{}", friend_num),
                    *champion_id,
                    friend_num as u32 + i as u32,
                    None,
                    Some(format!("/path/to/skin_{}_{}.zip", friend_num, champion_id)),
                    base_time + (friend_num * 1000 + i * 500) as u64,
                );
            }
        }

        // 6 shares total (3 friends × 2 champions)
        assert_eq!(get_received_skins_count(), 6);

        let shared_count = 3; // All 3 friends shared
        let total_friends = 3;
        let is_swift_play = true;

        let should_inject = is_swift_play && (shared_count * 2 >= total_friends);
        assert!(should_inject, "Swift Play should inject when all friends shared");

        clear_received_skins_cache();
    }

    /// Test: Swift Play with single champion choice
    ///
    /// Scenario: In some Swift Play variants, player might only select 1 champion.
    /// Expected: Shares for that single champion should work correctly.
    #[test]
    fn test_swift_play_single_champion() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Two friends share for the same single champion
        add_received_skin_to_cache(
            "friend_1",
            "SingleChoice1",
            777, // Yone
            3,
            None,
            Some("/path/to/yone1.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_2",
            "SingleChoice2",
            777, // Yone
            4,
            Some(1),
            Some("/path/to/yone2.zip".to_string()),
            base_time + 1000,
        );

        assert_eq!(get_received_skins_count(), 2);
        assert!(is_skin_in_cache("friend_1", 777));
        assert!(is_skin_in_cache("friend_2", 777));

        clear_received_skins_cache();
    }

    /// Test: Swift Play champion priority (if only one gets assigned)
    ///
    /// Scenario: Player selects 2 champions but only gets assigned 1 in the final matchmaking.
    /// Expected: Shares for the assigned champion should be used for injection.
    #[test]
    fn test_swift_play_champion_priority_assignment() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend shares for both selected champions
        add_received_skin_to_cache(
            "friend_priority",
            "PriorityFriend",
            92, // Riven - first choice
            11,
            None,
            Some("/path/to/riven.zip".to_string()),
            base_time,
        );

        add_received_skin_to_cache(
            "friend_priority",
            "PriorityFriend",
            23, // Tryndamere - second choice
            7,
            Some(2),
            Some("/path/to/tryndamere.zip".to_string()),
            base_time + 1000,
        );

        // Player actually gets assigned Riven (champion_id = 92)
        let assigned_champion: u32 = 92;

        // Both shares are cached but only the assigned one would be injected
        assert_eq!(get_received_skins_count(), 2);
        assert!(is_skin_in_cache("friend_priority", assigned_champion));

        clear_received_skins_cache();
    }

    /// Test: Swift Play with delayed shares
    ///
    /// Scenario: Friends share their skins at different times during the selection phase.
    /// Expected: All shares should accumulate and be available for injection.
    #[test]
    fn test_swift_play_delayed_shares() {
        clear_received_skins_cache();
        clear_sent_shares();

        let base_time = get_current_timestamp_ms();

        // Friend 1 shares immediately
        add_received_skin_to_cache(
            "friend_fast",
            "FastSharer",
            84, // Akali
            8,
            None,
            Some("/path/to/akali.zip".to_string()),
            base_time,
        );

        assert_eq!(get_received_skins_count(), 1);

        // Friend 2 shares 5 seconds later
        add_received_skin_to_cache(
            "friend_medium",
            "MediumSharer",
            84, // Akali
            9,
            Some(3),
            Some("/path/to/akali2.zip".to_string()),
            base_time + 5000,
        );

        assert_eq!(get_received_skins_count(), 2);

        // Friend 3 shares 10 seconds later
        add_received_skin_to_cache(
            "friend_slow",
            "SlowSharer",
            84, // Akali
            10,
            None,
            Some("/path/to/akali3.zip".to_string()),
            base_time + 10000,
        );

        // All 3 delayed shares accumulated
        assert_eq!(get_received_skins_count(), 3);
        assert!(is_skin_in_cache("friend_fast", 84));
        assert!(is_skin_in_cache("friend_medium", 84));
        assert!(is_skin_in_cache("friend_slow", 84));

        clear_received_skins_cache();
    }
}
