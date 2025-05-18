use cmake;
use std::{env, fs, path::Path, process::Command};

fn main() {
  // Determine build profile to avoid infinite rebuild loop in dev
  let profile = env::var("PROFILE").unwrap_or_default();
  if profile != "release" {
    // Skip C++ build and resource copy in debug/dev mode
    tauri_build::build();
    return;
  }
  // Build mod-tools C++ binary before Tauri bundle validation
  // Build mod-tools C++ binary from cslol-tools source
  let dst = cmake::Config::new("./cslol-manager/cslol-tools")
    .profile("Release")
    // Add policy to handle older CMake configurations
    .define("CMAKE_POLICY_DEFAULT_CMP0048", "NEW")
    .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
    // Build only the mod-tools executable target (skip install step)
    .build_target("mod-tools")
    .build();
  // Path to generated exe
  let exe_src = dst.join("build").join("Release").join("mod-tools.exe");
  // Output resources directory in src-tauri for bundling
  let out_resources = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap())
    .join("resources").join("cslol-tools");
  fs::create_dir_all(&out_resources).expect("Failed to create resources/cslol-tools directory");
  fs::copy(&exe_src, out_resources.join("mod-tools.exe")).expect("Failed to copy mod-tools.exe");
  
  // Pre-build default empty overlays for faster first-time injection
  pre_build_default_overlays(&out_resources.join("mod-tools.exe"), &out_resources);
  
  // Copy dependent DLLs and PDBs
  let build_dir = dst.join("build").join("Release");
  for entry in fs::read_dir(&build_dir).expect("Failed to read build directory") {
    let path = entry.expect("Invalid entry").path();
    if let Some(ext) = path.extension() {
      if ext == "dll" || ext == "pdb" {
        let name = path.file_name().unwrap();
        fs::copy(&path, out_resources.join(name)).expect("Failed to copy dependency");
      }
    }
  }

  // Run Tauri build (bundles resources including cslol-tools)
  tauri_build::build();
}

/// Create pre-built empty overlays to speed up first injection
fn pre_build_default_overlays(mod_tools_path: &Path, resources_dir: &Path) {
  println!("Building pre-generated overlay templates...");
  
  // Create empty mods directory structure
  let temp_dir = resources_dir.join("temp_build");
  let mods_dir = temp_dir.join("mods");
  let empty_overlay_dir = resources_dir.join("empty_overlay");
  
  // Clean up any previous build artifacts
  if temp_dir.exists() {
    fs::remove_dir_all(&temp_dir).expect("Failed to clean temp directory");
  }
  if empty_overlay_dir.exists() {
    fs::remove_dir_all(&empty_overlay_dir).expect("Failed to clean empty overlay directory");
  }
  
  // Create the directories
  fs::create_dir_all(&mods_dir).expect("Failed to create temporary mods directory");
  fs::create_dir_all(&empty_overlay_dir).expect("Failed to create empty overlay directory");
  
  // Create a dummy empty mod to ensure the structure is valid
  let dummy_mod_dir = mods_dir.join("empty_mod");
  fs::create_dir_all(&dummy_mod_dir.join("META")).expect("Failed to create META directory");
  fs::create_dir_all(&dummy_mod_dir.join("WAD")).expect("Failed to create WAD directory");
  
  // Create a basic info.json
  let info_json = r#"{
    "Name": "EmptyMod",
    "Version": "1.0.0",
    "Author": "osskins",
    "Description": "Pre-built empty mod for faster first injection"
  }"#;
  fs::write(dummy_mod_dir.join("META").join("info.json"), info_json)
    .expect("Failed to write info.json");
  
  // Run mod-tools to create an empty overlay
  // Use an arbitrary game path since we're just creating a template
  let game_path = temp_dir.join("game");
  fs::create_dir_all(&game_path).expect("Failed to create temporary game directory");
  
  // First try with cmd.exe to avoid potential PowerShell issues
  let status = Command::new("cmd")
    .args([
      "/C", 
      mod_tools_path.to_str().unwrap(),
      "mkoverlay",
      mods_dir.to_str().unwrap(),
      empty_overlay_dir.to_str().unwrap(),
      &format!("--game:{}", game_path.to_str().unwrap()),
      "--mods:empty_mod",
      "--noTFT",
      "--ignoreConflict"
    ])
    .status();
  
  match status {
    Ok(exit_status) => {
      if exit_status.success() {
        println!("Successfully pre-built empty overlay");
      } else {
        println!("Warning: Failed to pre-build empty overlay (Exit code: {:?})", exit_status.code());
      }
    },
    Err(e) => {
      println!("Warning: Failed to execute mod-tools for pre-building overlay: {}", e);
    }
  }
  
  // Clean up the temp directory
  if temp_dir.exists() {
    let _ = fs::remove_dir_all(&temp_dir);
  }
}
