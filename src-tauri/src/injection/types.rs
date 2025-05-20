use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

// Define the types we need
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub champion_id: u32,
    pub skin_id: u32,
    pub chroma_id: Option<u32>,
    pub fantome_path: Option<String>, // Add fantome path from the JSON
}

// ModState enum - Similar to CS LOL Manager's state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModState {
    Uninitialized,
    Idle,
    Busy,
    Running,
    #[allow(dead_code)]
    CriticalError,
}

// This represents a message event for the patcher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
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

// FileIndex struct to cache paths and champion data
#[derive(Debug, Default)]
pub struct FileIndex {
    // Map champion_id to champion name
    pub champion_names: HashMap<u32, String>,
    // Map (champion_id, skin_id) to fantome file path
    pub skin_paths: HashMap<(u32, Option<u32>), Vec<PathBuf>>,
    // Map champion name to champion ID
    pub champion_ids: HashMap<String, u32>,
    // Track all discovered fantome files
    pub all_fantome_files: Vec<(PathBuf, Instant)>,
    // Track fantome files by filename for quick lookup
    pub fantome_by_filename: HashMap<String, PathBuf>,
    // Last time the index was built
    pub last_indexed: Option<Instant>,
}

impl FileIndex {
    // Create a new empty index
    pub fn new() -> Self {
        Self::default()
    }
}