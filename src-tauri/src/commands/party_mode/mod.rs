// Party mode module - handles skin sharing with friends

pub mod types;
pub mod utils;
pub mod lcu;
pub mod party_detection;
pub mod session;
pub mod messaging;
pub mod commands;
pub mod handlers;

// Re-export public types and functions
pub use types::{
  RECEIVED_SKINS, 
  PARTY_MODE_VERBOSE,
};

pub use utils::clear_sent_shares;

pub use session::clear_received_skins;

pub use commands::{
  get_lcu_friends,
  add_party_friend,
  remove_paired_friend,
  set_party_mode_verbose_logging,
  get_party_mode_verbose_logging,
  set_party_mode_max_share_age,
  get_paired_friends,
  get_party_mode_settings,
  update_party_mode_settings,
};

pub use handlers::{
  handle_party_mode_message,
  send_skin_share_to_paired_friends,
  should_inject_now,
};
