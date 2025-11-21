# Party Mode Test Case Analysis

## Overview
This document provides a detailed analysis of how the test cases relate to the actual party mode implementation and what scenarios they validate.

## Test Case Mapping to Implementation

### 1. Timing Edge Cases

#### Test: `test_friend_shares_at_lobby_end`
**Implementation Reference**: `handlers.rs::handle_party_mode_message()`
- **What it tests**: When a friend shares their skin just as champion select timer is about to expire
- **Real behavior**: The share is stored in `RECEIVED_SKINS` global cache with a timestamp
- **Expected outcome**: Share is cached but injection may not complete if champion isn't locked in time
- **Code path**: Message arrives → parsed → timestamp validated against `MAX_SHARE_AGE_SECS` → stored in cache

#### Test: `test_both_fail_to_select_in_time`
**Implementation Reference**: `handlers.rs::should_inject_now()`, `injection.rs::trigger_party_mode_injection()`
- **What it tests**: Neither player locks champion before timer expires
- **Real behavior**: `should_inject_now()` checks if `champion_id == 0`, returns false
- **Expected outcome**: No injection occurs, shared skins remain in cache for potential next phase
- **Code path**: Check champion_id → if 0, return false → injection not triggered

#### Test: `test_stale_share_pruning`
**Implementation Reference**: `session.rs::prune_stale_received_skins()`, `types.rs::MAX_SHARE_AGE_SECS`
- **What it tests**: Shares older than 300 seconds are removed
- **Real behavior**: Pruning function compares current time with `received_at` timestamp
- **Expected outcome**: Old shares removed, recent ones kept
- **Code path**: Iterate RECEIVED_SKINS → calculate age → remove if age > MAX_SHARE_AGE_SECS

### 2. ARAM Mode Tests

#### Test: `test_champion_reroll_triggers_reshare`
**Implementation Reference**: `session.rs::get_selected_champion_id()`, `handlers.rs::send_skin_share_to_paired_friends()`
- **What it tests**: Friend re-rolls in ARAM and gets new champion
- **Real behavior**: New champion detected → new share sent → stored with different key `"summoner_champion"`
- **Expected outcome**: Both old and new champion shares exist in cache (different champion IDs)
- **Code path**: Detect re-roll → new champion_id → send_skin_share → cache with new key

#### Test: `test_aram_partial_share_injection`
**Implementation Reference**: `handlers.rs::should_inject_now()` lines 560-569
- **What it tests**: In ARAM, injection should proceed with partial shares (>0)
- **Real behavior**: Special ARAM logic: `if is_aram && shared > 0 { return true }`
- **Expected outcome**: Injection triggers even if not all friends have shared
- **Code path**: Check game mode → if ARAM && shared_count > 0 → inject early

### 3. Swift Play Mode Tests

#### Test: `test_swift_play_50_percent_threshold`
**Implementation Reference**: `handlers.rs::should_inject_now()` lines 571-578
- **What it tests**: Swift Play injects at 50% friend readiness
- **Real behavior**: Check `is_swift && shared * 2 >= total`
- **Expected outcome**: Injection proceeds when half or more friends have shared
- **Code path**: Detect Swift Play → calculate percentage → if >= 50% → inject

#### Test: `test_two_skins_shared_before_matchmaking`
**Implementation Reference**: `injection.rs::trigger_party_mode_injection_for_champions()`
- **What it tests**: Player selects 2 champions, friends share for both
- **Real behavior**: Multi-champion injection handles array of champion_ids
- **Expected outcome**: All skins for both champions collected and injected together
- **Code path**: Get champion_ids array → collect skins for each → batch inject

### 4. Session & State Management Tests

#### Test: `test_session_change_clears_skins`
**Implementation Reference**: `session.rs::refresh_session_tracker()`, `session.rs::clear_received_skins()`
- **What it tests**: New game session invalidates previous shares
- **Real behavior**: Session ID tracked via `CURRENT_SESSION_ID`, change triggers clear
- **Expected outcome**: Cache cleared on session change
- **Code path**: Fetch gameflow session → compare session_id → if changed → clear cache

#### Test: `test_sent_shares_deduplication`
**Implementation Reference**: `types.rs::SENT_SKIN_SHARES`, `handlers.rs::send_skin_share_to_paired_friends()` lines 302-312
- **What it tests**: Same skin not sent twice to same friend in one phase
- **Real behavior**: HashSet tracks `"friend:champion:skin:chroma"` keys
- **Expected outcome**: Second send attempt blocked
- **Code path**: Generate key → check if in SENT_SKIN_SHARES → if exists, skip → else add and send

### 5. Party Detection Tests

#### Test: `test_friend_in_party_vs_outside`
**Implementation Reference**: `party_detection.rs::get_current_party_member_summoner_ids()`, `handlers.rs::send_skin_share_to_paired_friends()` lines 247-272
- **What it tests**: Only friends actually in party receive shares
- **Real behavior**: Fetches party member IDs from LCU, filters paired friends
- **Expected outcome**: Shares sent only to in-party friends
- **Code path**: Get party IDs → filter paired_friends by membership → send only to filtered list

#### Test: `test_party_membership_changes`
**Implementation Reference**: `party_detection.rs::get_current_party_member_summoner_ids()`
- **What it tests**: Party composition can change during champion select
- **Real behavior**: Party membership re-queried on each share/injection check
- **Expected outcome**: Reflects current party state, not stale data
- **Code path**: Real-time LCU query → returns current members → used for filtering

### 6. Injection Logic Tests

#### Test: `test_skin_file_path_relative`
**Implementation Reference**: `injection.rs::trigger_party_mode_injection()` lines 196-374
- **What it tests**: Relative paths like "ezrea/skin.zip" are resolved
- **Real behavior**: Complex resolution logic tries multiple locations
- **Expected outcome**: Skin file found in champions directory
- **Code path**: Check if absolute → map portable prefixes → try variants in dirs → shallow scan → fallback

#### Test: `test_local_and_friend_skin_batch`
**Implementation Reference**: `injection.rs::trigger_party_mode_injection()` lines 167-189, 520-534
- **What it tests**: Local skin + friend skins injected together
- **Real behavior**: Collect local skin from config, collect friend skins from cache, deduplicate, inject all
- **Expected outcome**: Single injection call with all skins
- **Code path**: Add local skin → iterate RECEIVED_SKINS → deduplicate → inject_skins_and_misc_no_events

### 7. Race Condition Tests

#### Test: `test_concurrent_skin_shares`
**Implementation Reference**: `types.rs::RECEIVED_SKINS` (Mutex-protected)
- **What it tests**: Multiple friends sharing simultaneously
- **Real behavior**: Mutex on RECEIVED_SKINS ensures thread-safe access
- **Expected outcome**: All shares stored correctly without data race
- **Code path**: Lock mutex → insert share → unlock → repeat for each share

#### Test: `test_injection_trigger_race`
**Implementation Reference**: `watcher.rs` polling loop, `injection.rs::trigger_party_mode_injection()`
- **What it tests**: Champion lock happens during watcher check
- **Real behavior**: Watcher polls every 1500ms, may miss exact moment
- **Expected outcome**: Injection triggers on next poll cycle
- **Code path**: Watcher checks conditions → champion_id updated → next poll → injection triggered

## How Tests Validate Real Implementation

### 1. State Management Validation
Tests verify that global state (`RECEIVED_SKINS`, `SENT_SKIN_SHARES`) is:
- Thread-safe (Mutex-protected)
- Correctly cleared on phase/session transitions
- Properly deduplicated

### 2. Timing Logic Validation
Tests confirm that:
- Timestamp-based age checking works correctly
- Stale shares are pruned
- Late-arriving messages are handled appropriately

### 3. Game Mode Logic Validation
Tests ensure:
- ARAM's early injection (shared > 0) works
- Swift Play's 50% threshold is calculated correctly
- Normal mode waits for all friends

### 4. Party Filtering Validation
Tests verify:
- Only in-party friends receive shares
- Party membership is accurately detected
- Changes to party composition are reflected

### 5. Injection Path Validation
Tests confirm:
- File path resolution tries multiple strategies
- Fallback mechanisms work
- Deduplication prevents duplicate injections
- Batch injection includes all skins

## Coverage Analysis

### Covered Scenarios
✅ Timing edge cases (friend sends at last second)
✅ Both players fail to lock (no injection)
✅ ARAM re-rolls and re-sharing
✅ Swift Play multi-champion sharing
✅ Session changes clearing state
✅ Stale share pruning
✅ Party membership filtering
✅ Concurrent operations (thread safety)
✅ Skin file path resolution
✅ Injection deduplication

### Real-World Scenarios Not Covered by Unit Tests
(Would require integration tests or mock LCU server)
- Actual LCU websocket events
- Real network failures
- Invalid JSON parsing errors
- File I/O errors during injection
- Memory pressure with large friend lists
- Actual League client crashes during injection

## Running Tests Against Implementation

The tests validate logic but don't execute the full Tauri application. To verify against real implementation:

1. **Unit Test Level** (current): Tests validate logic using global state
2. **Integration Test Level** (future): Would need mock LCU server
3. **E2E Test Level** (future): Would need actual League client

## Maintenance Notes

When modifying party mode code:
1. Update corresponding tests if behavior changes
2. Add new tests for new edge cases
3. Ensure thread safety is maintained
4. Verify timing constants (MAX_SHARE_AGE_SECS) are still appropriate
5. Test with actual League client for full validation

## Test Execution Results

To see which tests pass:
```bash
cargo test party_mode::tests -- --nocapture
```

Expected output will show all test cases and their pass/fail status.
