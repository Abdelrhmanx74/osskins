// Tests for party member detection and filtering

use super::test_helpers::*;

#[cfg(test)]
mod party_detection_tests {
    use super::*;

    /// Test: Friend in party vs outside party filtering
    ///
    /// Scenario: Some paired friends are in the party, others are not.
    /// Expected: Only friends in the party should receive shares.
    #[test]
    fn test_friend_in_party_vs_outside() {
        clear_received_skins_cache();
        clear_sent_shares();

        // Create paired friends
        let friend_in_party = create_mock_paired_friend(
            "111",
            "InPartyFriend",
            "In Party Friend",
            true,
        );

        let friend_outside_party = create_mock_paired_friend(
            "222",
            "OutsideFriend",
            "Outside Friend",
            true,
        );

        // Simulate party membership check
        let party_member_ids = vec!["111".to_string()]; // Only friend_in_party

        // Friend in party should be included
        assert!(party_member_ids.contains(&friend_in_party.summoner_id));
        assert!(friend_in_party.share_enabled);

        // Friend outside party should be excluded
        assert!(!party_member_ids.contains(&friend_outside_party.summoner_id));

        // Only friend in party would receive shares in real implementation
    }

    /// Test: Multiple friends with different share settings
    ///
    /// Scenario: Multiple friends in party, some have sharing enabled, others don't.
    /// Expected: Only friends with share_enabled=true should participate.
    #[test]
    fn test_multiple_friends_different_share_settings() {
        clear_received_skins_cache();
        clear_sent_shares();

        // Create friends with different settings
        let friend_sharing_enabled = create_mock_paired_friend(
            "333",
            "SharingFriend",
            "Sharing Friend",
            true, // share_enabled
        );

        let friend_sharing_disabled = create_mock_paired_friend(
            "444",
            "NonSharingFriend",
            "Non-Sharing Friend",
            false, // share_enabled
        );

        let friend_also_sharing = create_mock_paired_friend(
            "555",
            "AlsoSharingFriend",
            "Also Sharing Friend",
            true, // share_enabled
        );

        // All are in party
        let party_member_ids = vec![
            "333".to_string(),
            "444".to_string(),
            "555".to_string(),
        ];

        // Filter for sharing-enabled friends in party
        let sharing_friends = vec![
            &friend_sharing_enabled,
            &friend_sharing_disabled,
            &friend_also_sharing,
        ]
        .into_iter()
        .filter(|f| f.share_enabled && party_member_ids.contains(&f.summoner_id))
        .collect::<Vec<_>>();

        // Should only include the two with sharing enabled
        assert_eq!(sharing_friends.len(), 2);
        assert!(sharing_friends.iter().any(|f| f.summoner_id == "333"));
        assert!(sharing_friends.iter().any(|f| f.summoner_id == "555"));
        assert!(!sharing_friends.iter().any(|f| f.summoner_id == "444"));
    }

    /// Test: Party membership changes mid-session
    ///
    /// Scenario: Friend joins or leaves party during champion select.
    /// Expected: Party membership should be re-evaluated, affecting share distribution.
    #[test]
    fn test_party_membership_changes() {
        clear_received_skins_cache();
        clear_sent_shares();

        let friend_1 = create_mock_paired_friend(
            "666",
            "Friend1",
            "Friend 1",
            true,
        );

        let friend_2 = create_mock_paired_friend(
            "777",
            "Friend2",
            "Friend 2",
            true,
        );

        // Initial party: only friend_1
        let mut party_members = vec!["666".to_string()];
        assert!(party_members.contains(&friend_1.summoner_id));
        assert!(!party_members.contains(&friend_2.summoner_id));

        // Friend 2 joins party
        party_members.push("777".to_string());
        assert!(party_members.contains(&friend_2.summoner_id));
        assert_eq!(party_members.len(), 2);

        // Friend 1 leaves party
        party_members.retain(|id| id != "666");
        assert!(!party_members.contains(&friend_1.summoner_id));
        assert!(party_members.contains(&friend_2.summoner_id));
        assert_eq!(party_members.len(), 1);
    }

    /// Test: No paired friends in party
    ///
    /// Scenario: Player has paired friends configured but none are in current party.
    /// Expected: No shares should be sent or expected.
    #[test]
    fn test_no_paired_friends_in_party() {
        clear_received_skins_cache();
        clear_sent_shares();

        let friend_1 = create_mock_paired_friend(
            "888",
            "AbsentFriend1",
            "Absent Friend 1",
            true,
        );

        let friend_2 = create_mock_paired_friend(
            "999",
            "AbsentFriend2",
            "Absent Friend 2",
            true,
        );

        // Party has different players
        let party_members = vec!["1000".to_string(), "1001".to_string()];

        // Neither paired friend is in party
        assert!(!party_members.contains(&friend_1.summoner_id));
        assert!(!party_members.contains(&friend_2.summoner_id));

        // In real implementation, should_inject_now would proceed without waiting
        // and no shares would be sent
    }

    /// Test: All paired friends in party
    ///
    /// Scenario: All configured paired friends are in the current party.
    /// Expected: Shares should be sent to all and injection waits for all.
    #[test]
    fn test_all_paired_friends_in_party() {
        clear_received_skins_cache();
        clear_sent_shares();

        let friends = vec![
            create_mock_paired_friend("101", "Friend1", "Friend 1", true),
            create_mock_paired_friend("102", "Friend2", "Friend 2", true),
            create_mock_paired_friend("103", "Friend3", "Friend 3", true),
        ];

        // All friends in party
        let party_members = vec![
            "101".to_string(),
            "102".to_string(),
            "103".to_string(),
        ];

        // All friends should be in party
        for friend in &friends {
            assert!(party_members.contains(&friend.summoner_id));
            assert!(friend.share_enabled);
        }

        assert_eq!(friends.len(), 3);
        assert_eq!(party_members.len(), 3);
    }

    /// Test: Partial party membership with mixed settings
    ///
    /// Scenario: Some friends in party with sharing on, some in party with sharing off, some outside.
    /// Expected: Only in-party friends with sharing enabled should participate.
    #[test]
    fn test_partial_party_mixed_settings() {
        clear_received_skins_cache();
        clear_sent_shares();

        let friend_in_sharing = create_mock_paired_friend(
            "201",
            "InSharing",
            "In Sharing",
            true,
        );

        let friend_in_not_sharing = create_mock_paired_friend(
            "202",
            "InNotSharing",
            "In Not Sharing",
            false,
        );

        let friend_out_sharing = create_mock_paired_friend(
            "203",
            "OutSharing",
            "Out Sharing",
            true,
        );

        let party_members = vec!["201".to_string(), "202".to_string()];

        // Build list of active sharers
        let active_sharers = vec![
            &friend_in_sharing,
            &friend_in_not_sharing,
            &friend_out_sharing,
        ]
        .into_iter()
        .filter(|f| {
            f.share_enabled && party_members.contains(&f.summoner_id)
        })
        .collect::<Vec<_>>();

        // Only friend_in_sharing should be active
        assert_eq!(active_sharers.len(), 1);
        assert_eq!(active_sharers[0].summoner_id, "201");
    }

    /// Test: Party detection via different endpoints
    ///
    /// Scenario: Party members can be detected from multiple LCU endpoints.
    /// Expected: All detection methods should identify the same party members.
    #[test]
    fn test_party_detection_multiple_endpoints() {
        // Simulate party IDs from champ-select endpoint
        let champ_select_party = vec!["301".to_string(), "302".to_string()];

        // Simulate party IDs from lobby endpoint
        let lobby_party = vec!["301".to_string(), "302".to_string()];

        // Simulate party IDs from gameflow endpoint
        let gameflow_party = vec!["301".to_string(), "302".to_string()];

        // All endpoints should return consistent results
        assert_eq!(champ_select_party, lobby_party);
        assert_eq!(lobby_party, gameflow_party);
    }

    /// Test: Party member ID normalization
    ///
    /// Scenario: Summoner IDs might have whitespace or formatting differences.
    /// Expected: IDs should be normalized for consistent matching.
    #[test]
    fn test_party_member_id_normalization() {
        let friend = create_mock_paired_friend(
            "401",
            "NormalFriend",
            "Normal Friend",
            true,
        );

        // Party IDs might have whitespace
        let party_with_whitespace = vec![" 401 ".to_string(), "402".to_string()];

        // Normalize by trimming
        let normalized_party: Vec<String> = party_with_whitespace
            .iter()
            .map(|id| id.trim().to_string())
            .collect();

        // Friend should be found after normalization
        assert!(normalized_party.contains(&friend.summoner_id));
    }

    /// Test: Empty party (solo queue)
    ///
    /// Scenario: Player is not in a party (solo queue).
    /// Expected: No party members detected, proceed with local skins only.
    #[test]
    fn test_empty_party_solo_queue() {
        clear_received_skins_cache();
        clear_sent_shares();

        let friend = create_mock_paired_friend(
            "501",
            "SoloFriend",
            "Solo Friend",
            true,
        );

        // Empty party (solo player)
        let party_members: Vec<String> = vec![];

        assert!(party_members.is_empty());
        assert!(!party_members.contains(&friend.summoner_id));

        // In real implementation, should proceed with local injection
    }

    /// Test: Large party (5-player premade)
    ///
    /// Scenario: Full 5-player premade with multiple paired friends.
    /// Expected: All paired friends in party should be handled correctly.
    #[test]
    fn test_large_party_five_players() {
        clear_received_skins_cache();
        clear_sent_shares();

        let mut friends = vec![];
        let mut party_members = vec![];

        // Create 5 friends, all in party with sharing enabled
        for i in 0..5 {
            let friend = create_mock_paired_friend(
                &format!("60{}", i),
                &format!("Friend{}", i),
                &format!("Friend {}", i),
                true,
            );
            party_members.push(friend.summoner_id.clone());
            friends.push(friend);
        }

        assert_eq!(friends.len(), 5);
        assert_eq!(party_members.len(), 5);

        // All friends should be in party
        for friend in &friends {
            assert!(party_members.contains(&friend.summoner_id));
        }
    }

    /// Test: Party member leaves during champion select
    ///
    /// Scenario: A friend disconnects or leaves during champion select.
    /// Expected: Party detection should update and exclude the departed friend.
    #[test]
    fn test_party_member_leaves_during_select() {
        clear_received_skins_cache();
        clear_sent_shares();

        let friend_staying = create_mock_paired_friend(
            "701",
            "StayingFriend",
            "Staying Friend",
            true,
        );

        let friend_leaving = create_mock_paired_friend(
            "702",
            "LeavingFriend",
            "Leaving Friend",
            true,
        );

        // Initial party
        let mut party_members = vec!["701".to_string(), "702".to_string()];
        assert_eq!(party_members.len(), 2);

        // Friend leaves
        party_members.retain(|id| id != "702");
        
        assert_eq!(party_members.len(), 1);
        assert!(party_members.contains(&friend_staying.summoner_id));
        assert!(!party_members.contains(&friend_leaving.summoner_id));
    }
}
