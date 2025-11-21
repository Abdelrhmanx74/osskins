use chrono::Utc;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

const RELEASE_API_URL: &str = "https://api.github.com/repos/LeagueToolkit/cslol-manager/releases/latest";
const PROGRESS_EVENT: &str = "cslol-tools-progress";
const TOOLS_DIR_NAME: &str = "cslol-tools";
const VERSION_FILE_NAME: &str = "cslol-tools-version.txt";

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
}

#[derive(Debug, Clone)]
struct LocalToolsPaths {
  app_data_dir: PathBuf,
  tools_dir: PathBuf,
  version_file: PathBuf,
}

fn emit_progress(app: &tauri::AppHandle, payload: ToolsProgressPayload) {
  let _ = app.emit(PROGRESS_EVENT, payload);
}

fn build_http_client() -> Result<reqwest::Client, String> {
  reqwest::Client::builder()
    .user_agent("osskins-tools-installer/1.0 (+https://github.com/Abdelrhmanx74/osskins)")
    .timeout(Duration::from_secs(300))
    .build()
    .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

async fn resolve_paths(app: &tauri::AppHandle) -> Result<LocalToolsPaths, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;

  let tools_dir = app_data_dir.join(TOOLS_DIR_NAME);
  let version_file = app_data_dir.join(VERSION_FILE_NAME);

  Ok(LocalToolsPaths {
    app_data_dir,
    tools_dir,
    version_file,
  })
}

async fn read_installed_version(paths: &LocalToolsPaths) -> Result<Option<String>, String> {
  if async_fs::metadata(&paths.version_file).await.is_err() {
    return Ok(None);
  }

  let data = async_fs::read_to_string(&paths.version_file)
    .await
    .map_err(|e| format!("Failed to read tools version file: {}", e))?;
  let trimmed = data.trim();
  if trimmed.is_empty() {
    Ok(None)
  } else {
    Ok(Some(trimmed.to_string()))
  }
}

async fn mod_tools_exists(paths: &LocalToolsPaths) -> bool {
  async_fs::metadata(paths.tools_dir.join("mod-tools.exe")).await.is_ok()
}

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

  let asset = payload
    .assets
    .iter()
    .find(|asset| asset.name == "cslol-manager.zip")
    .ok_or_else(|| "Could not find cslol-manager.zip in latest release".to_string())?;

  Ok(ReleaseInfo {
    version: payload.tag_name,
    download_url: asset.browser_download_url.clone(),
    size: asset.size,
  })
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
    return Err(format!("Download failed with status: {}", response.status()));
  }

  let content_length = response.content_length().unwrap_or(total_size);
  let mut file = async_fs::File::create(file_path)
    .await
    .map_err(|e| format!("Failed to create download file: {}", e))?;

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

    // Emit progress updates at most every 250ms or when download completes
    if last_emit.elapsed() >= Duration::from_millis(250) || downloaded >= content_length {
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

  Ok(downloaded)
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

  let zip_path = temp_dir.join("cslol-manager.zip");
  let extract_path = temp_dir.join("extracted");

  let client = build_http_client()?;
  
  // Download with progress tracking
  let version = release.version.clone();
  let source_str = source.to_string();
  let app_handle = app.clone();
  let downloaded = download_file_with_progress(
    &client,
    &release.download_url,
    &zip_path,
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

  let zip_path_clone = zip_path.clone();
  let extract_path_clone = extract_path.clone();
  tokio::task::spawn_blocking(move || -> Result<(), String> {
    let file = fs::File::open(&zip_path_clone)
      .map_err(|e| format!("Failed to open downloaded archive: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
      .map_err(|e| format!("Failed to read ZIP archive: {}", e))?;
    archive
      .extract(&extract_path_clone)
      .map_err(|e| format!("Failed to extract CSLOL Manager archive: {}", e))?;
    Ok(())
  })
  .await
  .map_err(|e| format!("Extraction task failed: {}", e))??;

  let mut candidates = vec![extract_path.join("cslol-manager").join(TOOLS_DIR_NAME)];
  candidates.push(extract_path.join(TOOLS_DIR_NAME));

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

  if async_fs::metadata(paths.tools_dir.join("mod-tools.exe")).await.is_err() {
    return Err("Installed CSLOL tools are missing mod-tools.exe".into());
  }

  async_fs::write(&paths.version_file, release.version.as_bytes())
    .await
    .map_err(|e| format!("Failed to write CSLOL tools version file: {}", e))?;

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
      Some(paths.tools_dir.join("mod-tools.exe").to_string_lossy().to_string())
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
        path: Some(paths.tools_dir.join("mod-tools.exe").to_string_lossy().to_string()),
      })
    } else {
      download_and_install_tools(&app, &paths, &release_info, source).await?;
      Ok(EnsureModToolsResult {
        installed: true,
        updated: true,
        skipped: false,
        version: Some(release_info.version.clone()),
        latest_version: Some(release_info.version.clone()),
        path: Some(paths.tools_dir.join("mod-tools.exe").to_string_lossy().to_string()),
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
