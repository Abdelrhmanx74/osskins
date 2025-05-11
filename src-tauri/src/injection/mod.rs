// Main injection module that re-exports all components
mod cache;
mod error;
mod injector;
mod types;
mod utils;

pub use cache::{OverlayCache, OVERLAY_CACHE};
pub use error::InjectionError;
pub use injector::SkinInjector;
pub use types::*;
pub use utils::*;

// Re-export the main public functions directly
pub use injector::{inject_skins, cleanup_injection};
pub use utils::{get_global_index, GLOBAL_FILE_INDEX, copy_default_overlay, emit_terminal_log_injection};