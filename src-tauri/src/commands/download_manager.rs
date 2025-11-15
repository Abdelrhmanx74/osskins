use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const EVENT_NAME: &str = "download-progress";

#[derive(Clone)]
struct DownloadCtrl {
  cancel: CancellationToken,
}

static TASKS: Lazy<Arc<Mutex<HashMap<String, DownloadCtrl>>>> =
  Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadProgressPayload {
  id: String,
  status: String,            // queued | downloading | completed | failed | canceled
  url: String,
  category: String,          // skin | data | tools | misc
  downloaded: Option<u64>,
  total: Option<u64>,
  speed: Option<f64>,        // bytes/sec
  champion_name: Option<String>,
  file_name: Option<String>,
  dest_path: Option<String>,
  error: Option<String>,
}

fn emit(app: &tauri::AppHandle, payload: DownloadProgressPayload) {
  let _ = app.emit(EVENT_NAME, payload);
}

static HTTP_CLIENT: Lazy<Arc<reqwest::Client>> = Lazy::new(|| {
  Arc::new(
    reqwest::Client::builder()
      .user_agent("osskins-downloader/2.0")
      .timeout(Duration::from_secs(600))
      .tcp_keepalive(Duration::from_secs(60))
      .pool_max_idle_per_host(16)
      .pool_idle_timeout(Duration::from_secs(120))
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
    let mut file = async_fs::File::create(&file_path)
      .await
      .map_err(|e| format!("Failed to create destination file: {}", e))?;

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
        dest_path: file_path.to_string_lossy().to_string().into(),
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
                    dest_path: file_path.to_string_lossy().to_string().into(),
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
  let mut map = tasks.lock().await;
  if let Some(ctrl) = map.get(&id) {
    ctrl.cancel.cancel();
    Ok(true)
  } else {
    Ok(false)
  }
}
