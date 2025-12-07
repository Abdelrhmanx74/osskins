// Types and static variables for party mode

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct InMemoryReceivedSkin {
  pub from_summoner_id: String,
  pub from_summoner_name: String,
  pub champion_id: u32,
  pub skin_id: u32,
  pub chroma_id: Option<u32>,
  pub skin_file_path: Option<String>,
  pub received_at: u64,
}

// Global in-memory map for received skins (key: summoner+champion)
pub static RECEIVED_SKINS: Lazy<Mutex<HashMap<String, InMemoryReceivedSkin>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

pub static CURRENT_SESSION_ID: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub static PARTY_MODE_VERBOSE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Outbound share de-duplication within a phase: track one send per friend per champion
pub static SENT_SKIN_SHARES: Lazy<Mutex<std::collections::HashSet<String>>> =
  Lazy::new(|| Mutex::new(std::collections::HashSet::new()));

pub const PARTY_MODE_MESSAGE_PREFIX: &str = "OSS:";
pub const MAX_SHARE_AGE_SECS: u64 = 300;

// Verbose logging macro - only logs when verbose mode is enabled
#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => ({
        if $crate::commands::party_mode::types::PARTY_MODE_VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*);
        }
    })
}

// Normal logging macro - always logs
#[macro_export]
macro_rules! normal_log {
    ($($arg:tt)*) => ({ println!($($arg)*); })
}

pub struct LcuConnection {
  pub port: String,
  pub token: String,
}

pub struct CurrentSummoner {
  pub summoner_id: String,
  pub display_name: String,
}

// ============================================================================
// Session validation helpers
// ============================================================================

/// Check if a message timestamp is valid for the current champ select session.
/// Uses the session tracking from lcu_watcher::types.
pub fn is_message_from_current_session(message_timestamp_ms: u64) -> bool {
  use crate::commands::lcu_watcher::types::is_message_timestamp_valid;
  let (valid, reason) = is_message_timestamp_valid(message_timestamp_ms);

  if !valid {
    if PARTY_MODE_VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
      println!(
        "[Party Mode][SESSION] Rejecting message with timestamp {} - {}",
        message_timestamp_ms, reason
      );
    }
  }

  valid
}

/// Log helper that respects verbose mode and adds consistent prefix
pub fn log_verbose(message: &str) {
  if PARTY_MODE_VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
    println!("[Party Mode][VERBOSE] {}", message);
  }
}

/// Log helper for important messages (always logged)
pub fn log_info(message: &str) {
  println!("[Party Mode] {}", message);
}

/// Log helper for warnings (always logged)
pub fn log_warn(message: &str) {
  println!("[Party Mode][WARN] {}", message);
}

/// Log helper for errors (always logged)
pub fn log_error(message: &str) {
  eprintln!("[Party Mode][ERROR] {}", message);
}

/// Log helper for debug messages (only in verbose mode)
pub fn log_debug(message: &str) {
  if PARTY_MODE_VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
    println!("[Party Mode][DEBUG] {}", message);
  }
}

/// Get a formatted string of the current session state for debugging
#[allow(dead_code)]
pub fn debug_session_state() -> String {
  use crate::commands::lcu_watcher::types::{
    current_time_ms, CHAMP_SELECT_SESSION_COUNTER, CHAMP_SELECT_START_TIME_MS,
    LAST_SHARED_CHAMPION_ID,
  };
  use std::sync::atomic::Ordering;

  let session_start = CHAMP_SELECT_START_TIME_MS.load(Ordering::SeqCst);
  let session_counter = CHAMP_SELECT_SESSION_COUNTER.load(Ordering::SeqCst);
  let last_shared_champ = LAST_SHARED_CHAMPION_ID.load(Ordering::SeqCst);
  let now = current_time_ms();
  let elapsed_secs = if session_start > 0 {
    (now - session_start) / 1000
  } else {
    0
  };

  format!(
    "session #{}, started {}s ago, last_shared_champ={}",
    session_counter, elapsed_secs, last_shared_champ
  )
}
