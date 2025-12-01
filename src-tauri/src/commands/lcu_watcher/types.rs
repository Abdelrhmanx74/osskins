// Types and constants for LCU watcher

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicU8};

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

// Hard gate: inject at most once per ChampSelect phase (prevents thrash when champion_id flips)
pub static PARTY_INJECTION_DONE_THIS_PHASE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Track last champion share time to debounce rapid ARAM rerolls
pub static LAST_CHAMPION_SHARE_TIME: Lazy<
  std::sync::Mutex<std::collections::HashMap<u32, std::time::Instant>>,
> = Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
