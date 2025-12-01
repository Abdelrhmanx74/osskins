// Party mode module - handles skin sharing with friends

pub mod commands;
pub mod handlers;
pub mod lcu;
pub mod messaging;
pub mod party_detection;
pub mod session;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

// Re-export public types and functions
pub use types::{PARTY_MODE_VERBOSE, RECEIVED_SKINS};

pub use utils::clear_sent_shares;

pub use session::clear_received_skins;

pub use commands::{
  add_party_friend, get_lcu_friends, get_paired_friends, get_party_mode_settings,
  get_party_mode_verbose_logging, remove_paired_friend, set_party_mode_max_share_age,
  set_party_mode_verbose_logging, update_party_mode_settings,
};

pub use handlers::{
  handle_party_mode_message, send_skin_share_to_paired_friends, should_inject_now,
};
