// LCU Watcher module - monitors League Client and handles skin injection

mod injection;
mod logging;
mod party_mode;
mod session;
pub mod types;
mod utils;
mod watcher;

// Re-export public types and functions
pub use logging::{append_global_log, print_logs};
pub use party_mode::start_party_mode_chat_monitor;
pub use utils::is_in_champ_select;
pub use watcher::start_lcu_watcher;
