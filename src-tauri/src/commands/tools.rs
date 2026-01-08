use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::Serialize;
use sevenz_rust::SevenZReader;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const TOOLS_DIR_NAME: &str = "cslol-tools";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnsureModToolsResult {
  pub installed: bool,
  pub updated: bool,
  pub skipped: bool,
  pub version: Option<String>,
  pub latest_version: Option<String>,
  pub path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ToolsProgressPayload {
  phase: String,
  progress: f64,
  downloaded: Option<u64>,
  total: Option<u64>,
  speed: Option<f64>,
  message: Option<String>,
  version: Option<String>,
  error: Option<String>,
  source: String,
}

// No longer needed: ReleaseResponse, ReleaseAsset, ReleaseInfo

#[derive(Debug, Clone)]
struct LocalToolsPaths {
  app_data_dir: PathBuf,
  tools_dir: PathBuf,
}

static WARMUP_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static WARMUP_DONE: AtomicBool = AtomicBool::new(false);

fn emit_progress(_app: &tauri::AppHandle, _payload: ToolsProgressPayload) {
  // No-op: CSLOL tools progress events removed. Kept for compatibility.
}

fn build_http_client() -> Result<reqwest::Client, String> {
  reqwest::Client::builder()
    .user_agent("osskins-tools-installer/1.0 (+https://github.com/Abdelrhmanx74/osskins)")
    .timeout(Duration::from_secs(300))
    .connect_timeout(Duration::from_secs(10))
    .pool_max_idle_per_host(32)
    .pool_idle_timeout(Duration::from_secs(120))
    .http2_adaptive_window(true)
    .build()
    .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

async fn resolve_paths(_app: &tauri::AppHandle) -> Result<LocalToolsPaths, String> {
  // Prefer app data locations, then bundled resource directory, then exe sibling locations
  // Resolve app_data_dir where we can store config; fall back to resource dir when unavailable
  let app_data_dir = _app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| {
      // fallback to resource dir or current exe parent
      _app
        .path()
        .resource_dir()
        .unwrap_or_else(|_| std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("."))).to_path_buf()
    });

  // Candidate tools dirs
  let mut candidates = Vec::new();
  candidates.push(app_data_dir.join(TOOLS_DIR_NAME));
  if let Ok(app_local) = _app.path().app_local_data_dir() {
    candidates.push(app_local.join(TOOLS_DIR_NAME));
    candidates.push(app_local.join("mod-tools.exe"));
  }
  if let Ok(resource_dir) = _app.path().resource_dir() {
    candidates.push(resource_dir.join(TOOLS_DIR_NAME));
    candidates.push(resource_dir.join("mod-tools.exe"));
  }
  if let Ok(exe_path) = std::env::current_exe() {
    if let Some(exe_dir) = exe_path.parent() {
      candidates.push(exe_dir.join(TOOLS_DIR_NAME));
      candidates.push(exe_dir.join("resources").join(TOOLS_DIR_NAME));
      candidates.push(exe_dir.join("mod-tools.exe"));
    }
  }

  // Pick the first existing candidate, or default to resource/tools folder
  let tools_dir = candidates
    .into_iter()
    .find(|p| p.exists())
    .unwrap_or_else(|| {
      // fallback: resource dir cslol-tools or app_data_dir/cslol-tools
      if let Ok(resource_dir) = _app.path().resource_dir() {
        resource_dir.join(TOOLS_DIR_NAME)
      } else {
        app_data_dir.join(TOOLS_DIR_NAME)
      }
    });

  Ok(LocalToolsPaths {
    app_data_dir,
    tools_dir,
  })
}

async fn read_installed_version(paths: &LocalToolsPaths) -> Result<Option<String>, String> {
  // First try reading from config.json (new location)
  let config_file = paths.app_data_dir.join("config").join("config.json");
  if async_fs::metadata(&config_file).await.is_ok() {
    let data = async_fs::read_to_string(&config_file)
      .await
      .map_err(|e| format!("Failed to read config file: {}", e))?;
    if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
      if let Some(version) = cfg.get("cslol_tools_version").and_then(|v| v.as_str()) {
        return Ok(Some(version.to_string()));
      }
    }
  }
  
  // Fallback to old version file location for backwards compatibility
  let old_version_file = paths.app_data_dir.join("cslol-tools-version.txt");
  if async_fs::metadata(&old_version_file).await.is_ok() {
    let data = async_fs::read_to_string(&old_version_file)
      .await
      .map_err(|e| format!("Failed to read old version file: {}", e))?;
    let trimmed = data.trim();
    if !trimmed.is_empty() {
      return Ok(Some(trimmed.to_string()));
    }
  }
  
  Ok(None)
}

async fn mod_tools_exists(paths: &LocalToolsPaths) -> bool {
  async_fs::metadata(paths.tools_dir.join("mod-tools.exe"))
    .await
    .is_ok()
}

async fn ensure_mod_tools_internal(
  app: &tauri::AppHandle,
  _force_download: bool,
  _source: &str,
) -> Result<EnsureModToolsResult, String> {
  let paths = resolve_paths(app).await?;
  let installed = mod_tools_exists(&paths).await;
  let version = read_installed_version(&paths).await?;
  Ok(EnsureModToolsResult {
    installed,
    updated: false,
    skipped: false,
    version,
    latest_version: None,
    path: if installed {
      Some(paths.tools_dir.join("mod-tools.exe").to_string_lossy().to_string())
    } else {
      None
    },
  })
}

/// Try to pick a suitable asset from the release payload.
/// Preference order:
/// 1) explicit zip (cslol-manager.zip)
/// 2) any archive-like asset (zip, 7z, tar.gz, tar, msi)
/// 3) executable (.exe)
/// If none found, returns an error listing available assets.
// fetch_latest_release_info removed: no longer needed

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), io::Error> {
  if !dst.exists() {
    fs::create_dir_all(dst)?;
  }

  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    let target_path = dst.join(entry.file_name());

    if file_type.is_dir() {
      copy_dir_recursive(&entry.path(), &target_path)?;
    } else if file_type.is_file() {
      if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
      }
      fs::copy(entry.path(), &target_path)?;
    }
  }

  Ok(())
}

/// Downloads a file with progress tracking using reqwest streaming
async fn download_file_with_progress(
  client: &reqwest::Client,
  url: &str,
  file_path: &Path,
  total_size: u64,
  on_progress: impl Fn(u64, u64, f64),
) -> Result<u64, String> {
  let response = client
    .get(url)
    .send()
    .await
    .map_err(|e| format!("Failed to start download: {}", e))?;

  if !response.status().is_success() {
    return Err(format!(
      "Download failed with status: {}",
      response.status()
    ));
  }

  let content_length = response.content_length().unwrap_or(total_size);
  
  // Use buffered writer for faster I/O
  let file = async_fs::File::create(file_path)
    .await
    .map_err(|e| format!("Failed to create download file: {}", e))?;
  let mut file = tokio::io::BufWriter::with_capacity(256 * 1024, file);

  let mut downloaded: u64 = 0;
  let mut last_emit = Instant::now();
  let started_at = Instant::now();
  let mut stream = response.bytes_stream();

  while let Some(chunk) = stream.next().await {
    let bytes = chunk.map_err(|e| format!("Download stream error: {}", e))?;
    file
      .write_all(&bytes)
      .await
      .map_err(|e| format!("Failed to write download chunk: {}", e))?;

    downloaded += bytes.len() as u64;

    // Emit progress updates at most every 500ms to reduce overhead
    if last_emit.elapsed() >= Duration::from_millis(500) || downloaded >= content_length {
      let elapsed = started_at.elapsed().as_secs_f64();
      let speed = if elapsed > 0.0 {
        downloaded as f64 / elapsed
      } else {
        0.0
      };
      on_progress(downloaded, content_length, speed);
      last_emit = Instant::now();
    }
  }

  file
    .flush()
    .await
    .map_err(|e| format!("Failed to finalize download: {}", e))?;
  
  file
    .into_inner()
    .sync_all()
    .await
    .map_err(|e| format!("Failed to sync download: {}", e))?;

  Ok(downloaded)
}

/// Extract a 7z SFX archive (self-extracting .exe) using sevenz-rust.
/// 7z SFX files have a small executable header followed by a valid .7z stream.
/// We need to find the 7z signature within the file and extract from there.
/// NOTE: This only works for true 7z SFX archives created with 7z -sfx
fn try_extract_sfx_with_sevenz(archive_path: &Path, extract_path: &Path) -> Result<(), String> {
  use std::io::{Read, Seek, SeekFrom};
  
  let mut file = fs::File::open(archive_path)
    .map_err(|e| format!("Failed to open archive: {}", e))?;

  // 7z signature: '7' 'z' 0xBC 0xAF 0x27 0x1C
  const SEVENZ_SIGNATURE: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];
  
  // Search for 7z signature in the file
  let mut buffer = Vec::new();
  file.read_to_end(&mut buffer)
    .map_err(|e| format!("Failed to read file: {}", e))?;
  
  // Find the 7z signature
  let signature_offset = buffer
    .windows(SEVENZ_SIGNATURE.len())
    .position(|window| window == SEVENZ_SIGNATURE)
    .ok_or_else(|| {
      "Not a 7z SFX archive - could not find 7z signature. This appears to be a regular .exe installer.".to_string()
    })?;
  
  if signature_offset == 0 {
    // It's a regular 7z file, not SFX
    file.seek(SeekFrom::Start(0))
      .map_err(|e| format!("Failed to seek file: {}", e))?;
  } else {
    // It's an SFX, skip to the 7z data
    file.seek(SeekFrom::Start(signature_offset as u64))
      .map_err(|e| format!("Failed to seek to 7z data: {}", e))?;
  }

  // Get remaining file size from signature position
  let file_size = buffer.len() as u64 - signature_offset as u64;
  
  // Create a cursor over the 7z portion of the data
  let sevenz_data = &buffer[signature_offset..];
  let cursor = std::io::Cursor::new(sevenz_data);

  let mut sz = SevenZReader::new(cursor, file_size, sevenz_rust::Password::empty())
    .map_err(|e| format!("Failed to read 7z archive structure: {}", e))?;

  // Extract all entries
  sz.for_each_entries(|entry, reader| {
    // Skip directories
    if entry.is_directory() {
      return Ok(true);
    }

    let entry_path = extract_path.join(entry.name());

    // Create parent directories if needed
    if let Some(parent) = entry_path.parent() {
      fs::create_dir_all(parent)?;
    }

    // Extract the file
    let mut out_file = fs::File::create(&entry_path)?;
    io::copy(reader, &mut out_file)?;

    Ok(true)
  })
  .map_err(|e| format!("Failed during 7z extraction: {}", e))?;

  Ok(())
}

/// Try to extract an archive using multiple strategies:
/// 1) Attempt to open as ZIP using zip crate
/// Try to extract an archive using multiple strategies:
/// 1) ZIP files: Use zip crate
/// 2) 7z archives and 7z SFX (.exe): Use sevenz-rust with SFX detection
/// 3) External tools are OPTIONAL fallbacks only
/// Returns Err with a descriptive message if all methods fail.
fn try_extract_archive(archive_path: &Path, extract_path: &Path) -> Result<(), String> {
  let mut errors = Vec::new();
  let file_ext = archive_path
    .extension()
    .and_then(OsStr::to_str)
    .map(|s| s.to_lowercase())
    .unwrap_or_default();

  // 1) Try ZIP archive extraction via zip crate for .zip files
  if file_ext == "zip" {
    if let Ok(file) = fs::File::open(&archive_path) {
      match zip::ZipArchive::new(file) {
        Ok(mut archive) => {
          match archive.extract(&extract_path) {
            Ok(()) => return Ok(()),
            Err(e) => {
              errors.push(format!("ZIP extraction failed: {}", e));
            }
          }
        }
        Err(e) => {
          errors.push(format!("Not a valid ZIP archive: {}", e));
        }
      }
    } else {
      return Err(format!(
        "Failed to open downloaded archive at {}",
        archive_path.display()
      ));
    }
  }

  // 2) Try sevenz-rust for 7z archives and 7z SFX (.exe) files
  // Our implementation searches for the 7z signature within the file
  if file_ext == "7z" || file_ext == "exe" {
    match try_extract_sfx_with_sevenz(archive_path, extract_path) {
      Ok(()) => return Ok(()),
      Err(e) => {
        errors.push(format!("sevenz-rust extraction failed: {}", e));
      }
    }
  }

  // If we've successfully handled common formats, we should have returned by now
  // The following are OPTIONAL fallbacks only if user has these tools installed

  // 3) OPTIONAL: Try external 7z extraction (only if user has 7-Zip installed)
  // This is NOT required and is only a fallback
  match Command::new("7z")
    .arg("x")
    .arg(archive_path.as_os_str())
    .arg(format!("-o{}", extract_path.to_string_lossy()))
    .arg("-y")
    .stderr(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
  {
    Ok(mut cmd) => {
      match cmd.wait() {
        Ok(status) if status.success() => return Ok(()),
        Ok(status) => {
          errors.push(format!("External 7z (optional fallback) failed with exit code: {:?}", status.code()));
        }
        Err(e) => {
          errors.push(format!("External 7z (optional fallback) process error: {}", e));
        }
      }
    }
    Err(_) => {
      // 7z not installed - this is EXPECTED and OK
      errors.push("External 7z not available (not required)".to_string());
    }
  }

  // 4) OPTIONAL: Try tar (only for tar-based formats and if tar is available)
  // 4) OPTIONAL: Try tar (only for tar-based formats and if tar is available)
  if file_ext == "tar"
    || file_ext == "gz"
    || file_ext == "tgz"
    || archive_path.to_string_lossy().to_lowercase().ends_with(".tar.gz")
  {
    match Command::new("tar")
      .arg("-xf")
      .arg(archive_path.as_os_str())
      .arg("-C")
      .arg(extract_path.as_os_str())
      .stderr(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn()
    {
      Ok(mut cmd) => {
        match cmd.wait() {
          Ok(status) if status.success() => return Ok(()),
          Ok(status) => {
            errors.push(format!("tar (optional fallback) failed with exit code: {:?}", status.code()));
          }
          Err(e) => {
            errors.push(format!("tar (optional fallback) process error: {}", e));
          }
        }
      }
      Err(_) => {
        // tar not installed - this is EXPECTED and OK for non-tar files
        errors.push("tar not available (not required for .zip/.7z/.exe files)".to_string());
      }
    }
  }

  // All extraction methods failed - provide detailed error information
  Err(format!(
    "Failed to extract archive '{}' (extension: .{}). Built-in extractors should handle .zip, .7z, and .exe files without external tools.\n\nAttempted methods:\n{}",
    archive_path.display(),
    file_ext,
    errors.join("\n")
  ))
}

// download_and_install_tools removed: no longer needed

// get_cslol_manager_status removed: no longer needed

#[tauri::command]
pub async fn ensure_mod_tools(
  app: tauri::AppHandle,
  _force: Option<bool>,
) -> Result<EnsureModToolsResult, String> {
  ensure_mod_tools_internal(&app, false, "bundled").await
}

async fn warm_mod_tools_binary(mod_tools_path: &Path) -> Result<(), String> {
  let mod_tools_path = mod_tools_path.to_path_buf();

  tokio::task::spawn_blocking(move || -> Result<(), String> {
    let mut cmd = Command::new(&mod_tools_path);
    cmd.arg("--version");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    {
      use std::os::windows::process::CommandExt;
      cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd
      .status()
      .map(|_| ())
      .map_err(|e| format!("Failed to warm up mod-tools.exe: {}", e))
  })
  .await
  .map_err(|e| format!("Warmup task failed: {}", e))?
}

// warmup_mod_tools removed: no longer needed

/// Manual SFX extraction command that can be called from JavaScript.
/// Extracts a 7z SFX archive (.exe file) to the specified output directory.
#[tauri::command]
pub async fn extract_sfx(path: String, output: String) -> Result<(), String> {
  let archive_path = PathBuf::from(path);
  let output_path = PathBuf::from(output);

  // Run extraction in a blocking task since it's CPU/IO intensive
  tokio::task::spawn_blocking(move || {
    try_extract_sfx_with_sevenz(&archive_path, &output_path)
  })
  .await
  .map_err(|e| format!("Extraction task failed: {}", e))?
}
