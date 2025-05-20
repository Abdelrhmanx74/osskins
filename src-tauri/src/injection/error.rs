use std::io;

// Error handling similar to CS LOL Manager
#[derive(Debug)]
#[allow(dead_code)]
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