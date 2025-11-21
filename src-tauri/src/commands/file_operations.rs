use futures_util::StreamExt;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

// File operations for skin_file and ZIP files

// Reusable HTTP client with connection pooling for better performance
// This reduces connection overhead and improves download speed
static HTTP_CLIENT: Lazy<Arc<reqwest::Client>> = Lazy::new(|| {
  Arc::new(
    reqwest::Client::builder()
      .user_agent("osskins-downloader/1.0")
      .timeout(Duration::from_secs(300)) // 5 minute timeout for large files
      .tcp_keepalive(Duration::from_secs(60)) // Keep connections alive
      .pool_max_idle_per_host(10) // Connection pooling - reuse connections
      .pool_idle_timeout(Duration::from_secs(90)) // Reuse connections for 90s
      .build()
      .expect("Failed to build HTTP client"),
  )
});

#[tauri::command]
/// Save a skin ZIP file to the champions directory
pub async fn save_zip_file(
  app: tauri::AppHandle,
  champion_name: String,
  file_name: String,
  content: Vec<u8>,
) -> Result<(), String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data directory: {}", e))?;

  let champions_dir = app_data_dir.join("champions");
  std::fs::create_dir_all(&champions_dir)
    .map_err(|e| format!("Failed to create champions directory: {}", e))?;

  let champion_dir = champions_dir.join(&champion_name);
  std::fs::create_dir_all(&champion_dir)
    .map_err(|e| format!("Failed to create champion directory: {}", e))?;

  let zip_path = champion_dir.join(&file_name);
  if let Some(parent) = zip_path.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| format!("Failed to create parent directory: {}", e))?;
  }

  std::fs::write(&zip_path, &content).map_err(|e| format!("Failed to write ZIP file: {}", e))?;

  Ok(())
}

/// Downloads a file directly from URL and saves it to disk, streaming to avoid loading entire file into memory
/// This is memory-efficient and prevents crashes on lower-tier PCs
#[tauri::command]
pub async fn download_and_save_file(
  app: tauri::AppHandle,
  url: String,
  champion_name: String,
  file_name: String,
) -> Result<(), String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to get app data directory: {}", e))?;

  let champions_dir = app_data_dir.join("champions");
  async_fs::create_dir_all(&champions_dir)
    .await
    .map_err(|e| format!("Failed to create champions directory: {}", e))?;

  let champion_dir = champions_dir.join(&champion_name);
  async_fs::create_dir_all(&champion_dir)
    .await
    .map_err(|e| format!("Failed to create champion directory: {}", e))?;

  let file_path = champion_dir.join(&file_name);
  if let Some(parent) = file_path.parent() {
    async_fs::create_dir_all(parent)
      .await
      .map_err(|e| format!("Failed to create parent directory: {}", e))?;
  }

  // Use shared HTTP client with connection pooling for better performance
  let client = HTTP_CLIENT.clone();

  // Retry logic for transient failures (network issues, timeouts, etc.)
  const MAX_RETRIES: u32 = 3;
  let mut last_error = None;

  for attempt in 0..=MAX_RETRIES {
    // Exponential backoff for retries
    if attempt > 0 {
      let delay_ms = 1000 * (1u64 << (attempt - 1)); // 1s, 2s, 4s
      tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    match download_with_retry(client.clone(), &url, &file_path).await {
      Ok(()) => return Ok(()),
      Err(e) => {
        last_error = Some(e);
        if attempt < MAX_RETRIES {
          continue; // Retry
        }
      }
    }
  }

  Err(format!(
    "Download failed after {} attempts: {}",
    MAX_RETRIES + 1,
    last_error.unwrap_or_else(|| "Unknown error".to_string())
  ))
}

/// Internal function that performs a single download attempt
async fn download_with_retry(
  client: Arc<reqwest::Client>,
  url: &str,
  file_path: &std::path::Path,
) -> Result<(), String> {
  // Start download
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

  // Create file and stream download directly to disk
  let mut file = async_fs::File::create(file_path)
    .await
    .map_err(|e| format!("Failed to create file: {}", e))?;

  let mut stream = response.bytes_stream();

  // Stream chunks directly to disk - never loads entire file into memory
  // This prevents crashes on lower-tier PCs by avoiding memory exhaustion
  while let Some(chunk) = stream.next().await {
    let bytes = chunk.map_err(|e| format!("Download stream error: {}", e))?;
    file
      .write_all(&bytes)
      .await
      .map_err(|e| format!("Failed to write chunk: {}", e))?;
  }

  file
    .flush()
    .await
    .map_err(|e| format!("Failed to finalize file: {}", e))?;

  Ok(())
}
