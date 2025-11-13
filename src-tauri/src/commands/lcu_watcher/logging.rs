// Logging utilities for LCU watcher

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use once_cell::sync::Lazy;
use chrono::Utc;
use copypasta::{ClipboardContext, ClipboardProvider};

// Global in-memory log buffer
pub static LOG_BUFFER: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn emit_terminal_log(_app: &AppHandle, message: &str) {
  // Buffer logs in memory for later printing. Keep emitting to stdout for backwards visibility.
  println!("{}", message);
  if let Ok(mut buf) = LOG_BUFFER.lock() {
    buf.push(message.to_string());
    // Keep buffer bounded to last 2000 lines
    if buf.len() > 2000 {
      let excess = buf.len() - 2000;
      buf.drain(0..excess);
    }
  }

  // Also append to an on-disk live log so we can export the full runtime log later.
  if let Ok(app_dir) = _app.path().app_data_dir() {
    let logs_dir = app_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
      // Non-fatal: we still keep logs in memory
      println!("[LCU Watcher] Failed to ensure logs dir exists: {}", e);
      return;
    }

    let live_log = logs_dir.join("osskins-live.log");
    // Try to append; ignore failures to avoid crashing the watcher thread
    if let Ok(mut f) = File::options().create(true).append(true).open(&live_log) {
      if let Err(e) = writeln!(f, "{}", message) {
        println!("[LCU Watcher] Failed to write to live log: {}", e);
      }
    } else {
      // Could not open the file - non-fatal
      // Keep in-memory buffer as fallback
    }
  }
}

pub fn append_global_log(message: &str) {
  // Print to stdout for normal visibility
  println!("{}", message);

  // Append to in-memory buffer (bounded)
  if let Ok(mut buf) = LOG_BUFFER.lock() {
    buf.push(message.to_string());
    if buf.len() > 2000 {
      let excess = buf.len() - 2000;
      buf.drain(0..excess);
    }
  }

  // Append to on-disk live log using APPDATA fallback if necessary
  let logs_dir = std::env::var("APPDATA")
    .map(|ap| PathBuf::from(ap).join("com.osskins.app").join("logs"))
    .unwrap_or_else(|_| PathBuf::from(".").join("logs"));

  if let Err(e) = std::fs::create_dir_all(&logs_dir) {
    // Non-fatal
    eprintln!("[LCU Watcher] Failed to ensure logs dir exists: {}", e);
    return;
  }

  let live_log = logs_dir.join("osskins-live.log");
  if let Ok(mut f) = File::options().create(true).append(true).open(&live_log) {
    if let Err(e) = writeln!(f, "{}", message) {
      eprintln!("[LCU Watcher] Failed to write to live log: {}", e);
    }
  }
}

/// Write buffered logs to a timestamped temp file and copy contents to clipboard.
#[tauri::command]
pub fn print_logs(app: AppHandle) -> Result<String, String> {
  // Prefer the on-disk live log (captures everything appended since app start).
  let app_dir = app
    .path()
    .app_data_dir()
    .unwrap_or_else(|_| PathBuf::from("."));
  let out_dir = app_dir.join("logs");
  if let Err(e) = std::fs::create_dir_all(&out_dir) {
    return Err(format!("Failed to create log dir: {}", e));
  }

  let live_log = out_dir.join("osskins-live.log");

  // Read from the live log file if it exists and is readable. Otherwise fall back to in-memory buffer.
  let full_contents = if live_log.exists() {
    match std::fs::read_to_string(&live_log) {
      Ok(s) if !s.is_empty() => s,
      _ => {
        // Fall back to buffer
        let buf = LOG_BUFFER
          .lock()
          .map_err(|e| format!("Lock error: {}", e))?;
        if buf.is_empty() {
          return Err("No logs available".to_string());
        }
        buf.join("\n")
      }
    }
  } else {
    let buf = LOG_BUFFER
      .lock()
      .map_err(|e| format!("Lock error: {}", e))?;
    if buf.is_empty() {
      return Err("No logs available".to_string());
    }
    buf.join("\n")
  };

  // Write exported timestamped copy
  let filename = format!("osskins-logs-{}.txt", Utc::now().format("%Y%m%d-%H%M%S"));
  let out_path = out_dir.join(&filename);
  let mut file = File::create(&out_path).map_err(|e| format!("Failed to create file: {}", e))?;
  if let Err(e) = write!(file, "{}", full_contents) {
    return Err(format!("Failed to write logs: {}", e));
  }

  // Copy to clipboard
  let mut ctx = ClipboardContext::new().map_err(|e| format!("Clipboard init failed: {}", e))?;
  ctx
    .set_contents(full_contents)
    .map_err(|e| format!("Clipboard write failed: {}", e))?;

  Ok(out_path.to_string_lossy().to_string())
}

#[allow(dead_code)]
pub fn delayed_log(app: &AppHandle, message: &str) {
  emit_terminal_log(app, message);
  thread::sleep(Duration::from_millis(100)); // Small delay for better readability
}
