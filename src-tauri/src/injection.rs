use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

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

// Main skin injector class - mimicking CSLOLToolsImpl
pub struct SkinInjector {
    state: ModState,
    app_dir: PathBuf,
    game_path: PathBuf,
    status: String,
    log_file: Option<File>,
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
        fs::create_dir_all(app_dir.join("installed"))?;
        fs::create_dir_all(app_dir.join("profiles"))?;
        
        // Create log file
        let log_path = app_dir.join("log.txt");
        let log_file = File::create(&log_path)?;
        
        Ok(Self {
            state: ModState::Uninitialized,
            app_dir,
            game_path,
            status: String::new(),
            log_file: Some(log_file),
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
        fs::create_dir_all(self.app_dir.join("installed"))?;
        fs::create_dir_all(self.app_dir.join("profiles"))?;
        
        // Set to idle state when done
        self.set_state(ModState::Idle);
        Ok(())
    }
    
    // Get the mod list (similar to CSLOLToolsImpl::modList)
    pub fn mod_list(&self) -> Result<Vec<String>, InjectionError> {
        let mut result = Vec::new();
        let installed_dir = self.app_dir.join("installed");
        
        if !installed_dir.exists() {
            return Ok(result);
        }
        
        for entry in fs::read_dir(installed_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Skip non-directory entries or those ending with .tmp
            if !path.is_dir() || path.to_string_lossy().ends_with(".tmp") {
                continue;
            }
            
            // Skip . and .. directories
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if name == "." || name == ".." {
                continue;
            }
            
            // Check for META/info.json
            if !path.join("META").join("info.json").exists() {
                continue;
            }
            
            result.push(name.to_string());
        }
        
        result.sort();
        Ok(result)
    }

    // Profile-related methods
    fn list_profiles(&self) -> Result<Vec<String>, InjectionError> {
        let mut profiles = Vec::new();
        let profiles_dir = self.app_dir.join("profiles");
        
        if !profiles_dir.exists() {
            fs::create_dir_all(&profiles_dir)?;
        }
        
        for entry in fs::read_dir(profiles_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if !path.is_dir() {
                continue;
            }
            
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
                
            if name == "." || name == ".." {
                continue;
            }
            
            profiles.push(name.to_string());
        }
        
        // Ensure Default Profile exists
        if !profiles.contains(&"Default Profile".to_string()) {
            profiles.push("Default Profile".to_string());
        }
        
        profiles.sort();
        Ok(profiles)
    }
    
    fn read_profile(&self, profile_name: &str) -> Result<HashMap<String, bool>, InjectionError> {
        let mut profile = HashMap::new();
        let profile_file = self.app_dir.join("profiles").join(format!("{}.profile", profile_name));
        
        if profile_file.exists() {
            let data = fs::read_to_string(profile_file)?;
            for line in data.split('\n') {
                let line = line.trim();
                if !line.is_empty() {
                    profile.insert(line.to_string(), true);
                }
            }
        }
        
        Ok(profile)
    }
    
    fn write_profile(&self, profile_name: &str, mods: &HashMap<String, bool>) -> Result<(), InjectionError> {
        let profiles_dir = self.app_dir.join("profiles");
        fs::create_dir_all(&profiles_dir)?;
        
        let profile_file = profiles_dir.join(format!("{}.profile", profile_name));
        let mut file = File::create(profile_file)?;
        
        for mod_name in mods.keys() {
            if !mod_name.is_empty() {
                writeln!(file, "{}", mod_name)?;
            }
        }
        
        Ok(())
    }
    
    fn read_current_profile(&self) -> Result<String, InjectionError> {
        let current_profile_path = self.app_dir.join("current.profile");
        if !current_profile_path.exists() {
            return Ok("Default Profile".to_string());
        }
        
        let data = fs::read_to_string(current_profile_path)?;
        let name = data.trim().to_string();
        
        if name.is_empty() {
            Ok("Default Profile".to_string())
        } else {
            Ok(name)
        }
    }
    
    fn write_current_profile(&self, profile: &str) -> Result<(), InjectionError> {
        let current_profile_path = self.app_dir.join("current.profile");
        fs::write(current_profile_path, profile)?;
        Ok(())
    }
    
    // Debug function to list all fantome files in a directory
    fn list_all_fantome_files(&mut self, dir: &Path) -> Result<Vec<PathBuf>, InjectionError> {
        let mut result = Vec::new();
        
        if !dir.exists() {
            self.log(&format!("Directory does not exist: {}", dir.display()));
            return Ok(result);
        }
        
        self.log(&format!("Scanning for ALL fantome files in: {}", dir.display()));
        
        // Walk through all files recursively
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("fantome") {
                self.log(&format!("Found fantome file: {}", path.display()));
                result.push(path.to_path_buf());
            }
        }
        
        self.log(&format!("Found {} fantome files total", result.len()));
        return Ok(result);
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

    // Create a profile from a set of skins
    pub fn create_profile_from_skins(&mut self, profile_name: &str, skins: &[Skin], fantome_files_dir: &Path) -> Result<(), InjectionError> {
        if self.state != ModState::Idle {
            return Err(InjectionError::ProcessError("Injector is not in idle state".into()));
        }
        
        self.set_state(ModState::Busy);
        self.log(&format!("Creating profile '{}' from skins...", profile_name));
        
        // Create profile directory
        let profile_dir = self.app_dir.join("profiles").join(profile_name);
        if profile_dir.exists() {
            fs::remove_dir_all(&profile_dir)?;
        }
        fs::create_dir_all(&profile_dir)?;
        fs::create_dir_all(profile_dir.join("META"))?;
        fs::create_dir_all(profile_dir.join("WAD"))?;
        
        // Create META/info.json
        let info_json = format!(r#"{{
            "Name": "{}",
            "Version": "1.0.0",
            "Author": "fuck-exalted",
            "Description": "Skin profile created at {}"
        }}"#, profile_name, chrono::Local::now().to_rfc3339());
        
        fs::write(profile_dir.join("META").join("info.json"), info_json)?;
        
        // List all available fantome files for debugging
        let all_fantome_files = self.list_all_fantome_files(fantome_files_dir)?;
        
        // Copy fantome files
        let mut copied_count = 0;
        
        for skin in skins {
            self.log(&format!("Processing skin: champion_id={}, skin_id={}, chroma_id={:?}", 
                skin.champion_id, skin.skin_id, skin.chroma_id));
            
            // Check if we have a direct fantome path from the JSON data
            if let Some(fantome_path) = &skin.fantome_path {
                self.log(&format!("Using fantome path from JSON: {}", fantome_path));
                
                // First try the direct path from the JSON
                let direct_path = fantome_files_dir.join(fantome_path);
                if direct_path.exists() {
                    self.log(&format!("Found exact file at path: {}", direct_path.display()));
                    
                    let target_path = profile_dir.join("WAD").join(direct_path.file_name().unwrap());
                    fs::copy(&direct_path, &target_path)?;
                    copied_count += 1;
                    continue;
                }
                
                // If direct path fails, try to find it by champion name
                if let Some(champion_name) = self.get_champion_name(skin.champion_id) {
                    let champ_path = fantome_files_dir.join(champion_name).join(fantome_path.split('/').last().unwrap_or(""));
                    if champ_path.exists() {
                        self.log(&format!("Found file at champion path: {}", champ_path.display()));
                        
                        let target_path = profile_dir.join("WAD").join(champ_path.file_name().unwrap());
                        fs::copy(&champ_path, &target_path)?;
                        copied_count += 1;
                        continue;
                    }
                }
                
                // If we can't find the exact file, search for a match in all files
                self.log(&format!("Searching for a file matching fantome path: {}", fantome_path));
                let file_name = fantome_path.split('/').last().unwrap_or("");
                
                for fantome_path in &all_fantome_files {
                    let path_file_name = fantome_path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                        
                    if path_file_name == file_name {
                        self.log(&format!("Found matching file: {}", fantome_path.display()));
                        
                        let target_path = profile_dir.join("WAD").join(fantome_path.file_name().unwrap());
                        fs::copy(fantome_path, &target_path)?;
                        copied_count += 1;
                        break;
                    }
                }
            } else {
                // If no fantome path is available, fall back to the original search methods
                self.log("No fantome path available, using fallback search methods");
                
                // Try various search methods as before...
                let mut found = false;
                
                // Strategy 1: Try exact path using champion ID
                let id_path = fantome_files_dir.join(skin.champion_id.to_string());
                if id_path.exists() {
                    found = self.find_and_copy_skin_in_dir(&id_path, skin, &profile_dir)?;
                }
                
                // Strategy 2: Try path using champion name
                if !found {
                    if let Some(champion_name) = self.get_champion_name(skin.champion_id) {
                        let name_path = fantome_files_dir.join(champion_name);
                        if name_path.exists() {
                            found = self.find_and_copy_skin_in_dir(&name_path, skin, &profile_dir)?;
                        }
                    }
                }
                
                // Strategy 3: Search all files for a match
                if !found {
                    self.log("Searching all files for a match...");
                    let skin_id_str = skin.skin_id.to_string();
                    
                    for fantome_path in &all_fantome_files {
                        let file_name = fantome_path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        
                        // Match by skin ID
                        if file_name.contains(&skin_id_str) {
                            // For chroma skins
                            if let Some(chroma_id) = skin.chroma_id {
                                if file_name.contains("chroma") && file_name.contains(&chroma_id.to_string()) {
                                    self.log(&format!("Chroma match found: {}", fantome_path.display()));
                                    
                                    let target_path = profile_dir.join("WAD").join(fantome_path.file_name().unwrap());
                                    fs::copy(fantome_path, &target_path)?;
                                    copied_count += 1;
                                    found = true;
                                    break;
                                }
                            } else if !file_name.contains("chroma") {
                                self.log(&format!("Non-chroma match found: {}", fantome_path.display()));
                                
                                let target_path = profile_dir.join("WAD").join(fantome_path.file_name().unwrap());
                                fs::copy(fantome_path, &target_path)?;
                                copied_count += 1;
                                found = true;
                                break;
                            }
                        }
                    }
                }

                if !found {
                    self.log(&format!("Warning: Could not find fantome file for champion_id={}, skin_id={}, chroma_id={:?}", 
                        skin.champion_id, skin.skin_id, skin.chroma_id));
                }
            }
        }
        
        self.log(&format!("Copied {} fantome files to profile", copied_count));
        
        // Create profile entry
        let mut mods = HashMap::new();
        mods.insert(profile_name.to_string(), true);
        self.write_profile(profile_name, &mods)?;
        self.write_current_profile(profile_name)?;
        
        self.set_state(ModState::Idle);
        Ok(())
    }
    
    // Helper method to find and copy a skin in a specific directory
    fn find_and_copy_skin_in_dir(&mut self, dir_path: &Path, skin: &Skin, profile_dir: &Path) -> Result<bool, InjectionError> {
        let mut found = false;
        
        // Read all entries in the directory
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("fantome") {
                continue;
            }
            
            let file_name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
                
            let skin_id_str = skin.skin_id.to_string();
            
            // Check if this fantome file matches our skin
            if file_name.contains(&skin_id_str) {
                // For chroma skins
                if let Some(chroma_id) = skin.chroma_id {
                    if file_name.contains("chroma") && file_name.contains(&chroma_id.to_string()) {
                        self.log(&format!("Found chroma match in directory: {}", path.display()));
                        let target_path = profile_dir.join("WAD").join(path.file_name().unwrap());
                        fs::copy(&path, &target_path)?;
                        found = true;
                        break;
                    }
                } else if !file_name.contains("chroma") {
                    // For non-chroma skins
                    self.log(&format!("Found non-chroma match in directory: {}", path.display()));
                    let target_path = profile_dir.join("WAD").join(path.file_name().unwrap());
                    fs::copy(&path, &target_path)?;
                    found = true;
                    break;
                }
            }
        }
        
        Ok(found)
    }
    
    // This method runs mkoverlay similar to CS LOL Manager
    pub fn make_overlay(&mut self, profile_name: &str) -> Result<(), InjectionError> {
        if self.state != ModState::Idle {
            return Err(InjectionError::ProcessError("Injector is not in idle state".into()));
        }
        
        self.set_state(ModState::Busy);
        self.log(&format!("Creating overlay for profile '{}'...", profile_name));
        
        // Check if profile directory exists, if not create it
        let profile_dir = self.app_dir.join("profiles").join(profile_name);
        if !profile_dir.exists() {
            fs::create_dir_all(&profile_dir)?;
            fs::create_dir_all(profile_dir.join("META"))?;
            fs::create_dir_all(profile_dir.join("WAD"))?;
        }
        
        // Create profile config
        let config_path = self.app_dir.join("profiles").join(format!("{}.config", profile_name));
        let config_content = format!(r#"{{
            "gameDirectory": "{}",
            "overlayDirectory": "{}",
            "enabled": true
        }}"#, 
        self.game_path.display().to_string().replace('\\', "/"),
        profile_dir.display().to_string().replace('\\', "/"));
        
        fs::write(&config_path, config_content)?;
        self.log(&format!("Created config at: {}", config_path.display()));
        
        // Process all fantome files in profile
        self.process_fantome_files(profile_name)?;
        
        self.set_state(ModState::Idle);
        Ok(())
    }
    
    // Helper method to process fantome files
    fn process_fantome_files(&mut self, profile_name: &str) -> Result<(), InjectionError> {
        let profile_dir = self.app_dir.join("profiles").join(profile_name);
        let wad_dir = profile_dir.join("WAD");
        
        if !wad_dir.exists() {
            return Ok(()); // No WAD directory, nothing to do
        }
        
        let mut fantome_count = 0;
        
        // Copy all fantome files to game's mods directory
        let game_mods_dir = self.game_path.join("mods");
        fs::create_dir_all(&game_mods_dir)?;
        
        // First clear out old mods
        for entry in fs::read_dir(&game_mods_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("fantome") {
                fs::remove_file(path)?;
            }
        }
        
        // Now copy new mods
        for entry in WalkDir::new(&wad_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                
                // Only process .fantome files
                if path.extension().and_then(|e| e.to_str()) != Some("fantome") {
                    continue;
                }
                
                let filename = path.file_name()
                    .ok_or_else(|| InjectionError::MissingFantomeFile("Invalid fantome filename".into()))?;
                    
                let target_path = game_mods_dir.join(filename);
                fs::copy(path, &target_path)?;
                fantome_count += 1;
            }
        }
        
        self.log(&format!("Copied {} fantome files to game mods directory", fantome_count));
        
        // Update Game.cfg to enable mods
        self.enable_mods_in_game_cfg()?;
        
        Ok(())
    }
    
    // Enable mods in Game.cfg
    fn enable_mods_in_game_cfg(&mut self) -> Result<(), InjectionError> {
        let game_cfg_path = self.game_path.join("Game.cfg");
        
        // If file doesn't exist, create it with EnableMods=1
        if !game_cfg_path.exists() {
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
    
    // Main injection method that does all steps
    pub fn inject_skins(&mut self, skins: &[Skin], fantome_files_dir: &Path) -> Result<(), InjectionError> {
        if self.state != ModState::Idle {
            return Err(InjectionError::ProcessError("Injector is not in idle state".into()));
        }
        
        // Generate a unique profile name
        let profile_name = format!("InjectionProfile_{}", chrono::Local::now().timestamp());
        
        // Create profile with skins
        self.create_profile_from_skins(&profile_name, skins, fantome_files_dir)?;
        
        // Create overlay
        self.make_overlay(&profile_name)?;
        
        self.log("Skin injection completed successfully");
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