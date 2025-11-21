# Party Mode Test Suite

This directory contains comprehensive unit tests for the party mode feature backend. The test suite validates various scenarios including timing edge cases, game mode specifics (ARAM, Swift Play), and race conditions.

## Test Structure

The test suite is organized into the following modules:

### 1. `test_timing_edge_cases.rs`
Tests for timing-sensitive scenarios:
- Friend sends request while lobby timing is ending
- Both friends fail to select in time
- Last-second champion lock before phase transition
- Message arrival after phase change
- Race between skin share and champion lock
- Stale share pruning based on MAX_SHARE_AGE_SECS
- Multiple timing scenarios in sequence

### 2. `test_aram_mode.rs`
Tests specific to ARAM mode:
- Champion re-roll triggers re-sharing
- Champion swap between friends
- Multiple re-rolls in sequence
- Re-share after champion trade
- Partial share injection (50% threshold)
- Normal ARAM flow without re-rolls
- Re-roll with same champion (edge case)

### 3. `test_swift_play.rs`
Tests for Swift Play (instant-assign) mode:
- Two skins shared before matchmaking phase
- Multiple champions selection and sharing
- Champion selection change mid-phase
- 50% threshold injection logic
- Below 50% threshold behavior
- All friends shared scenario
- Single champion choice
- Champion priority assignment
- Delayed shares accumulation

### 4. `test_session_state.rs`
Tests for session tracking and state management:
- Session ID change clears received skins
- Stale skin share pruning
- Deduplication of sent shares
- Phase transition state reset
- Multiple shares from same friend for different champions
- Overwriting skin share for same champion
- Sent shares cleared on new phase
- State persistence during champion lock changes
- MAX_SHARE_AGE_SECS boundary condition
- Concurrent state access

### 5. `test_party_detection.rs`
Tests for party member detection and filtering:
- Friend in party vs outside party filtering
- Multiple friends with different share settings
- Party membership changes mid-session
- No paired friends in party
- All paired friends in party
- Partial party membership with mixed settings
- Party detection via different endpoints
- Party member ID normalization
- Empty party (solo queue)
- Large party (5-player premade)
- Party member leaves during champion select

### 6. `test_injection_logic.rs`
Tests for injection logic and skin file resolution:
- should_inject_now with all friends ready
- should_inject_now with partial readiness
- Skin file path resolution (absolute and relative paths)
- Local + friend skin injection batch
- Custom skin handling
- No champion locked scenario
- Skin file path with chroma
- Multiple skins for same champion
- Missing skin file path
- Injection deduplication
- Fallback to local skin
- Injection with misc items

### 7. `test_race_conditions.rs`
Tests for concurrent operations and race conditions:
- Concurrent skin shares from multiple friends
- Message processing during phase transition
- Injection trigger race with watcher loop
- Concurrent sent share deduplication
- Phase state and cache consistency
- Rapid champion lock/unlock cycles
- Session ID check race with skin receipt
- Concurrent cache reads and writes
- Injection already in progress flag
- Party membership query during share processing
- Watcher polling interval race
- Duplicate message ID handling

### 8. `test_helpers.rs`
Utility functions and mock data creation:
- Mock LCU connection
- Mock skin share creation
- Mock paired friend creation
- Cache manipulation helpers
- Timestamp utilities
- Self-tests for helper functions

## Running the Tests

### Run all party mode tests:
```bash
cd src-tauri
cargo test party_mode::tests
```

### Run a specific test module:
```bash
cargo test party_mode::tests::timing_tests
cargo test party_mode::tests::aram_tests
cargo test party_mode::tests::swift_play_tests
cargo test party_mode::tests::session_state_tests
cargo test party_mode::tests::party_detection_tests
cargo test party_mode::tests::injection_logic_tests
cargo test party_mode::tests::race_condition_tests
```

### Run a specific test:
```bash
cargo test test_friend_shares_at_lobby_end
cargo test test_champion_reroll_triggers_reshare
cargo test test_concurrent_skin_shares
```

### Run tests with output:
```bash
cargo test party_mode::tests -- --show-output
```

### Run tests in verbose mode:
```bash
cargo test party_mode::tests -- --nocapture
```

## Test Coverage

The test suite covers the following key scenarios mentioned in the requirements:

1. **Friend sends request while lobby timing is ending**
   - `test_friend_shares_at_lobby_end`
   - `test_both_fail_to_select_in_time`
   - `test_last_second_champion_lock`

2. **ARAM champion re-roll and skin re-sharing**
   - `test_champion_reroll_triggers_reshare`
   - `test_multiple_rerolls_in_sequence`
   - `test_reshare_after_champion_trade`
   - `test_champion_swap_between_friends`

3. **Swift Play two skins before matchmaking**
   - `test_two_skins_shared_before_matchmaking`
   - `test_multiple_champions_multiple_friends`
   - `test_swift_play_50_percent_threshold`

## Implementation Notes

### State Management
The tests verify that party mode state (received skins, sent shares) is properly managed across phase transitions and session changes. The global state is accessed through thread-safe Mutex-protected structures.

### Timing Considerations
Many tests simulate timing-sensitive scenarios by using timestamps that are seconds or milliseconds apart. The MAX_SHARE_AGE_SECS constant (300 seconds = 5 minutes) is used to determine stale shares.

### Thread Safety
The RECEIVED_SKINS and SENT_SKIN_SHARES global maps are protected by Mutex to ensure thread-safe concurrent access. Tests verify this behavior with concurrent operation tests.

### Cache Behavior
- Received skins are keyed by `"<summoner_id>_<champion_id>"`
- Sent shares are keyed by `"<friend_id>:<champion_id>:<skin_id>:<chroma_id>"`
- Cache is cleared on session change and phase transitions

## Known Limitations

1. **System Dependencies**: Full test execution requires system libraries (glib, gobject) for Tauri. Tests can be validated for syntax/logic without full build.

2. **Mock Data**: Tests use simplified mock data structures. Real implementation uses full Tauri AppHandle and LCU connection objects.

3. **Integration Testing**: These are primarily unit tests. Full integration testing would require a mock LCU server or actual League of Legends client.

## Future Enhancements

Potential areas for additional test coverage:
- Network failure scenarios
- LCU connection drops during skin sharing
- Invalid JSON in party mode messages
- Malformed skin file paths
- Corrupted skin files
- Memory pressure scenarios with large friend lists
- Performance tests for high-frequency share operations

## Contributing

When adding new party mode features, please:
1. Add corresponding test cases in the appropriate test module
2. Update this README with new test descriptions
3. Ensure all existing tests still pass
4. Add integration tests for complex multi-component interactions
