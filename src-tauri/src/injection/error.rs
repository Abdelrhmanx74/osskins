use std::io;
use walkdir;
use zip;
use serde::{Deserialize, Serialize};

// Error handling for injection operations

#[derive(Debug)]
pub enum InjectionError {
    IoError(io::Error),
    InvalidGamePath(String),
    MissingFantomeFile(String),
    ProcessError(String),
    ConfigError(String),
    Timeout(String),
    Aborted(String),
    WalkdirError(walkdir::Error),
    ZipError(zip::result::ZipError),
}

impl std::fmt::Display for InjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IO Error: {}", err),
            Self::InvalidGamePath(msg) => write!(f, "Invalid game path: {}", msg),
            Self::MissingFantomeFile(msg) => write!(f, "Missing fantome file: {}", msg),
            Self::ProcessError(msg) => write!(f, "Process error: {}", msg),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::Aborted(msg) => write!(f, "Aborted: {}", msg),
            Self::WalkdirError(err) => write!(f, "Walkdir error: {}", err),
            Self::ZipError(err) => write!(f, "Zip error: {}", err),
        }
    }
}

impl std::error::Error for InjectionError {}

impl From<io::Error> for InjectionError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<walkdir::Error> for InjectionError {
    fn from(err: walkdir::Error) -> Self {
        Self::WalkdirError(err)
    }
}

impl From<zip::result::ZipError> for InjectionError {
    fn from(err: zip::result::ZipError) -> Self {
        Self::ZipError(err)
    }
}

// Define the types we need
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome_path: Option<String>, // Add fantome path from the JSON
}

// Misc item for injection alongside skins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscItem {
    pub id: String,
    pub name: String,
    pub item_type: String, // "map", "language", "hud", "misc"
    pub fantome_path: String,
}

// Injection request that includes both skins and misc items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionRequest {
    pub skins: Vec<Skin>,
    pub misc_items: Vec<MiscItem>,
}

// ModState enum - Similar to CS LOL Manager's state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModState {
    Uninitialized,
    Idle,
    Busy,
    Running,
    CriticalError,
}

// This represents a message event for the patcher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatcherMessage {
    WaitStart,
    Found,
    WaitInit,
    Scan,
    NeedSave,
    WaitPatchable,
    Patch,
    WaitExit,
    Done,
}

impl PatcherMessage {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::WaitStart => "Waiting for league match to start",
            Self::Found => "Found League",
            Self::WaitInit => "Wait initialized",
            Self::Scan => "Scanning",
            Self::NeedSave => "Saving",
            Self::WaitPatchable => "Wait patchable",
            Self::Patch => "Patching",
            Self::WaitExit => "Waiting for exit",
            Self::Done => "League exited",
        }
    }
}