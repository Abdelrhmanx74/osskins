use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::injection::Skin;

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
    pub fantome: Option<String>, // Add fantome path from the JSON
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
    pub isDark: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedConfig {
    pub league_path: Option<String>,
    pub skins: Vec<SkinData>,
    pub favorites: Vec<u32>,
    #[serde(default)]
    pub theme: Option<ThemePreferences>,
    #[serde(default)]
    pub party_mode: PartyModeConfig,
}

// Party Mode related types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyModeConfig {
    #[serde(default)]
    pub paired_friends: Vec<PairedFriend>,
    #[serde(default)]
    pub auto_share: bool,
    #[serde(default)]
    pub notifications: bool,
    #[serde(default)]
    pub received_skins: std::collections::HashMap<String, ReceivedSkinData>,
    #[serde(default)]
    pub ignored_request_ids: Vec<String>,
    #[serde(default)]
    pub ignored_summoners: Vec<String>,
}

impl Default for PartyModeConfig {
    fn default() -> Self {
        Self {
            paired_friends: Vec::new(),
            auto_share: true,
            notifications: true,
            received_skins: std::collections::HashMap::new(),
            ignored_request_ids: Vec::new(),
            ignored_summoners: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedFriend {
    pub summoner_id: String,
    pub summoner_name: String,
    pub display_name: String,
    pub paired_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedSkinData {
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome_path: Option<String>,
    pub received_at: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRequest {
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub timestamp: u64,
}

// Party Mode message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyModeMessage {
    pub message_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub request_id: String,
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingResponse {
    pub request_id: String,
    pub accepted: bool,
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinShare {
    pub from_summoner_id: String,
    pub from_summoner_name: String,
    pub champion_id: u32,
    pub skin_id: u32,
    pub skin_name: String, // Add skin name field
    pub chroma_id: Option<u32>,
    pub fantome_path: Option<String>,
    pub timestamp: u64,
}

// Copy paste your type definitions here - no additional imports needed as they're already included above
