// Types and static variables for party mode

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
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

// Verbose logging macro
#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => ({
        if $crate::commands::party_mode::types::PARTY_MODE_VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*);
        }
    })
}

// Normal logging macro
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
