use serde::{Deserialize, Serialize};
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
}

// Copy paste your type definitions here - no additional imports needed as they're already included above
