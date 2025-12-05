use crate::injection::Skin;
use serde::{Deserialize, Serialize};

// Data structures for various operations

#[derive(Debug, Serialize, Deserialize)]
pub struct DataUpdateProgress {
  pub current_champion: String,
  pub total_champions: usize,
  pub processed_champions: usize,
  pub status: String,
  pub progress: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataUpdateResult {
  pub success: bool,
  pub error: Option<String>,
  #[serde(default)]
  pub updated_champions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinInjectionRequest {
  pub league_path: String,
  pub skins: Vec<Skin>,
}

// Add a new structure to match the JSON data for skins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinData {
  pub champion_id: u32,
  pub skin_id: u32,
  pub chroma_id: Option<u32>,
  pub skin_file: Option<String>, // Add skin_file path from the JSON
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSkinData {
  pub id: String,
  pub name: String,
  pub champion_id: u32,
  pub champion_name: String,
  pub file_path: String,
  pub created_at: u64,
  pub preview_image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemePreferences {
  pub tone: Option<String>,
  pub is_dark: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedConfig {
  pub league_path: Option<String>,
  pub skins: Vec<SkinData>,
  #[serde(default)]
  pub custom_skins: Vec<CustomSkinData>,
  pub favorites: Vec<u32>,
  #[serde(default)]
  pub theme: Option<ThemePreferences>,
  #[serde(default)]
  pub party_mode: PartyModeConfig,
  #[serde(default)]
  pub selected_misc_items: std::collections::HashMap<String, Vec<String>>,
  // New settings
  #[serde(default = "default_auto_update_data")]
  pub auto_update_data: bool,
  #[serde(default)]
  pub last_data_commit: Option<String>,
  #[serde(default)]
  pub cslol_tools_version: Option<String>,
}

fn default_auto_update_data() -> bool {
  true
}

// Party Mode related types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyModeConfig {
  #[serde(default)]
  pub paired_friends: Vec<PairedFriend>,
  #[serde(default)]
  pub notifications: bool,
  #[serde(default)]
  pub verbose_logging: bool,
  #[serde(default = "default_max_share_age")]
  pub max_share_age_secs: u64,
}

impl Default for PartyModeConfig {
  fn default() -> Self {
    Self {
      paired_friends: Vec::new(),
      notifications: true,
      verbose_logging: false,
      max_share_age_secs: default_max_share_age(),
    }
  }
}

fn default_max_share_age() -> u64 {
  300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedFriend {
  pub summoner_id: String,
  pub summoner_name: String,
  pub display_name: String,
  pub paired_at: u64,
  #[serde(default = "default_share_enabled")]
  pub share_enabled: bool,
}

fn default_share_enabled() -> bool {
  true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInfo {
  pub summoner_id: String,
  pub summoner_name: String,
  pub display_name: String,
  pub is_online: bool,
  pub availability: Option<String>,
  pub puuid: String,
  pub pid: String,
}

// Party Mode message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyModeMessage {
  pub message_type: String,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinShare {
  pub from_summoner_id: String,
  pub from_summoner_name: String,
  pub champion_id: u32,
  pub skin_id: u32,
  pub skin_name: String, // Add skin name field
  pub chroma_id: Option<u32>,
  pub skin_file_path: Option<String>,
  pub timestamp: u64,
}

// Copy paste your type definitions here - no additional imports needed as they're already included above
