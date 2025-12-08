// Types and constants for LCU watcher

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8};
use std::time::{SystemTime, UNIX_EPOCH};

// Injection mode selection – stored in config.json under "injection_mode"
#[derive(PartialEq, Eq)]
pub enum InjectionMode {
  ChampSelect,
  Lobby,
}

// 0 = Unknown, 1 = ChampSelect, 2 = Other
pub static PHASE_STATE: Lazy<AtomicU8> = Lazy::new(|| AtomicU8::new(0));

// Prevent repeated injections in the same ChampSelect phase
pub static LAST_PARTY_INJECTION_SIGNATURE: Lazy<std::sync::Mutex<Option<String>>> =
  Lazy::new(|| std::sync::Mutex::new(None));

// Tracks the last champion set used for instant-assign multi-champion injection (Lobby->Matchmaking)
pub static LAST_INSTANT_ASSIGN_CHAMPIONS: Lazy<std::sync::Mutex<Vec<u32>>> =
  Lazy::new(|| std::sync::Mutex::new(Vec::new()));

// Hard gate: inject at most once per ChampSelect phase (prevents thrash when champion_id flips)
pub static PARTY_INJECTION_DONE_THIS_PHASE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Track last champion share time to debounce rapid ARAM rerolls
pub static LAST_CHAMPION_SHARE_TIME: Lazy<
  std::sync::Mutex<std::collections::HashMap<u32, std::time::Instant>>,
> = Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

// ============================================================================
// Session tracking for party mode message filtering
// ============================================================================

/// Timestamp (milliseconds since UNIX epoch) when current champ select session started.
/// Messages with timestamps BEFORE this value should be ignored as stale.
pub static CHAMP_SELECT_START_TIME_MS: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

/// Counter that increments each time we enter ChampSelect. Used to detect session changes.
pub static CHAMP_SELECT_SESSION_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

/// Flag to prevent multiple LCU watcher threads from running simultaneously.
/// Only one watcher should be active at a time.
pub static LCU_WATCHER_ACTIVE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Unique ID for the current watcher instance (random value set on start).
/// Used to detect if a newer watcher has started and this one should exit.
pub static LCU_WATCHER_INSTANCE_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

/// Track the last champion ID that was shared to detect rerolls/swaps in ARAM/URF
pub static LAST_SHARED_CHAMPION_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

/// Track when we last triggered a re-injection due to champion change
pub static LAST_REINJECTION_TIME_MS: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

// ============================================================================
// Helper functions for session tracking
// ============================================================================

/// Get current time in milliseconds since UNIX epoch
pub fn current_time_ms() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64
}

/// Mark the start of a new champ select session. Call this when entering ChampSelect phase.
pub fn start_new_champ_select_session() {
  use std::sync::atomic::Ordering;

  let now = current_time_ms();
  CHAMP_SELECT_START_TIME_MS.store(now, Ordering::SeqCst);
  CHAMP_SELECT_SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
  LAST_SHARED_CHAMPION_ID.store(0, Ordering::SeqCst);
  LAST_REINJECTION_TIME_MS.store(0, Ordering::SeqCst);

  println!(
    "[LCU Watcher][SESSION] New champ select session started at {} (session #{})",
    now,
    CHAMP_SELECT_SESSION_COUNTER.load(Ordering::SeqCst)
  );
}

/// Check if a message timestamp is valid for the current session.
/// Returns (is_valid, reason) for logging purposes.
pub fn is_message_timestamp_valid(message_timestamp_ms: u64) -> (bool, &'static str) {
  use std::sync::atomic::Ordering;

  let session_start = CHAMP_SELECT_START_TIME_MS.load(Ordering::SeqCst);

  // If no session has started yet, accept all messages (shouldn't happen normally)
  if session_start == 0 {
    return (true, "no session tracked yet");
  }

  // Message is from before this champ select session started
  if message_timestamp_ms < session_start {
    return (false, "message predates current session");
  }

  // Message is from the future (clock skew) - allow with small tolerance (10 seconds)
  let now = current_time_ms();
  if message_timestamp_ms > now + 10_000 {
    return (false, "message timestamp is in the future");
  }

  (true, "timestamp valid")
}

/// Check if enough time has passed since last re-injection to allow another one.
/// This prevents rapid re-injections on fast champion changes (e.g., spam rerolling).
#[allow(dead_code)]
pub fn can_reinjection_happen(cooldown_ms: u64) -> bool {
  use std::sync::atomic::Ordering;

  let last_reinjection = LAST_REINJECTION_TIME_MS.load(Ordering::SeqCst);
  let now = current_time_ms();

  if last_reinjection == 0 {
    return true;
  }

  now.saturating_sub(last_reinjection) >= cooldown_ms
}

/// Mark that a re-injection just happened
#[allow(dead_code)]
pub fn mark_reinjection_time() {
  use std::sync::atomic::Ordering;
  LAST_REINJECTION_TIME_MS.store(current_time_ms(), Ordering::SeqCst);
}

/// Generate a new unique watcher instance ID
pub fn generate_watcher_instance_id() -> u64 {
  use std::sync::atomic::Ordering;

  let id = current_time_ms() ^ (std::process::id() as u64);
  LCU_WATCHER_INSTANCE_ID.store(id, Ordering::SeqCst);
  id
}

/// Check if this watcher instance is still the current one
pub fn is_current_watcher_instance(my_id: u64) -> bool {
  use std::sync::atomic::Ordering;
  LCU_WATCHER_INSTANCE_ID.load(Ordering::SeqCst) == my_id
}
