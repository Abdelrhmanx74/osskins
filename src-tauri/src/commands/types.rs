use serde::{Deserialize, Serialize};
use crate::injection::Skin;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataUpdateProgress {
    pub current_champion: String,
    pub total_champions: usize,
    pub processed_champions: usize,
    pub status: String,
    pub progress: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinInjectionRequest {
    pub league_path: String,
    pub skins: Vec<Skin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinData {
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataVersion {
    pub version: String,
    pub timestamp: String,
    pub commit_hash: Option<String>,
    pub last_checked: i64,
    pub last_updated: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubCommit {
    pub sha: String,
    pub commit: GitHubCommitDetail,
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubCommitDetail {
    pub message: String,
    pub author: GitHubAuthor,
    pub committer: GitHubAuthor,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubAuthor {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommitter {
    pub date: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommitDetails {
    pub message: String,
    pub committer: GitHubCommitter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataUpdateResult {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub updated_champions: Vec<String>,
    #[serde(default)]
    pub has_update: bool,
    #[serde(default)]
    pub current_version: Option<String>,
    #[serde(default)]
    pub available_version: Option<String>,
    #[serde(default)]
    pub update_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemePreferences {
    pub tone: Option<String>,
    pub is_dark: Option<bool>,
    pub auto_update_champion_data: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedConfig {
    pub league_path: Option<String>,
    pub skins: Vec<SkinData>,
    pub favorites: Vec<u32>,
    #[serde(default)]
    pub theme: Option<ThemePreferences>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalLog {
    pub message: String,
    pub log_type: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Friend {
    pub id: String,
    pub name: String,
    pub availability: String,
    #[serde(rename = "gameTag")]
    pub game_tag: Option<String>,
    #[serde(rename = "note")]
    pub note: Option<String>,
}

// Constants
pub const GITHUB_API_URL: &str = "https://api.github.com/repos/darkseal-org/lol-skins-developer";
pub const USER_AGENT: &str = "fuck-exalted-app/1.0";
pub const _DATA_VERSION_FILE: &str = "data_version.json";
pub const GITHUB_API_VERSION: &str = "2022-11-28";
pub const _GITHUB_REPO_OWNER: &str = "darkseal-org";
pub const _GITHUB_REPO_NAME: &str = "lol-skins-developer";