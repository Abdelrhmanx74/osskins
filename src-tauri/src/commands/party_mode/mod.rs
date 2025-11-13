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
  InMemoryReceivedSkin, 
  RECEIVED_SKINS, 
  CURRENT_SESSION_ID,
  PARTY_MODE_VERBOSE,
  SENT_SKIN_SHARES,
  PARTY_MODE_MESSAGE_PREFIX,
  MAX_SHARE_AGE_SECS,
  LcuConnection,
  CurrentSummoner,
};

pub use utils::{
  received_skin_key,
  sent_share_key,
  clear_sent_shares,
  get_configured_max_share_age_secs,
  get_skin_name_from_config,
};

pub use lcu::{
  get_lcu_connection,
  get_current_summoner,
  get_friends_with_connection,
  get_friend_display_name,
  get_conversation_id,
};

pub use party_detection::{
  get_current_party_member_summoner_ids,
  get_gameflow_party_member_summoner_ids,
  collect_ids_from_json,
};

pub use session::{
  refresh_session_tracker,
  prune_stale_received_skins,
  clear_received_skins,
};

pub use messaging::send_chat_message;

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
