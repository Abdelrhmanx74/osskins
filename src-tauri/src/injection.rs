use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;
use zip::ZipArchive;

// Error handling similar to CS LOL Manager
#[derive(Debug)]
pub enum InjectionError {
    IoError(io::Error),
    InvalidGamePath(String),
    MissingFantomeFile(String),
    ProcessError(String),
    ConfigError(String),
    OverlayError(String),
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
            Self::OverlayError(msg) => write!(f, "Overlay error: {}", msg),
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

// Main skin injector class - simplified without profiles
pub struct SkinInjector {
    state: ModState,
    app_dir: PathBuf,
    game_path: PathBuf,
    status: String,
    log_file: Option<File>,
    mod_tools_path: Option<PathBuf>, // Add mod_tools path
}

impl SkinInjector {
    pub fn new(app_handle: &AppHandle, game_path: &str) -> Result<Self, InjectionError> {
        // Get the app directory
        let app_dir = app_handle.path().app_data_dir()
            .map_err(|e| InjectionError::IoError(io::Error::new(io::ErrorKind::NotFound, format!("{}", e))))?;
        
        // Validate game path
        let game_path = PathBuf::from(game_path);
        if !game_path.join("League of Legends.exe").exists() {
            return Err(InjectionError::InvalidGamePath("League of Legends.exe not found".into()));
        }
        
        // Create directories needed
        fs::create_dir_all(app_dir.join("mods"))?;
        fs::create_dir_all(app_dir.join("temp"))?;
        
        // Create log file
        let log_path = app_dir.join("log.txt");
        let log_file = File::create(&log_path)?;

        // Look for mod-tools executable in multiple locations
        let mut mod_tools_path = None;
        
        // Check in resource directory
        match app_handle.path().resource_dir() {
            Ok(resource_dir) => {
                let candidate = resource_dir.join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate);
                }
            }
            Err(e) => {
                println!("Warning: Could not check resources directory: {}", e);
            }
        }
        
        // Check next to the app executable
        if mod_tools_path.is_none() {
            if let Ok(app_local_dir) = app_handle.path().app_local_data_dir() {
                let candidate = app_local_dir.join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate);
                }
            }
        }
        
        // Check in CSLOL directory
        if mod_tools_path.is_none() {
            if let Ok(app_local_dir) = app_handle.path().app_local_data_dir() {
                // Try looking in cslol-tools subdirectory
                let candidate = app_local_dir.join("cslol-tools").join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate);
                }
                
                // Try looking in the original CSLOL Manager directory
                let candidate = app_local_dir.join("..").join("cslol-manager-2024-10-27-401067d-prerelease").join("cslol-tools").join("mod-tools.exe");
                if candidate.exists() {
                    mod_tools_path = Some(candidate.canonicalize().unwrap_or(candidate));
                }
            }
        }
        
        Ok(Self {
            state: ModState::Uninitialized,
            app_dir,
            game_path,
            status: String::new(),
            log_file: Some(log_file),
            mod_tools_path,
        })
    }
    
    fn log(&mut self, message: &str) {
        // Append timestamp and log message
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Some(log_file) = &mut self.log_file {
            let _ = writeln!(log_file, "[{}] {}", timestamp, message);
            let _ = log_file.flush();
        }
        println!("[{}] {}", timestamp, message);
        self.status = message.to_string();
    }
    
    fn set_state(&mut self, new_state: ModState) {
        if self.state != new_state {
            self.state = new_state;
            self.log(&format!("State changed to: {:?}", new_state));
        }
    }

    pub fn initialize(&mut self) -> Result<(), InjectionError> {
        if self.state != ModState::Uninitialized {
            return Ok(());
        }
        
        self.set_state(ModState::Busy);
        self.log("Initializing...");
        
        // Create required directories
        fs::create_dir_all(self.app_dir.join("mods"))?;
        fs::create_dir_all(self.app_dir.join("temp"))?;
        
        // Set to idle state when done
        self.set_state(ModState::Idle);
        Ok(())
    }

    // Helper function to get champion name from ID
    fn get_champion_name(&self, champion_id: u32) -> Option<&'static str> {
        // This is a mapping of champion IDs to their names
        // We need this because the directory structure might use champion names instead of IDs
        match champion_id {
            1 => Some("annie"),
            2 => Some("olaf"),
            3 => Some("galio"),
            4 => Some("twistedfate"),
            5 => Some("xinzhao"),
            6 => Some("urgot"),
            7 => Some("leblanc"),
            8 => Some("vladimir"),
            9 => Some("fiddlesticks"),
            10 => Some("kayle"),
            11 => Some("masteryi"),
            12 => Some("alistar"),
            13 => Some("ryze"),
            14 => Some("sion"),
            15 => Some("sivir"),
            16 => Some("soraka"),
            17 => Some("teemo"),
            18 => Some("tristana"),
            19 => Some("warwick"),
            20 => Some("nunu"),
            21 => Some("missfortune"),
            22 => Some("ashe"),
            23 => Some("tryndamere"),
            24 => Some("jax"),
            25 => Some("morgana"),
            26 => Some("zilean"),
            27 => Some("singed"),
            28 => Some("evelynn"),
            29 => Some("twitch"),
            30 => Some("karthus"),
            31 => Some("chogath"),
            32 => Some("amumu"),
            33 => Some("rammus"),
            34 => Some("anivia"),
            35 => Some("shaco"),
            36 => Some("drmundo"),
            37 => Some("sona"),
            38 => Some("kassadin"),
            39 => Some("irelia"),
            40 => Some("janna"),
            41 => Some("gangplank"),
            42 => Some("corki"),
            43 => Some("karma"),
            44 => Some("taric"),
            45 => Some("veigar"),
            48 => Some("trundle"),
            50 => Some("swain"),
            51 => Some("caitlyn"),
            53 => Some("blitzcrank"),
            54 => Some("malphite"),
            55 => Some("katarina"),
            56 => Some("nocturne"),
            57 => Some("maokai"),
            58 => Some("renekton"),
            59 => Some("jarvaniv"),
            60 => Some("elise"),
            61 => Some("orianna"),
            62 => Some("wukong"),
            63 => Some("brand"),
            64 => Some("leesin"),
            67 => Some("vayne"),
            68 => Some("rumble"),
            69 => Some("cassiopeia"),
            72 => Some("skarner"),
            74 => Some("heimerdinger"),
            75 => Some("nasus"),
            76 => Some("nidalee"),
            77 => Some("udyr"),
            78 => Some("poppy"),
            79 => Some("gragas"),
            80 => Some("pantheon"),
            81 => Some("ezreal"),
            82 => Some("mordekaiser"),
            83 => Some("yorick"),
            84 => Some("akali"),
            85 => Some("kennen"),
            86 => Some("garen"),
            89 => Some("leona"),
            90 => Some("malzahar"),
            91 => Some("talon"),
            92 => Some("riven"),
            96 => Some("kogmaw"),
            98 => Some("shen"),
            99 => Some("lux"),
            101 => Some("xerath"),
            102 => Some("shyvana"),
            103 => Some("ahri"),
            104 => Some("graves"),
            105 => Some("fizz"),
            106 => Some("volibear"),
            107 => Some("rengar"),
            110 => Some("varus"),
            111 => Some("nautilus"),
            112 => Some("viktor"),
            113 => Some("sejuani"),
            114 => Some("fiora"),
            115 => Some("ziggs"),
            117 => Some("lulu"),
            119 => Some("draven"),
            120 => Some("hecarim"),
            121 => Some("khazix"),
            122 => Some("darius"),
            126 => Some("jayce"),
            127 => Some("lissandra"),
            131 => Some("diana"),
            133 => Some("quinn"),
            134 => Some("syndra"),
            136 => Some("aurelionsol"),
            141 => Some("kayn"),
            142 => Some("zoe"),
            143 => Some("zyra"),
            145 => Some("kaisa"),
            147 => Some("seraphine"),
            150 => Some("gnar"),
            154 => Some("zac"),
            157 => Some("yasuo"),
            161 => Some("velkoz"),
            163 => Some("taliyah"),
            164 => Some("camille"),
            201 => Some("braum"),
            202 => Some("jhin"),
            203 => Some("kindred"),
            222 => Some("jinx"),
            223 => Some("tahmkench"),
            234 => Some("viego"),
            235 => Some("senna"),
            236 => Some("lucian"),
            238 => Some("zed"),
            240 => Some("kled"),
            245 => Some("ekko"),
            246 => Some("qiyana"),
            254 => Some("vi"),
            266 => Some("aatrox"),
            267 => Some("nami"),
            268 => Some("azir"),
            350 => Some("yuumi"),
            360 => Some("samira"),
            412 => Some("thresh"),
            420 => Some("illaoi"),
            421 => Some("reksai"),
            427 => Some("ivern"),
            429 => Some("kalista"),
            432 => Some("bard"),
            497 => Some("rakan"),
            498 => Some("xayah"),
            516 => Some("ornn"),
            517 => Some("sylas"),
            518 => Some("neeko"),
            523 => Some("aphelios"),
            526 => Some("rell"),
            555 => Some("pyke"),
            711 => Some("vex"),
            777 => Some("yone"),
            875 => Some("sett"),
            876 => Some("lillia"),
            887 => Some("gwen"),
            888 => Some("renata"),
            895 => Some("nilah"),
            897 => Some("ksante"),
            902 => Some("milio"),
            950 => Some("naafiri"),
            // Add more mappings as needed
            _ => None,
        }
    }
    
    // Extract .fantome file (similar to utility::unzip in CSLOL Manager)
    fn extract_fantome(&mut self, fantome_path: &Path, output_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Extracting fantome file: {}", fantome_path.display()));
        
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Open and extract the zip file
        let file = fs::File::open(fantome_path)?;
        let mut archive = ZipArchive::new(file)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => output_dir.join(path),
                None => continue,
            };
            
            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }
        
        Ok(())
    }
    
    // Check if directory contains META/info.json to confirm it's a valid mod
    fn is_valid_mod_dir(&self, dir_path: &Path) -> bool {
        dir_path.join("META").join("info.json").exists()
    }
    
    // Find appropriate .fantome file for a skin
    fn find_fantome_for_skin(&mut self, skin: &Skin, fantome_files_dir: &Path) -> Result<Option<PathBuf>, InjectionError> {
        // First try direct path from JSON
        if let Some(fantome_path) = &skin.fantome_path {
            self.log(&format!("Using fantome path from JSON: {}", fantome_path));
            
            // Try direct path
            let direct_path = fantome_files_dir.join(fantome_path);
            if direct_path.exists() {
                self.log(&format!("Found exact file at path: {}", direct_path.display()));
                return Ok(Some(direct_path));
            }
            
            // Try path using champion name
            if let Some(champion_name) = self.get_champion_name(skin.champion_id) {
                let champ_path = fantome_files_dir.join(champion_name).join(fantome_path.split('/').last().unwrap_or(""));
                if champ_path.exists() {
                    self.log(&format!("Found file at champion path: {}", champ_path.display()));
                    return Ok(Some(champ_path));
                }
            }
            
            // Search for matching filename
            let file_name = fantome_path.split('/').last().unwrap_or("");
            for entry in WalkDir::new(fantome_files_dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if path.file_name()
                       .map(|name| name.to_string_lossy() == file_name)
                       .unwrap_or(false) {
                        self.log(&format!("Found matching file: {}", path.display()));
                        return Ok(Some(path.to_path_buf()));
                    }
                }
            }
        }
        
        // Fall back to searching by ID
        self.log(&format!("Searching for skin with champion_id={}, skin_id={}, chroma_id={:?}", 
            skin.champion_id, skin.skin_id, skin.chroma_id));
            
        let skin_id_str = skin.skin_id.to_string();
        
        // Try champion directory first
        if let Some(champion_name) = self.get_champion_name(skin.champion_id) {
            let champ_dir = fantome_files_dir.join(champion_name);
            if champ_dir.exists() {
                for entry in fs::read_dir(champ_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("fantome") {
                        continue;
                    }
                    
                    let file_name = path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                        
                    if file_name.contains(&skin_id_str) {
                        // Check for chroma
                        if let Some(chroma_id) = skin.chroma_id {
                            if file_name.contains("chroma") && file_name.contains(&chroma_id.to_string()) {
                                self.log(&format!("Found chroma match: {}", path.display()));
                                return Ok(Some(path.to_path_buf()));
                            }
                        } else if !file_name.contains("chroma") {
                            self.log(&format!("Found non-chroma match: {}", path.display()));
                            return Ok(Some(path.to_path_buf()));
                        }
                    }
                }
            }
        }
        
        // Search all files as last resort
        for entry in WalkDir::new(fantome_files_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("fantome") {
                    continue;
                }
                
                let file_name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                    
                if file_name.contains(&skin_id_str) {
                    // Check for chroma
                    if let Some(chroma_id) = skin.chroma_id {
                        if file_name.contains("chroma") && file_name.contains(&chroma_id.to_string()) {
                            self.log(&format!("Found chroma match in full search: {}", path.display()));
                            return Ok(Some(path.to_path_buf()));
                        }
                    } else if !file_name.contains("chroma") {
                        self.log(&format!("Found non-chroma match in full search: {}", path.display()));
                        return Ok(Some(path.to_path_buf()));
                    }
                }
            }
        }
        
        self.log(&format!("No fantome file found for skin: champion_id={}, skin_id={}, chroma_id={:?}",
            skin.champion_id, skin.skin_id, skin.chroma_id));
        Ok(None)
    }
    
    // Create a mod directory structure from extracted fantome files
    fn create_mod_from_extracted(&mut self, extract_dir: &Path, mod_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Creating mod from extracted files at: {}", extract_dir.display()));
        
        // Create mod directories
        fs::create_dir_all(mod_dir.join("META"))?;
        fs::create_dir_all(mod_dir.join("WAD"))?;
        
        // Check if there's already a META/info.json in the extracted content
        let extracted_info_json = extract_dir.join("META").join("info.json");
        let mod_info_json = mod_dir.join("META").join("info.json");
        
        if extracted_info_json.exists() {
            // Copy the existing info.json
            fs::copy(&extracted_info_json, &mod_info_json)?;
        } else {
            // Create a basic info.json
            let info_json = format!(r#"{{
                "Name": "ExtractedMod",
                "Version": "1.0.0",
                "Author": "fuck-exalted",
                "Description": "Extracted from fantome file at {}"
            }}"#, chrono::Local::now().to_rfc3339());
            
            fs::write(&mod_info_json, info_json)?;
        }
        
        // Look for WAD directory in extracted content
        let extracted_wad_dir = extract_dir.join("WAD");
        if extracted_wad_dir.exists() {
            // Copy WAD files
            for entry in WalkDir::new(&extracted_wad_dir) {
                let entry = entry?;
                let path = entry.path();
                let rel_path = path.strip_prefix(&extracted_wad_dir)
                    .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
                
                let target_path = mod_dir.join("WAD").join(rel_path);
                
                if path.is_dir() {
                    fs::create_dir_all(&target_path)?;
                } else if path.is_file() {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(path, &target_path)?;
                }
            }
        } else {
            // If no WAD directory, look for WAD files in the root
            for entry in WalkDir::new(extract_dir) {
                let entry = entry?;
                let path = entry.path();
                
                // Skip META directory
                if path.starts_with(extract_dir.join("META")) {
                    continue;
                }
                
                // Check if this is a WAD file
                if path.is_file() && 
                   (path.extension().and_then(|ext| ext.to_str()) == Some("wad") ||
                    path.to_string_lossy().ends_with(".wad.client")) {
                    
                    let file_name = path.file_name().unwrap();
                    let target_path = mod_dir.join("WAD").join(file_name);
                    
                    fs::copy(path, &target_path)?;
                }
            }
        }
        
        Ok(())
    }
    
    // Process .fantome files to create proper mod structure (similar to CSLOL's mod_import)
    fn process_fantome_file(&mut self, fantome_path: &Path) -> Result<PathBuf, InjectionError> {
        self.log(&format!("Processing fantome file: {}", fantome_path.display()));
        
        // Create temp extraction directory
        let file_stem = fantome_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let extract_dir = self.app_dir.join("temp").join(&file_stem);
        let mod_dir = self.app_dir.join("mods").join(&file_stem);
        
        // Clean up any existing directories
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        if mod_dir.exists() {
            fs::remove_dir_all(&mod_dir)?;
        }
        
        // Extract the fantome file
        self.extract_fantome(fantome_path, &extract_dir)?;
        
        // Create mod structure
        self.create_mod_from_extracted(&extract_dir, &mod_dir)?;
        
        // Clean up extraction directory
        fs::remove_dir_all(&extract_dir)?;
        
        Ok(mod_dir)
    }
    
    // Enable mods in Game.cfg
    fn enable_mods_in_game_cfg(&mut self) -> Result<(), InjectionError> {
        let game_cfg_path = self.game_path.join("Game.cfg");
        
        // If file doesn't exist, create it with EnableMods=1
        if (!game_cfg_path.exists()) {
            fs::write(game_cfg_path, "[General]\nEnableMods=1\n")?;
            self.log("Created Game.cfg with EnableMods=1");
            return Ok(());
        }
        
        // Otherwise, read and modify the file
        let content = fs::read_to_string(&game_cfg_path)?;
        
        // Check if EnableMods is already set correctly
        if content.contains("EnableMods=1") {
            self.log("Game.cfg already has EnableMods=1");
            return Ok(());
        }
        
        // Replace EnableMods=0 with EnableMods=1 if it exists
        let mut new_content = content.clone();
        if content.contains("EnableMods=0") {
            new_content = content.replace("EnableMods=0", "EnableMods=1");
        } else {
            // Add EnableMods=1 to the [General] section if it exists
            if content.contains("[General]") {
                let parts: Vec<&str> = content.split("[General]").collect();
                if parts.len() >= 2 {
                    // Fix the temporary value borrowed error
                    let new_part = format!("\nEnableMods=1{}", parts[1]);
                    new_content = format!("{}[General]{}", parts[0], new_part);
                }
            } else {
                // If no [General] section, add it
                new_content = format!("{}\n[General]\nEnableMods=1\n", content);
            }
        }
        
        // Write the updated content
        fs::write(game_cfg_path, new_content)?;
        self.log("Updated Game.cfg to enable mods");
        
        Ok(())
    }
    
    // Copy a processed mod directory to the game's mods directory
    fn copy_mod_to_game(&mut self, mod_dir: &Path) -> Result<(), InjectionError> {
        self.log(&format!("Copying mod to game directory: {}", mod_dir.display()));

        // Use the mod directory name as the subfolder
        let mod_name = mod_dir.file_name().unwrap();
        let game_mod_dir = self.game_path.join("mods").join(mod_name);

        // Remove any existing mod with the same name
        if game_mod_dir.exists() {
            fs::remove_dir_all(&game_mod_dir)?;
        }
        fs::create_dir_all(&game_mod_dir)?;

        // Copy everything from mod_dir into game_mod_dir
        for entry in WalkDir::new(mod_dir) {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path.strip_prefix(mod_dir)
                .map_err(|e| InjectionError::ProcessError(format!("Path error: {}", e)))?;
            let target_path = game_mod_dir.join(rel_path);

            if path.is_dir() {
                fs::create_dir_all(&target_path)?;
            } else if path.is_file() {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &target_path)?;
            }
        }
        Ok(())
    }
    
    // Run the overlay process using mod-tools.exe - this is the critical function that was missing!
    fn run_overlay(&mut self) -> Result<(), InjectionError> {
        // Validate mod-tools.exe exists
        let mod_tools_path = self.mod_tools_path
            .as_ref()
            .map(|path| path.clone())
            .ok_or_else(|| InjectionError::ProcessError("mod-tools.exe not found".into()))?;

        self.log(&format!("Using mod-tools.exe from: {}", mod_tools_path.display()));

        // First create the overlay
        let game_mods_dir = self.game_path.join("mods");
        let overlay_dir = self.app_dir.join("overlay");
        if overlay_dir.exists() {
            fs::remove_dir_all(&overlay_dir)?;
        }
        fs::create_dir_all(&overlay_dir)?;

        // Get list of mod names (just the directory names, no paths)
        let mut mod_names = Vec::new();
        for entry in fs::read_dir(&game_mods_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("META").join("info.json").exists() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        mod_names.push(name_str.to_string());
                    }
                }
            }
        }

        // Join mod names with / as CSLOL expects
        let mods_arg = mod_names.join("/");

        self.log("Creating mod overlay...");
        let output = std::process::Command::new(&mod_tools_path)
            .args([
                "mkoverlay",
                game_mods_dir.to_str().unwrap(),
                overlay_dir.to_str().unwrap(),
                &format!("--game:{}", self.game_path.to_str().unwrap()),
                &format!("--mods:{}", mods_arg),
                "--noTFT",
                "--ignoreConflict"
            ])
            .output()
            .map_err(|e| InjectionError::ProcessError(format!("Failed to create overlay: {}", e)))?;

        if !output.status.success() {
            return Err(InjectionError::ProcessError(
                String::from_utf8_lossy(&output.stderr).into_owned()
            ));
        }

        // Create config.json
        let config_path = self.app_dir.join("config.json");
        let config_content = r#"{"enableMods":true}"#;
        fs::write(&config_path, config_content)?;

        self.log("Starting overlay process...");

        // Important: Set state to Running BEFORE spawning process
        self.set_state(ModState::Running);

        // Run the overlay process - EXACT format from CSLOL
        let mut command = std::process::Command::new(&mod_tools_path);
        command.args([
            "runoverlay",
            overlay_dir.to_str().unwrap(),
            config_path.to_str().unwrap(),
            &format!("--game:{}", self.game_path.to_str().unwrap()),
            "--opts:configless"
        ]);

        match command.spawn() {
            Ok(_) => {
                self.log("Overlay process started successfully");
                Ok(())
            },
            Err(e) => {
                self.set_state(ModState::Idle); // Reset state on error
                self.log(&format!("Failed to start overlay process: {}", e));
                Err(InjectionError::OverlayError(format!(
                    "Failed to start overlay process: {}. See documentation for obtaining mod-tools.exe", e
                )))
            }
        }
    }

    // Main injection method that does all steps
    pub fn inject_skins(&mut self, skins: &[Skin], fantome_files_dir: &Path) -> Result<(), InjectionError> {
        if self.state != ModState::Idle {
            return Err(InjectionError::ProcessError("Injector is not in idle state".into()));
        }
        
        self.set_state(ModState::Busy);
        self.log("Starting skin injection process...");
        
        // First, clean up the game's mods directory
        let game_mods_dir = self.game_path.join("mods");
        if game_mods_dir.exists() {
            self.log("Cleaning up existing mods in game directory");
            fs::remove_dir_all(&game_mods_dir)?;
        }
        fs::create_dir_all(&game_mods_dir)?;
        
        // Process each skin
        for (i, skin) in skins.iter().enumerate() {
            self.log(&format!("Processing skin {}/{}: champion_id={}, skin_id={}, chroma_id={:?}", 
                i + 1, skins.len(), skin.champion_id, skin.skin_id, skin.chroma_id));
                
            // Find the fantome file
            if let Some(fantome_path) = self.find_fantome_for_skin(skin, fantome_files_dir)? {
                self.log(&format!("Found fantome file: {}", fantome_path.display()));
                
                // Process the fantome file to create a proper mod structure
                let mod_dir = self.process_fantome_file(&fantome_path)?;
                
                // Copy the processed mod to the game
                if self.is_valid_mod_dir(&mod_dir) {
                    self.log("Mod structure is valid, copying to game directory");
                    self.copy_mod_to_game(&mod_dir)?;
                } else {
                    // If processing failed, fall back to direct copy
                    self.log("WARNING: Processing failed, falling back to direct copy");
                    let game_fantome_path = game_mods_dir.join(fantome_path.file_name().unwrap());
                    fs::copy(&fantome_path, &game_fantome_path)?;
                }
            } else {
                self.log(&format!("WARNING: No fantome file found for skin: champion_id={}, skin_id={}, chroma_id={:?}",
                    skin.champion_id, skin.skin_id, skin.chroma_id));
            }
        }
        
        // Enable mods in Game.cfg
        self.enable_mods_in_game_cfg()?;
        
        // Start the overlay process - THIS is the key part that makes skins actually show in-game!
        if let Err(e) = self.run_overlay() {
            self.log(&format!("WARNING: Failed to start overlay process: {}. The mods have been copied but the overlay could not be started. You need to obtain mod-tools.exe from CSLOL Manager.", e));
            // Still consider it a success as we've copied the mods
            self.set_state(ModState::Idle);
            return Ok(());
        }
        
        self.log("Skin injection completed successfully");
        // Note: We don't set state to Idle because we're now in Running state with the overlay active
        Ok(())
    }
}

// Main wrapper function that is called from commands.rs
pub fn inject_skins(
    app_handle: &AppHandle, 
    game_path: &str, 
    skins: &[Skin], 
    fantome_files_dir: &Path
) -> Result<(), String> {
    // Create injector
    let mut injector = SkinInjector::new(app_handle, game_path)
        .map_err(|e| format!("Failed to create injector: {}", e))?;
    
    // Initialize
    injector.initialize()
        .map_err(|e| format!("Failed to initialize: {}", e))?;
    
    // Inject skins
    injector.inject_skins(skins, fantome_files_dir)
        .map_err(|e| format!("Failed to inject skins: {}", e))
}