// Injection module - Re-exports all injection functionalities
pub mod core;
pub mod error;
pub mod game_config;
pub mod mod_tools;
pub mod skin_file;

// Re-export all public types and functions
pub use core::*;
pub use error::*;

// Additional helper function for multi-champion injections without event emission
pub fn inject_skins_and_misc_no_events(
  app: &tauri::AppHandle,
  league_path: &str,
  skins: &[Skin],
  misc_items: &[MiscItem],
  skin_file_files_dir: &std::path::Path,
) -> Result<(), InjectionError> {
  let mut injector = core::SkinInjector::new(app, league_path)?;
  injector.inject_skins_and_misc_no_events(skins, misc_items, skin_file_files_dir)
}
