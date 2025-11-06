#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

// League of Legends directory detection and selection

#[tauri::command]
pub async fn select_league_directory() -> Result<String, String> {
  #[cfg(target_os = "windows")]
  const CREATE_NO_WINDOW: u32 = 0x08000000;

  let mut command = Command::new("powershell");

  #[cfg(target_os = "windows")]
  command.creation_flags(CREATE_NO_WINDOW); // CREATE_NO_WINDOW flag

  command.args([
    "-NoProfile",
    "-Command",
    r#"Add-Type -AssemblyName System.Windows.Forms; 
            $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; 
            $dialog.Description = 'Select League of Legends Installation Directory'; 
            if($dialog.ShowDialog() -eq 'OK') { $dialog.SelectedPath }"#,
  ]);

  let output = command
    .output()
    .map_err(|e| format!("Failed to execute powershell command: {}", e))?;

  if !output.status.success() {
    return Err("Directory selection cancelled".to_string());
  }

  let path = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse selected path: {}", e))?
    .trim()
    .to_string();

  if path.is_empty() {
    return Err("No directory selected".to_string());
  }

  // Validate that this appears to be a League of Legends directory
  // Check for either the Game\League of Legends.exe or LeagueClient.exe
  let selected_dir = Path::new(&path);
  let game_exe_path = selected_dir.join("Game").join("League of Legends.exe");
  let client_exe_path = selected_dir.join("LeagueClient.exe");

  if !client_exe_path.exists() && !game_exe_path.exists() {
    return Err(
      "Selected directory does not appear to be a valid League of Legends installation".to_string(),
    );
  }

  // Always return the root League directory path
  Ok(path)
}

#[tauri::command]
pub async fn auto_detect_league() -> Result<String, String> {
  // Common League of Legends installation paths on Windows
  let common_paths = [
    r"C:\Riot Games\League of Legends",
    r"C:\Program Files\Riot Games\League of Legends",
    r"C:\Program Files (x86)\Riot Games\League of Legends",
  ];

  for path in common_paths.iter() {
    let client_path = Path::new(path).join("LeagueClient.exe");
    if client_path.exists() {
      return Ok(path.to_string());
    }
  }

  // Try to find through registry as fallback
  let mut command = Command::new("powershell");
  #[cfg(target_os = "windows")]
  command.creation_flags(0x08000000); // CREATE_NO_WINDOW flag

  command
        .args([
            "-NoProfile",
            "-Command",
            r#"Get-ItemProperty -Path 'HKLM:\SOFTWARE\WOW6432Node\Riot Games, Inc\League of Legends' -Name 'Location' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Location"#,
        ]);

  if let Ok(output) = command.output() {
    if output.status.success() {
      if let Ok(path) = String::from_utf8(output.stdout) {
        let path = path.trim();
        if !path.is_empty() {
          let path = Path::new(path);
          if path.join("LeagueClient.exe").exists() {
            return Ok(path.to_string_lossy().to_string());
          }
        }
      }
    }
  }

  Err("League of Legends installation not found".to_string())
}
