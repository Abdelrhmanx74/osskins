use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const EVENT_NAME: &str = "download-progress";
const BATCH_EVENT_NAME: &str = "batch-download-progress";

// New LeagueSkins repo base URL
pub const LEAGUE_SKINS_RAW_BASE: &str =
  "https://raw.githubusercontent.com/Alban1911/LeagueSkins/main";

// Maximum concurrent downloads for batch operations - high for GitHub CDN
const MAX_CONCURRENT_DOWNLOADS: usize = 16;
// Buffer size for file writing (1MB) - larger buffer = fewer syscalls
const WRITE_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Clone)]
struct DownloadCtrl {
  cancel: CancellationToken,
}

static TASKS: Lazy<Arc<Mutex<HashMap<String, DownloadCtrl>>>> =
  Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Semaphore for limiting concurrent downloads
static DOWNLOAD_SEMAPHORE: Lazy<Arc<Semaphore>> =
  Lazy::new(|| Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)));

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadProgressPayload {
  id: String,
  status: String, // queued | downloading | completed | failed | canceled
  url: String,
  category: String, // skin | data | tools | misc
  downloaded: Option<u64>,
  total: Option<u64>,
  speed: Option<f64>, // bytes/sec
  champion_name: Option<String>,
  file_name: Option<String>,
  dest_path: Option<String>,
  error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchDownloadProgress {
  pub batch_id: String,
  pub total_items: usize,
  pub completed_items: usize,
  pub failed_items: usize,
  pub current_items: Vec<String>, // Currently downloading item IDs
  pub total_bytes: u64,
  pub downloaded_bytes: u64,
  pub speed: f64,
  pub status: String, // "downloading" | "completed" | "failed" | "canceled"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinDownloadRequest {
  pub champion_id: u32,
  pub skin_id: u32,
  pub chroma_id: Option<u32>,
  pub form_id: Option<u32>,
  pub champion_name: String,
  pub file_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDownloadResult {
  pub batch_id: String,
  pub successful: Vec<String>,
  pub failed: Vec<(String, String)>, // (item_id, error)
  pub total_bytes: u64,
  pub elapsed_secs: f64,
}

fn emit(app: &tauri::AppHandle, payload: DownloadProgressPayload) {
  let _ = app.emit(EVENT_NAME, payload);
}

fn emit_batch(app: &tauri::AppHandle, payload: BatchDownloadProgress) {
  let _ = app.emit(BATCH_EVENT_NAME, payload);
}

static HTTP_CLIENT: Lazy<Arc<reqwest::Client>> = Lazy::new(|| {
  Arc::new(
    reqwest::Client::builder()
      .user_agent("osskins-downloader/3.0")
      .timeout(Duration::from_secs(600))
      .connect_timeout(Duration::from_secs(5)) // Faster connection timeout
      .tcp_keepalive(Duration::from_secs(30))
      .tcp_nodelay(true) // Disable Nagle's algorithm for lower latency
      .pool_max_idle_per_host(128) // Large pool for parallel downloads
      .pool_idle_timeout(Duration::from_secs(90))
      .http2_adaptive_window(true)
      .http2_initial_stream_window_size(Some(2 * 1024 * 1024)) // 2MB initial window
      .http2_initial_connection_window_size(Some(4 * 1024 * 1024)) // 4MB connection window
      .http2_keep_alive_interval(Duration::from_secs(20))
      .http2_keep_alive_timeout(Duration::from_secs(10))
      .http2_max_frame_size(Some(32 * 1024)) // 32KB frames
      // Compression is enabled by default in reqwest
      .build()
      .expect("Failed to build HTTP client"),
  )
});

async fn champions_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data directory: {}", e))?;
  Ok(app_data_dir.join("champions"))
}

/// Starts a download to the champions/<champion>/<file_name> path and emits unified progress.
/// The command is async and runs the download in-process, emitting progress along the way.
#[tauri::command]
pub async fn download_file_to_champion_with_progress(
  app: tauri::AppHandle,
  url: String,
  champion_name: String,
  file_name: String,
) -> Result<String, String> {
  let id = Uuid::new_v4().to_string();
  let category = "skin".to_string();

  emit(
    &app,
    DownloadProgressPayload {
      id: id.clone(),
      status: "queued".into(),
      url: url.clone(),
      category: category.clone(),
      downloaded: None,
      total: None,
      speed: None,
      champion_name: Some(champion_name.clone()),
      file_name: Some(file_name.clone()),
      dest_path: None,
      error: None,
    },
  );

  let cancel = CancellationToken::new();
  TASKS.lock().await.insert(
    id.clone(),
    DownloadCtrl {
      cancel: cancel.clone(),
    },
  );

  let client = HTTP_CLIENT.clone();
  let app_handle = app.clone();
  let id_clone = id.clone();

  // Prepare destination
  let champions_root = champions_dir(&app).await?;
  let champion_dir = champions_root.join(&champion_name);
  async_fs::create_dir_all(&champion_dir)
    .await
    .map_err(|e| format!("Failed to create champion directory: {}", e))?;
  let file_path = champion_dir.join(&file_name);

  // Perform the download
  let result = async {
    let response = client
      .get(&url)
      .send()
      .await
      .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
      return Err(format!(
        "Download failed with status: {}",
        response.status()
      ));
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    
    // Use 1MB buffer for better I/O performance
    let file = async_fs::File::create(&file_path)
      .await
      .map_err(|e| format!("Failed to create destination file: {}", e))?;
    let mut file = tokio::io::BufWriter::with_capacity(WRITE_BUFFER_SIZE, file);

    let mut downloaded: u64 = 0;
    let started = Instant::now();
    let mut last_emit = Instant::now();

    emit(
      &app_handle,
      DownloadProgressPayload {
        id: id_clone.clone(),
        status: "downloading".into(),
        url: url.clone(),
        category: category.clone(),
        downloaded: Some(0),
        total,
        speed: Some(0.0),
        champion_name: Some(champion_name.clone()),
        file_name: Some(file_name.clone()),
        dest_path: Some(file_path.to_string_lossy().to_string()),
        error: None,
      },
    );

    loop {
      tokio::select! {
        _ = cancel.cancelled() => {
          return Err("canceled".into());
        }
        maybe_chunk = stream.next() => {
          match maybe_chunk {
            Some(chunk) => {
              let bytes = chunk.map_err(|e| format!("Download stream error: {}", e))?;
              file
                .write_all(&bytes)
                .await
                .map_err(|e| format!("Failed to write chunk: {}", e))?;

              downloaded += bytes.len() as u64;

              // Emit progress every 250ms for smooth UI updates
              if last_emit.elapsed() >= Duration::from_millis(250) {
                let elapsed = started.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };
                emit(
                  &app_handle,
                  DownloadProgressPayload {
                    id: id_clone.clone(),
                    status: "downloading".into(),
                    url: url.clone(),
                    category: category.clone(),
                    downloaded: Some(downloaded),
                    total,
                    speed: Some(speed),
                    champion_name: Some(champion_name.clone()),
                    file_name: Some(file_name.clone()),
                      dest_path: Some(file_path.to_string_lossy().to_string()),
                    error: None,
                  }
                );
                last_emit = Instant::now();
              }
            }
            None => break,
          }
        }
      }
    }

    file
      .flush()
      .await
      .map_err(|e| format!("Failed to finalize file: {}", e))?;
    
    // Skip sync_all for speed - OS will handle eventual sync
    // This significantly improves download performance

    Ok::<(), String>(())
  }
  .await;

  // Emit final event and cleanup
  match result {
    Ok(()) => {
      emit(
        &app,
        DownloadProgressPayload {
          id: id.clone(),
          status: "completed".into(),
          url,
          category,
          downloaded: None,
          total: None,
          speed: None,
          champion_name: Some(champion_name),
          file_name: Some(file_name),
          dest_path: Some(file_path.to_string_lossy().to_string()),
          error: None,
        },
      );
      TASKS.lock().await.remove(&id);
      Ok(id)
    }
    Err(err) if err == "canceled" => {
      // remove partial file if present
      let _ = async_fs::remove_file(&file_path).await;
      emit(
        &app,
        DownloadProgressPayload {
          id: id.clone(),
          status: "canceled".into(),
          url,
          category,
          downloaded: None,
          total: None,
          speed: None,
          champion_name: Some(champion_name),
          file_name: Some(file_name),
          dest_path: Some(file_path.to_string_lossy().to_string()),
          error: None,
        },
      );
      TASKS.lock().await.remove(&id);
      Err("Download canceled".into())
    }
    Err(err) => {
      // remove partial file if present
      let _ = async_fs::remove_file(&file_path).await;
      emit(
        &app,
        DownloadProgressPayload {
          id: id.clone(),
          status: "failed".into(),
          url,
          category,
          downloaded: None,
          total: None,
          speed: None,
          champion_name: Some(champion_name),
          file_name: Some(file_name),
          dest_path: Some(file_path.to_string_lossy().to_string()),
          error: Some(err.clone()),
        },
      );
      TASKS.lock().await.remove(&id);
      Err(err)
    }
  }
}

#[tauri::command]
pub async fn cancel_download(id: String) -> Result<bool, String> {
  let tasks = TASKS.clone();
  let map = tasks.lock().await;
  if let Some(ctrl) = map.get(&id) {
    ctrl.cancel.cancel();
    Ok(true)
  } else {
    Ok(false)
  }
}

/// Build the download URL for a skin from LeagueSkins repo
/// Structure: skins/{champion_id}/{skin_id}/{skin_id}.zip
/// For chromas: skins/{champion_id}/{skin_id}/{chroma_id}/{chroma_id}.zip
/// For forms: skins/{champion_id}/{skin_id}/{form_id}/{form_id}.zip
pub fn build_skin_download_url(
  champion_id: u32,
  skin_id: u32,
  chroma_id: Option<u32>,
  form_id: Option<u32>,
) -> String {
  let base = format!("{}/skins/{}/{}", LEAGUE_SKINS_RAW_BASE, champion_id, skin_id);

  if let Some(chroma) = chroma_id {
    format!("{}/{}/{}.zip", base, chroma, chroma)
  } else if let Some(form) = form_id {
    format!("{}/{}/{}.zip", base, form, form)
  } else {
    format!("{}/{}.zip", base, skin_id)
  }
}

/// Download a single skin using the new LeagueSkins repo structure
#[tauri::command]
pub async fn download_skin_by_id(
  app: tauri::AppHandle,
  champion_id: u32,
  skin_id: u32,
  chroma_id: Option<u32>,
  form_id: Option<u32>,
  champion_name: String,
  file_name: String,
) -> Result<String, String> {
  let url = build_skin_download_url(champion_id, skin_id, chroma_id, form_id);
  download_file_to_champion_with_progress(app, url, champion_name, file_name).await
}

/// High-performance batch download for multiple skins
/// Uses parallel downloads with semaphore-controlled concurrency
#[tauri::command]
pub async fn batch_download_skins(
  app: tauri::AppHandle,
  requests: Vec<SkinDownloadRequest>,
) -> Result<BatchDownloadResult, String> {
  let batch_id = Uuid::new_v4().to_string();
  let total_items = requests.len();
  let started = Instant::now();

  // Shared state for progress tracking
  let completed_count = Arc::new(AtomicU64::new(0));
  let failed_count = Arc::new(AtomicU64::new(0));
  let downloaded_bytes = Arc::new(AtomicU64::new(0));
  let total_bytes = Arc::new(AtomicU64::new(0));

  // Cancel token for the entire batch
  let batch_cancel = CancellationToken::new();
  TASKS.lock().await.insert(
    batch_id.clone(),
    DownloadCtrl {
      cancel: batch_cancel.clone(),
    },
  );

  // Emit initial progress
  emit_batch(
    &app,
    BatchDownloadProgress {
      batch_id: batch_id.clone(),
      total_items,
      completed_items: 0,
      failed_items: 0,
      current_items: vec![],
      total_bytes: 0,
      downloaded_bytes: 0,
      speed: 0.0,
      status: "downloading".into(),
    },
  );

  let champions_root = champions_dir(&app).await?;
  let client = HTTP_CLIENT.clone();

  // Create download tasks
  let mut handles = Vec::new();
  let successful = Arc::new(Mutex::new(Vec::new()));
  let failed = Arc::new(Mutex::new(Vec::new()));

  for req in requests {
    let app_handle = app.clone();
    let batch_id_clone = batch_id.clone();
    let client_clone = client.clone();
    let champions_root_clone = champions_root.clone();
    let batch_cancel_clone = batch_cancel.clone();
    let completed_count_clone = completed_count.clone();
    let failed_count_clone = failed_count.clone();
    let downloaded_bytes_clone = downloaded_bytes.clone();
    let total_bytes_clone = total_bytes.clone();
    let successful_clone = successful.clone();
    let failed_clone = failed.clone();
    let semaphore = DOWNLOAD_SEMAPHORE.clone();
    let started_clone = started;

    let handle = tokio::spawn(async move {
      // Acquire semaphore permit to limit concurrency
      let _permit = semaphore.acquire().await.unwrap();

      if batch_cancel_clone.is_cancelled() {
        return;
      }

      let url = build_skin_download_url(req.champion_id, req.skin_id, req.chroma_id, req.form_id);
      let item_id = format!(
        "{}_{}_{}",
        req.champion_id,
        req.skin_id,
        req.chroma_id.or(req.form_id).unwrap_or(0)
      );

      let champion_dir = champions_root_clone.join(&req.champion_name);
      if async_fs::create_dir_all(&champion_dir).await.is_err() {
        failed_count_clone.fetch_add(1, Ordering::Relaxed);
        failed_clone
          .lock()
          .await
          .push((item_id, "Failed to create directory".to_string()));
        return;
      }

      let file_path = champion_dir.join(&req.file_name);

      // Perform download with progress tracking
      match download_with_progress(
        client_clone,
        &url,
        &file_path,
        batch_cancel_clone.clone(),
        downloaded_bytes_clone.clone(),
        total_bytes_clone.clone(),
      )
      .await
      {
        Ok(_bytes) => {
          completed_count_clone.fetch_add(1, Ordering::Relaxed);
          successful_clone.lock().await.push(item_id.clone());

          // Emit progress update
          let completed = completed_count_clone.load(Ordering::Relaxed) as usize;
          let failed = failed_count_clone.load(Ordering::Relaxed) as usize;
          let total_dl = downloaded_bytes_clone.load(Ordering::Relaxed);
          let total_size = total_bytes_clone.load(Ordering::Relaxed);
          let elapsed = started_clone.elapsed().as_secs_f64();
          let speed = if elapsed > 0.0 {
            total_dl as f64 / elapsed
          } else {
            0.0
          };

          emit_batch(
            &app_handle,
            BatchDownloadProgress {
              batch_id: batch_id_clone.clone(),
              total_items,
              completed_items: completed,
              failed_items: failed,
              current_items: vec![],
              total_bytes: total_size,
              downloaded_bytes: total_dl,
              speed,
              status: "downloading".into(),
            },
          );
        }
        Err(e) => {
          failed_count_clone.fetch_add(1, Ordering::Relaxed);
          failed_clone.lock().await.push((item_id, e));
        }
      }
    });

    handles.push(handle);
  }

  // Wait for all downloads to complete
  for handle in handles {
    let _ = handle.await;
  }

  let elapsed = started.elapsed().as_secs_f64();
  let total_dl = downloaded_bytes.load(Ordering::Relaxed);
  let successful_items = successful.lock().await.clone();
  let failed_items = failed.lock().await.clone();

  // Emit final status
  let status = if batch_cancel.is_cancelled() {
    "canceled"
  } else if failed_items.is_empty() {
    "completed"
  } else if successful_items.is_empty() {
    "failed"
  } else {
    "completed"
  };

  emit_batch(
    &app,
    BatchDownloadProgress {
      batch_id: batch_id.clone(),
      total_items,
      completed_items: successful_items.len(),
      failed_items: failed_items.len(),
      current_items: vec![],
      total_bytes: total_dl,
      downloaded_bytes: total_dl,
      speed: 0.0,
      status: status.into(),
    },
  );

  TASKS.lock().await.remove(&batch_id);

  Ok(BatchDownloadResult {
    batch_id,
    successful: successful_items,
    failed: failed_items,
    total_bytes: total_dl,
    elapsed_secs: elapsed,
  })
}

/// Internal download function with progress tracking for batch downloads
/// Optimized for maximum throughput with large buffers and minimal syscalls
async fn download_with_progress(
  client: Arc<reqwest::Client>,
  url: &str,
  file_path: &PathBuf,
  cancel: CancellationToken,
  downloaded_bytes: Arc<AtomicU64>,
  total_bytes: Arc<AtomicU64>,
) -> Result<u64, String> {
  let response = client
    .get(url)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;

  if !response.status().is_success() {
    return Err(format!("HTTP {}", response.status()));
  }

  let content_length = response.content_length().unwrap_or(0);
  total_bytes.fetch_add(content_length, Ordering::Relaxed);

  let mut stream = response.bytes_stream();
  let file = async_fs::File::create(file_path)
    .await
    .map_err(|e| format!("Failed to create file: {}", e))?;
  // Use 1MB buffer for better I/O throughput
  let mut file = tokio::io::BufWriter::with_capacity(WRITE_BUFFER_SIZE, file);

  let mut item_downloaded: u64 = 0;
  // Accumulator to batch atomic updates - reduces contention
  let mut pending_bytes: u64 = 0;
  const BATCH_THRESHOLD: u64 = 256 * 1024; // Update every 256KB

  loop {
    tokio::select! {
      biased; // Prioritize cancel check
      _ = cancel.cancelled() => {
        drop(file); // Drop writer before removing file
        let _ = async_fs::remove_file(file_path).await;
        return Err("Canceled".into());
      }
      maybe_chunk = stream.next() => {
        match maybe_chunk {
          Some(Ok(bytes)) => {
            file.write_all(&bytes).await.map_err(|e| format!("Write error: {}", e))?;
            let len = bytes.len() as u64;
            item_downloaded += len;
            pending_bytes += len;
            
            // Batch atomic updates to reduce contention
            if pending_bytes >= BATCH_THRESHOLD {
              downloaded_bytes.fetch_add(pending_bytes, Ordering::Relaxed);
              pending_bytes = 0;
            }
          }
          Some(Err(e)) => {
            drop(file);
            let _ = async_fs::remove_file(file_path).await;
            return Err(format!("Stream error: {}", e));
          }
          None => break,
        }
      }
    }
  }

  // Flush remaining pending bytes
  if pending_bytes > 0 {
    downloaded_bytes.fetch_add(pending_bytes, Ordering::Relaxed);
  }

  file.flush().await.map_err(|e| format!("Flush error: {}", e))?;
  // Skip sync_all for speed - OS will handle eventual sync
  // file.into_inner().sync_all().await.map_err(|e| format!("Sync error: {}", e))?;

  Ok(item_downloaded)
}

/// Check if a skin exists in the LeagueSkins repo
#[tauri::command]
pub async fn check_skin_exists(
  champion_id: u32,
  skin_id: u32,
  chroma_id: Option<u32>,
  form_id: Option<u32>,
) -> Result<bool, String> {
  let url = build_skin_download_url(champion_id, skin_id, chroma_id, form_id);
  let client = HTTP_CLIENT.clone();

  let response = client
    .head(&url)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;

  Ok(response.status().is_success())
}

/// Get file size of a skin before downloading
#[tauri::command]
pub async fn get_skin_file_size(
  champion_id: u32,
  skin_id: u32,
  chroma_id: Option<u32>,
  form_id: Option<u32>,
) -> Result<Option<u64>, String> {
  let url = build_skin_download_url(champion_id, skin_id, chroma_id, form_id);
  let client = HTTP_CLIENT.clone();

  let response = client
    .head(&url)
    .send()
    .await
    .map_err(|e| format!("Request failed: {}", e))?;

  if !response.status().is_success() {
    return Ok(None);
  }

  Ok(response.content_length())
}

/// Cancel a batch download
#[tauri::command]
pub async fn cancel_batch_download(batch_id: String) -> Result<bool, String> {
  cancel_download(batch_id).await
}
