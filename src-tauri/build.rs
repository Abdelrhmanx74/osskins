use cmake;
use std::{env, fs, path::Path};

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
  let dst = cmake::Config::new("../cslol-manager-2024-10-27-401067d-prerelease/cslol-tools")
    .profile("Release")
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
