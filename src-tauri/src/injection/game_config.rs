use crate::injection::error::InjectionError;
use std::fs;

// Game.cfg management operations

impl crate::injection::core::SkinInjector {
  // Enable mods in Game.cfg
  pub(crate) fn enable_mods_in_game_cfg(&mut self) -> Result<(), InjectionError> {
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
}
