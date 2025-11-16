# Party Mode Test Suite - Visual Overview

## Test Organization Structure

```
src-tauri/src/commands/party_mode/tests/
│
├── mod.rs (Module declarations)
│
├── test_helpers.rs (8 tests)
│   ├── Mock data creation
│   ├── Cache manipulation helpers
│   ├── Timestamp utilities
│   └── Self-validation tests
│
├── test_timing_edge_cases.rs (7 tests)
│   ├── Friend shares at lobby end
│   ├── Both fail to select
│   ├── Last-second lock
│   ├── Message after phase change
│   ├── Share/lock race
│   ├── Stale share pruning
│   └── Multiple timing scenarios
│
├── test_aram_mode.rs (7 tests)
│   ├── Re-roll triggers reshare
│   ├── Champion swap
│   ├── Multiple re-rolls
│   ├── Reshare after trade
│   ├── Partial share injection
│   ├── Normal flow
│   └── Re-roll same champion
│
├── test_swift_play.rs (10 tests)
│   ├── Two skins before matchmaking
│   ├── Multiple champions/friends
│   ├── Selection change mid-phase
│   ├── 50% threshold trigger
│   ├── Below threshold
│   ├── All friends shared
│   ├── Single champion
│   ├── Priority assignment
│   ├── Delayed shares
│   └── [Additional Swift Play tests]
│
├── test_session_state.rs (10 tests)
│   ├── Session change clears
│   ├── Stale pruning
│   ├── Sent share deduplication
│   ├── Phase transition reset
│   ├── Multiple shares per friend
│   ├── Overwrite same champion
│   ├── State persistence
│   ├── Boundary conditions
│   ├── Concurrent access
│   └── [Additional state tests]
│
├── test_party_detection.rs (12 tests)
│   ├── In-party vs outside filtering
│   ├── Mixed share settings
│   ├── Membership changes
│   ├── No paired friends
│   ├── All friends in party
│   ├── Partial membership
│   ├── Multiple endpoints
│   ├── ID normalization
│   ├── Solo queue
│   ├── Large party (5 players)
│   ├── Member leaves
│   └── [Additional party tests]
│
├── test_injection_logic.rs (14 tests)
│   ├── should_inject_now (all ready)
│   ├── should_inject_now (partial)
│   ├── Absolute path resolution
│   ├── Relative path resolution
│   ├── Local + friend batch
│   ├── Custom skin handling
│   ├── No champion locked
│   ├── Skin with chroma
│   ├── Multiple skins same champion
│   ├── Missing file path
│   ├── Injection deduplication
│   ├── Fallback to local
│   ├── Injection with misc items
│   └── [Additional injection tests]
│
└── test_race_conditions.rs (12 tests)
    ├── Concurrent skin shares
    ├── Message during phase transition
    ├── Injection trigger race
    ├── Concurrent deduplication
    ├── Phase state consistency
    ├── Rapid lock/unlock
    ├── Session ID race
    ├── Concurrent cache ops
    ├── Injection in progress
    ├── Party query during share
    ├── Polling interval race
    └── Duplicate message ID
```

## Test Flow Diagram

```
                    [Champion Select Starts]
                             |
                             v
          ┌──────────────────────────────────────┐
          │  Phase State: ChampSelect            │
          │  - Clear SENT_SKIN_SHARES            │
          │  - Clear RECEIVED_SKINS (new session)│
          └──────────────────────────────────────┘
                             |
                             v
          ┌─────────────────────────────────────────────┐
          │  Friend Shares Skin                         │
          │  - Message arrives via LCU chat             │
          │  - Parse & validate timestamp               │
          │  - Check age < MAX_SHARE_AGE_SECS (300s)   │
          │  - Store in RECEIVED_SKINS                  │
          └─────────────────────────────────────────────┘
                             |
                             v
          ┌─────────────────────────────────────────────┐
          │  Watcher Loop (every 1.5s)                  │
          │  - Check champion locked                    │
          │  - Check party membership                   │
          │  - Call should_inject_now()                 │
          └─────────────────────────────────────────────┘
                             |
                             v
          ┌─────────────────────────────────────────────┐
          │  should_inject_now() Decision                │
          ├─────────────────────────────────────────────┤
          │  IF champion_id == 0: NO                    │
          │  ELSE IF no paired friends: YES (local only)│
          │  ELSE IF ARAM && shared > 0: YES (early)    │
          │  ELSE IF Swift && shared*2 >= total: YES    │
          │  ELSE IF shared == total: YES               │
          │  ELSE: NO (wait for more)                   │
          └─────────────────────────────────────────────┘
                             |
                             v
          ┌─────────────────────────────────────────────┐
          │  Injection Triggered                         │
          │  - Collect local skin from config           │
          │  - Collect friend skins from RECEIVED_SKINS │
          │  - Resolve skin file paths (6 strategies)   │
          │  - Deduplicate by (champ,skin,chroma,file)  │
          │  - Batch inject all skins                   │
          └─────────────────────────────────────────────┘
                             |
                             v
          ┌─────────────────────────────────────────────┐
          │  Phase Transition                            │
          │  - Clear RECEIVED_SKINS                      │
          │  - Clear SENT_SKIN_SHARES                    │
          │  - Reset injection flags                     │
          └─────────────────────────────────────────────┘
```

## Test Coverage Map

```
┌─────────────────────────────────────────────────────────────────┐
│                    PARTY MODE BACKEND                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐         ┌──────────────────┐             │
│  │  Timing Tests    │────────▶│  Session Tests   │             │
│  │  (Edge cases)    │         │  (State mgmt)    │             │
│  └──────────────────┘         └──────────────────┘             │
│          │                              │                        │
│          │                              │                        │
│          v                              v                        │
│  ┌──────────────────┐         ┌──────────────────┐             │
│  │  ARAM Tests      │         │  Swift Play Tests│             │
│  │  (Re-rolls)      │         │  (Multi-champ)   │             │
│  └──────────────────┘         └──────────────────┘             │
│          │                              │                        │
│          └──────────────┬───────────────┘                        │
│                         │                                        │
│                         v                                        │
│  ┌─────────────────────────────────────────────┐               │
│  │          Party Detection Tests               │               │
│  │       (Membership & Filtering)               │               │
│  └─────────────────────────────────────────────┘               │
│                         │                                        │
│          ┌──────────────┴───────────────┐                       │
│          │                              │                        │
│          v                              v                        │
│  ┌──────────────────┐         ┌──────────────────┐             │
│  │  Injection Tests │         │  Race Condition  │             │
│  │  (Logic & Path)  │         │  Tests (Threads) │             │
│  └──────────────────┘         └──────────────────┘             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Test Execution Priority

For debugging issues, run tests in this order:

1. **test_helpers** - Ensure test infrastructure works
2. **test_session_state** - Verify state management basics
3. **test_timing_edge_cases** - Check timing logic
4. **test_party_detection** - Verify party filtering
5. **test_injection_logic** - Test injection mechanics
6. **test_aram_mode** - ARAM-specific behavior
7. **test_swift_play** - Swift Play behavior
8. **test_race_conditions** - Concurrency issues (if any)

## Quick Test Commands

```bash
# Run all tests with output
cargo test party_mode::tests -- --nocapture

# Run specific category
cargo test timing_tests
cargo test aram_tests
cargo test swift_play_tests
cargo test session_state_tests
cargo test party_detection_tests
cargo test injection_logic_tests
cargo test race_condition_tests

# Run specific test
cargo test test_friend_shares_at_lobby_end -- --nocapture

# Run with multiple threads (faster)
cargo test party_mode::tests -- --test-threads=4

# Run with single thread (for debugging)
cargo test party_mode::tests -- --test-threads=1
```

## Coverage Summary

```
Total Tests: 80+
├── Timing Edge Cases: 7
├── ARAM Mode: 7
├── Swift Play: 10
├── Session/State: 10
├── Party Detection: 12
├── Injection Logic: 14
├── Race Conditions: 12
└── Test Helpers: 8
```

## Test Success Criteria

✅ All tests pass without panics
✅ Thread safety verified (concurrent tests)
✅ State isolation confirmed (no test interference)
✅ Edge cases handled correctly
✅ Documentation matches implementation
✅ Code compiles without warnings
