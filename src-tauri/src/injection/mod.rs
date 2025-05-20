// Main injection module that re-exports all components
mod cache;
mod error;
mod injector;
mod types;
mod utils;

pub use injector::SkinInjector;
pub use types::*;
pub use utils::*;

// Re-export the main public functions directly
pub use injector::inject_skins;