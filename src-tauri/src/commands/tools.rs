use chrono::Utc;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sevenz_rust::SevenZReader;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

const RELEASE_API_URL: &str =
  "https://api.github.com/repos/LeagueToolkit/cslol-manager/releases/latest";
const PROGRESS_EVENT: &str = "cslol-tools-progress";
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CslolManagerStatus {
  pub installed: bool,
  pub version: Option<String>,
  pub latest_version: Option<String>,
  pub has_update: bool,
  pub path: Option<String>,
  pub download_size: Option<u64>,
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

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
  tag_name: String,
  assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
  name: String,
  browser_download_url: String,
  size: u64,
}

#[derive(Debug, Clone)]
struct ReleaseInfo {
  version: String,
  download_url: String,
  size: u64,
  asset_name: String,
}

#[derive(Debug, Clone)]
struct LocalToolsPaths {
  app_data_dir: PathBuf,
  tools_dir: PathBuf,
}

fn emit_progress(app: &tauri::AppHandle, payload: ToolsProgressPayload) {
  let _ = app.emit(PROGRESS_EVENT, payload);
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

async fn resolve_paths(app: &tauri::AppHandle) -> Result<LocalToolsPaths, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;

  let tools_dir = app_data_dir.join(TOOLS_DIR_NAME);

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

/// Try to pick a suitable asset from the release payload.
/// Preference order:
/// 1) explicit zip (cslol-manager.zip)
/// 2) any archive-like asset (zip, 7z, tar.gz, tar, msi)
/// 3) executable (.exe)
/// If none found, returns an error listing available assets.
async fn fetch_latest_release_info() -> Result<ReleaseInfo, String> {
  let client = build_http_client()?;
  let response = client
    .get(RELEASE_API_URL)
    .send()
    .await
    .map_err(|e| format!("Failed to fetch CSLOL Manager release info: {}", e))?;

  if response.status() == reqwest::StatusCode::FORBIDDEN {
    return Err("GitHub API rate limit reached. Please try again later.".into());
  }

  if !response.status().is_success() {
    return Err(format!(
      "GitHub API returned {} while fetching release info",
      response.status()
    ));
  }

  let payload: ReleaseResponse = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse release info: {}", e))?;

  // Helper closures
  let is_archive = |n: &str| {
    let l = n.to_lowercase();
    l.ends_with(".zip")
      || l.ends_with(".7z")
      || l.ends_with(".tar.gz")
      || l.ends_with(".tgz")
      || l.ends_with(".tar")
  };
  let is_sfx = |n: &str| n.to_lowercase().ends_with(".exe");

  // 1) Prefer explicit cslol-manager.zip if present
  if let Some(asset) = payload
    .assets
    .iter()
    .find(|asset| asset.name == "cslol-manager.zip")
  {
    return Ok(ReleaseInfo {
      version: payload.tag_name,
      download_url: asset.browser_download_url.clone(),
      size: asset.size,
      asset_name: asset.name.clone(),
    });
  }

  // 2) Try any ZIP/7z archive that looks like the manager package
  // Prefer regular archives over SFX
  if let Some(asset) = payload.assets.iter().find(|asset| {
    let name_matches = asset.name.to_lowercase().contains("cslol-manager")
      || asset.name.to_lowercase().contains("cslol");
    name_matches && is_archive(&asset.name)
  }) {
    return Ok(ReleaseInfo {
      version: payload.tag_name,
      download_url: asset.browser_download_url.clone(),
      size: asset.size,
      asset_name: asset.name.clone(),
    });
  }

  // 3) Fallback to 7z SFX (.exe) if no regular archives found
  if let Some(asset) = payload.assets.iter().find(|asset| {
    let name_matches = asset.name.to_lowercase().contains("cslol-manager")
      || asset.name.to_lowercase().contains("cslol");
    name_matches && is_sfx(&asset.name)
  }) {
    return Ok(ReleaseInfo {
      version: payload.tag_name,
      download_url: asset.browser_download_url.clone(),
      size: asset.size,
      asset_name: asset.name.clone(),
    });
  }

  // Nothing matched - build informative error
  let names = payload
    .assets
    .iter()
    .map(|a| a.name.clone())
    .collect::<Vec<_>>()
    .join(", ");
  Err(format!(
    "Could not find a suitable cslol-manager archive (.zip, .7z, or .exe SFX) in latest release. Available assets: {}",
    names
  ))
}

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

async fn download_and_install_tools(
  app: &tauri::AppHandle,
  paths: &LocalToolsPaths,
  release: &ReleaseInfo,
  source: &str,
) -> Result<(), String> {
  let temp_root = paths.app_data_dir.join("tmp");
  async_fs::create_dir_all(&temp_root)
    .await
    .map_err(|e| format!("Failed to create temporary directory: {}", e))?;

  let temp_dir = temp_root.join(format!("cslol-tools-{}", Utc::now().timestamp()));
  async_fs::create_dir_all(&temp_dir)
    .await
    .map_err(|e| format!("Failed to create temporary directory: {}", e))?;

  // Use the actual asset name for the downloaded file (preserves extension)
  let download_file = temp_dir.join(&release.asset_name);
  let extract_path = temp_dir.join("extracted");

  let client = build_http_client()?;

  // Download with progress tracking
  let version = release.version.clone();
  let source_str = source.to_string();
  let app_handle = app.clone();
  let downloaded = download_file_with_progress(
    &client,
    &release.download_url,
    &download_file,
    release.size,
    move |downloaded, total, speed| {
      let progress = if total > 0 {
        (downloaded as f64 / total as f64) * 100.0
      } else {
        0.0
      };
      emit_progress(
        &app_handle,
        ToolsProgressPayload {
          phase: "downloading".to_string(),
          progress,
          downloaded: Some(downloaded),
          total: Some(total),
          speed: Some(speed),
          message: None,
          version: Some(version.clone()),
          error: None,
          source: source_str.clone(),
        },
      );
    },
  )
  .await?;

  emit_progress(
    app,
    ToolsProgressPayload {
      phase: "installing".to_string(),
      progress: 95.0,
      downloaded: Some(downloaded),
      total: Some(release.size),
      speed: None,
      message: None,
      version: Some(release.version.clone()),
      error: None,
      source: source.to_string(),
    },
  );

  // Attempt extraction using multiple strategies. This runs in blocking thread because extraction can be CPU/IO heavy.
  let download_file_clone = download_file.clone();
  let extract_path_clone = extract_path.clone();
  tokio::task::spawn_blocking(move || -> Result<(), String> {
    // Ensure extract path exists
    if let Err(e) = fs::create_dir_all(&extract_path_clone) {
      return Err(format!(
        "Failed to create extract directory {}: {}",
        extract_path_clone.display(),
        e
      ));
    }

    // try_extract_archive now handles: ZIP, 7z/SFX (.exe), external 7z, and tar
    try_extract_archive(&download_file_clone, &extract_path_clone)
      .map_err(|e| format!("Extraction failed: {}", e))
  })
  .await
  .map_err(|e| format!("Extraction task failed: {}", e))??;

  // After extraction, look for the tools folder in common locations
  let mut candidates = vec![extract_path.join("cslol-manager").join(TOOLS_DIR_NAME)];
  candidates.push(extract_path.join(TOOLS_DIR_NAME));

  // Some self-extracting exes may produce a nested folder or produce contents at root
  // add additional candidates: root of extracted, and any first-level directory that contains the TOOLS_DIR_NAME
  candidates.push(extract_path.clone());
  if let Ok(read_dir) = fs::read_dir(&extract_path) {
    if let Ok(iter) = read_dir.into_iter().collect::<Result<Vec<_>, _>>() {
      for entry in iter {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
          candidates.push(entry.path().join(TOOLS_DIR_NAME));
          candidates.push(entry.path());
        }
      }
    }
  }

  let tools_source = candidates
    .into_iter()
    .find(|path| path.exists())
    .ok_or_else(|| "Extracted archive does not contain cslol-tools folder".to_string())?;

  if async_fs::metadata(&paths.tools_dir).await.is_ok() {
    async_fs::remove_dir_all(&paths.tools_dir)
      .await
      .map_err(|e| format!("Failed to remove existing CSLOL tools: {}", e))?;
  }

  let tools_dir_clone = paths.tools_dir.clone();
  let tools_source_clone = tools_source.clone();
  tokio::task::spawn_blocking(move || -> Result<(), String> {
    copy_dir_recursive(&tools_source_clone, &tools_dir_clone)
      .map_err(|e| format!("Failed to install CSLOL tools: {}", e))
  })
  .await
  .map_err(|e| format!("Install task failed: {}", e))??;

  if async_fs::metadata(paths.tools_dir.join("mod-tools.exe"))
    .await
    .is_err()
  {
    return Err("Installed CSLOL tools are missing mod-tools.exe".into());
  }

  // Write version to config.json
  let config_dir = paths.app_data_dir.join("config");
  async_fs::create_dir_all(&config_dir)
    .await
    .map_err(|e| format!("Failed to create config dir: {}", e))?;
  let config_file = config_dir.join("config.json");
  
  let mut cfg: serde_json::Value = if async_fs::metadata(&config_file).await.is_ok() {
    let content = async_fs::read_to_string(&config_file)
      .await
      .map_err(|e| format!("Failed to read config: {}", e))?;
    serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
  } else {
    serde_json::json!({})
  };
  
  cfg["cslol_tools_version"] = serde_json::json!(release.version.clone());
  
  let data = serde_json::to_string_pretty(&cfg)
    .map_err(|e| format!("Failed to serialize config: {}", e))?;
  async_fs::write(&config_file, data)
    .await
    .map_err(|e| format!("Failed to write config: {}", e))?;

  // Clean up temporary directory (best effort)
  let _ = async_fs::remove_dir_all(&temp_dir).await;

  Ok(())
}

#[tauri::command]
pub async fn get_cslol_manager_status(app: tauri::AppHandle) -> Result<CslolManagerStatus, String> {
  let paths = resolve_paths(&app).await?;
  let installed = mod_tools_exists(&paths).await;
  let version = read_installed_version(&paths).await?;
  let release_info = fetch_latest_release_info().await?;

  let has_update = if installed {
    match &version {
      Some(current) => current != &release_info.version,
      None => true,
    }
  } else {
    true
  };

  Ok(CslolManagerStatus {
    installed,
    version,
    latest_version: Some(release_info.version),
    has_update,
    path: if installed {
      Some(
        paths
          .tools_dir
          .join("mod-tools.exe")
          .to_string_lossy()
          .to_string(),
      )
    } else {
      None
    },
    download_size: Some(release_info.size),
  })
}

#[tauri::command]
pub async fn ensure_mod_tools(
  app: tauri::AppHandle,
  force: Option<bool>,
) -> Result<EnsureModToolsResult, String> {
  let force_download = force.unwrap_or(false);
  let source = if force_download { "manual" } else { "auto" };

  emit_progress(
    &app,
    ToolsProgressPayload {
      phase: "checking".to_string(),
      progress: 0.0,
      downloaded: None,
      total: None,
      speed: None,
      message: None,
      version: None,
      error: None,
      source: source.to_string(),
    },
  );

  let result: Result<EnsureModToolsResult, String> = async {
    let paths = resolve_paths(&app).await?;
    let release_info = fetch_latest_release_info().await?;
    let installed = mod_tools_exists(&paths).await;
    let current_version = read_installed_version(&paths).await?;

    let up_to_date = installed
      && !force_download
      && current_version
        .as_ref()
        .map(|v| v == &release_info.version)
        .unwrap_or(false);

    if up_to_date {
      Ok(EnsureModToolsResult {
        installed: true,
        updated: false,
        skipped: true,
        version: current_version.clone(),
        latest_version: Some(release_info.version.clone()),
        path: Some(
          paths
            .tools_dir
            .join("mod-tools.exe")
            .to_string_lossy()
            .to_string(),
        ),
      })
    } else {
      download_and_install_tools(&app, &paths, &release_info, source).await?;
      Ok(EnsureModToolsResult {
        installed: true,
        updated: true,
        skipped: false,
        version: Some(release_info.version.clone()),
        latest_version: Some(release_info.version.clone()),
        path: Some(
          paths
            .tools_dir
            .join("mod-tools.exe")
            .to_string_lossy()
            .to_string(),
        ),
      })
    }
  }
  .await;

  match &result {
    Ok(ensure) if ensure.skipped => {
      emit_progress(
        &app,
        ToolsProgressPayload {
          phase: "skipped".to_string(),
          progress: 100.0,
          downloaded: None,
          total: None,
          speed: None,
          message: None,
          version: ensure.version.clone(),
          error: None,
          source: source.to_string(),
        },
      );
    }
    Ok(ensure) => {
      emit_progress(
        &app,
        ToolsProgressPayload {
          phase: "completed".to_string(),
          progress: 100.0,
          downloaded: None,
          total: None,
          speed: None,
          message: None,
          version: ensure.version.clone(),
          error: None,
          source: source.to_string(),
        },
      );
    }
    Err(err) => {
      emit_progress(
        &app,
        ToolsProgressPayload {
          phase: "error".to_string(),
          progress: 0.0,
          downloaded: None,
          total: None,
          speed: None,
          message: None,
          version: None,
          error: Some(err.clone()),
          source: source.to_string(),
        },
      );
    }
  }

  result
}

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
