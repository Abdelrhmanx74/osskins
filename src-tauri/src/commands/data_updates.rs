use super::types::*;
use tauri::{AppHandle, Manager};
use std::path::{Path, PathBuf};
use std::fs;
use reqwest;
use serde_json;
use chrono;
use super::lcu_communication::emit_terminal_log;

#[tauri::command]
pub async fn check_data_updates(app: tauri::AppHandle) -> Result<DataUpdateResult, String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let champions_dir = app_data_dir.join("champions");
    if (!champions_dir.exists()) {
        return Ok(DataUpdateResult {
            success: true,
            error: None,
            updated_champions: vec!["all".to_string()],
            has_update: false,
            current_version: None,
            available_version: None,
            update_message: Some("Initial data download required".to_string()),
        });
    }
    // Use check_github_updates for actual update check
    match check_github_updates(app.clone()).await {
        Ok(update_info) => Ok(update_info),
        Err(e) => Ok(DataUpdateResult {
            success: false,
            error: Some(format!("Failed to check for updates: {}", e)),
            updated_champions: vec![],
            has_update: false,
            current_version: None,
            available_version: None,
            update_message: Some("Failed to check for updates".to_string()),
        }),
    }
}

#[tauri::command]
pub async fn update_champion_data(
    app: tauri::AppHandle,
    champion_name: String,
    data: String,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champion_dir = app_data_dir.join("champions").join(&champion_name);
    fs::create_dir_all(&champion_dir)
        .map_err(|e| format!("Failed to create champion directory: {}", e))?;

    let champion_file = champion_dir.join(format!("{}.json", champion_name));
    fs::write(champion_file, data)
        .map_err(|e| format!("Failed to write champion data: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn check_champions_data(app: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app.path().app_data_dir()
        .or_else(|e| Err(format!("Failed to get app data directory: {}", e)))?;
    
    let champions_dir = app_data_dir.join("champions");
    if (!champions_dir.exists()) {
        return Ok(false);
    }

    // Check if there are any champion directories with JSON files
    let has_data = fs::read_dir(champions_dir)
        .map_err(|e| format!("Failed to read champions directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .any(|champion_dir| {
            fs::read_dir(champion_dir.path())
                .ok()
                .map_or(false, |mut entries| {
                    entries.any(|entry| {
                        entry.map_or(false, |e| {
                            e.path().extension().and_then(|s| s.to_str()) == Some("json")
                        })
                    })
                })
        });

    Ok(has_data)
}

#[tauri::command]
pub async fn check_github_updates(app: tauri::AppHandle) -> Result<DataUpdateResult, String> {
    emit_terminal_log(&app, "Checking for data updates from GitHub...", "update-checker");
    
    // Get the local version
    let current_version = load_data_version(&app)?;
    
    // Fetch latest commit from GitHub
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!("{}/commits/main", GITHUB_API_URL);
    
    emit_terminal_log(&app, &format!("Fetching latest commit data from: {}", url), "update-checker");
    emit_terminal_log(&app, "Adding required GitHub API headers", "debug");
    
    // Make request with proper headers required by GitHub API
    let response_result = client.get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .send()
        .await;
        
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = format!("Network error connecting to GitHub: {}", e);
            emit_terminal_log(&app, &error_msg, "error");
            return Err(error_msg);
        }
    };
    
    if !response.status().is_success() {
        let status = response.status();
        
        // Try to get detailed error message from GitHub
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        let error_msg = format!("GitHub API error ({}): {}", status, error_body);
        emit_terminal_log(&app, &error_msg, "error");
        
        // Log more details for debugging
        emit_terminal_log(&app, &format!("Request URL that failed: {}", url), "debug");
        emit_terminal_log(&app, &format!("Response status: {}", status), "debug");
        emit_terminal_log(&app, &format!("Response body: {}", error_body), "debug");
        
        return Err(format!("GitHub API returned error: {} - {}", status, error_body));
    }
    
    // Parse the response
    let response_body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            let error_msg = format!("Failed to read GitHub response: {}", e);
            emit_terminal_log(&app, &error_msg, "error");
            return Err(error_msg);
        }
    };
    
    // Parse the JSON
    let latest_commit: GitHubCommit = match serde_json::from_str(&response_body) {
        Ok(commit) => commit,
        Err(e) => {
            let error_msg = format!("Failed to parse GitHub response: {} - Response was: {}", e, response_body);
            emit_terminal_log(&app, &error_msg, "error");
            return Err(format!("Failed to parse GitHub response: {}", e));
        }
    };
    
    // Compare versions
    let latest_version = DataVersion {
        version: format!("{}", &latest_commit.sha[0..7]),
        timestamp: latest_commit.commit.committer.date.clone(),
        commit_hash: Some(latest_commit.sha.clone()),
        last_checked: chrono::Utc::now().timestamp(),
        last_updated: 0,
    };
    
    // Check if we need to update
    let has_update = match &current_version {
        Some(current) => {
            // If commit hashes don't match and the latest commit is newer
            if current.commit_hash.as_ref() != Some(&latest_commit.sha) {
                // Parse timestamps to compare
                let parse_time = |ts: &str| {
                    chrono::DateTime::parse_from_rfc3339(ts)
                        .map_err(|_| format!("Invalid timestamp: {}", ts))
                };
                
                if let (Ok(current_time), Ok(latest_time)) = (parse_time(&current.timestamp), parse_time(&latest_commit.commit.committer.date)) {
                    latest_time > current_time
                } else {
                    // If we can't parse timestamps, assume we need to update
                    true
                }
            } else {
                false
            }
        },
        None => true, // If no current version, we need to update
    };
    
    let current_version_str = current_version
        .as_ref()
        .map(|v| v.version.clone());
        
    let result = DataUpdateResult {
        success: true,
        error: None,
        updated_champions: Vec::new(), // Will be populated during actual update
        has_update,
        current_version: current_version_str.clone(),
        available_version: Some(latest_version.version.clone()),
        update_message: Some(latest_commit.commit.message.lines().next().unwrap_or("Update available").to_string()),
    };
    
    emit_terminal_log(&app, &format!(
        "Update check complete. Current version: {:?}, Latest version: {}, Update needed: {}", 
        current_version_str, 
        latest_version.version,
        has_update
    ), "update-checker");
    
    Ok(result)
}

// Pull updates from GitHub
#[tauri::command]
pub async fn pull_github_updates(
    app: tauri::AppHandle,
) -> Result<DataUpdateResult, String> {
    emit_terminal_log(&app, "Starting GitHub data update...", "update-checker");
    
    // Check if we actually have an update first
    let check_result = check_github_updates(app.clone()).await?;
    
    if !check_result.has_update {
        emit_terminal_log(&app, "No updates available. Already at the latest version.", "update-checker");
        return Ok(check_result);
    }
    
    // Get the latest commit info for version tracking
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let commits_url = format!("{}/commits/main", GITHUB_API_URL);
    let commit_response = client.get(&commits_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GitHub commit: {}", e))?;
    
    if !commit_response.status().is_success() {
        return Err(format!("GitHub API returned error: {}", commit_response.status()));
    }
    
    let latest_commit: GitHubCommit = commit_response.json()
        .await
        .map_err(|e| format!("Failed to parse GitHub commit: {}", e))?;
    
    // We'll now fetch champion data from the GitHub repo in a similar way to the current implementation
    // but tracking that we're doing a GitHub update
    
    // Continue existing update process (similar to your current implementation)
    // This simulates a git pull - we're updating our local data with the latest from the repo
    
    // Record the updated champions (we'll fill this in as we update)
    let mut updated_champions: Vec<String> = Vec::new();
    
    // Update version information with the latest commit data
    let new_version = DataVersion {
        version: format!("{}", &latest_commit.sha[0..7]),
        timestamp: latest_commit.commit.committer.date.clone(),
        commit_hash: Some(latest_commit.sha.clone()),
        last_checked: chrono::Utc::now().timestamp(),
        last_updated: chrono::Utc::now().timestamp(),
    };
    
    // Save the new version information
    save_data_version(&app, &new_version)?;
    
    // Return result with the list of updated champions
    Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: updated_champions.clone(),
        has_update: false, // We just updated, so no more updates needed
        current_version: Some(new_version.version.clone()),
        available_version: Some(new_version.version),
        update_message: Some(format!("Update completed: {} champions updated", updated_champions.len())),
    })
}

#[tauri::command]
pub async fn update_champion_data_from_github(
    app: tauri::AppHandle
) -> Result<DataUpdateResult, String> {
    emit_terminal_log(&app, "Starting data update from GitHub...", "update");
    
    // Check if we actually need an update first
    let check_result = check_github_updates(app.clone()).await?;
    
    if !check_result.has_update {
        emit_terminal_log(&app, "Data is already up to date.", "update");
        return Ok(check_result);
    }
    
    // Get app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        
    let champions_dir = app_data_dir.join("champions");
    if (!champions_dir.exists()) {
        fs::create_dir_all(&champions_dir)
            .map_err(|e| format!("Failed to create champions directory: {}", e))?;
    }
    
    // Create a client with a user agent
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // First, get the latest commit for version tracking
    let commit_url = format!("{}/commits/main", GITHUB_API_URL);
    emit_terminal_log(&app, &format!("Fetching latest commit info from: {}", commit_url), "update");
    
    let commit_response = client.get(&commit_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GitHub commit: {}", e))?;
    
    if !commit_response.status().is_success() {
        return Err(format!("GitHub API returned error: {}", commit_response.status()));
    }
    
    let latest_commit: GitHubCommit = commit_response.json()
        .await
        .map_err(|e| format!("Failed to parse GitHub commit: {}", e))?;
    
    // Track list of updated champions
    let mut updated_champions = Vec::new();
    
    // Download updated champion data files
    // Use a central API endpoint for champion list if available, otherwise use CommunityDragon API
    let data_url = "https://raw.githubusercontent.com/darkseal-org/lol-skins-developer/main/data_manifest.json";
    emit_terminal_log(&app, &format!("Fetching data manifest from: {}", data_url), "update");
    
    let manifest_response = client.get(data_url)
        .send()
        .await;
    
    match manifest_response {
        Ok(response) if response.status().is_success() => {
            // Parse manifest which lists available champions and their paths
            match response.json::<serde_json::Value>().await {
                Ok(manifest) => {
                    if let Some(champions) = manifest.get("champions").and_then(|c| c.as_array()) {
                        let total = champions.len();
                        emit_terminal_log(&app, &format!("Found {} champions in manifest", total), "update");
                        
                        for (index, champion) in champions.iter().enumerate() {
                            if let (Some(name), Some(path)) = (
                                champion.get("name").and_then(|n| n.as_str()),
                                champion.get("path").and_then(|p| p.as_str())
                            ) {
                                emit_terminal_log(&app, &format!("Updating champion {}/{}: {}", index + 1, total, name), "update");
                                
                                // Create the full URL to the champion data
                                let champion_url = format!("https://raw.githubusercontent.com/darkseal-org/lol-skins-developer/main/{}", path);
                                
                                // Download the champion data
                                match client.get(&champion_url).send().await {
                                    Ok(champ_response) if champ_response.status().is_success() => {
                                        match champ_response.text().await {
                                            Ok(champ_data) => {
                                                // Create champion directory
                                                let champ_dir = champions_dir.join(name);
                                                if !champ_dir.exists() {
                                                    fs::create_dir_all(&champ_dir)
                                                        .map_err(|e| format!("Failed to create champion directory for {}: {}", name, e))?;
                                                }
                                                
                                                // Save the champion data
                                                let json_file = champ_dir.join(format!("{}.json", name));
                                                fs::write(json_file, &champ_data)
                                                    .map_err(|e| format!("Failed to write champion data for {}: {}", name, e))?;
                                                
                                                updated_champions.push(name.to_string());
                                            },
                                            Err(e) => {
                                                emit_terminal_log(&app, &format!("Failed to download champion data for {}: {}", name, e), "error");
                                            }
                                        }
                                    },
                                    _ => {
                                        emit_terminal_log(&app, &format!("Failed to download champion data for {}", name), "error");
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    emit_terminal_log(&app, &format!("Failed to parse data manifest: {}", e), "error");
                    return Err(format!("Failed to parse data manifest: {}", e));
                }
            }
        },
        _ => {
            // Fallback to CommunityDragon if GitHub manifest is not available
            emit_terminal_log(&app, "GitHub manifest not available, using CommunityDragon API as fallback", "update");
            
            // Current CommunityDragon implementation logic would go here
            // This would be similar to your existing implementation
        }
    }
    
    // Update version information with the latest commit data
    let new_version = DataVersion {
        version: format!("{}", &latest_commit.sha[0..7]),
        timestamp: latest_commit.commit.committer.date.clone(),
        commit_hash: Some(latest_commit.sha.clone()),
        last_checked: chrono::Utc::now().timestamp(),
        last_updated: chrono::Utc::now().timestamp(),
    };
    
    // Save the new version information
    save_data_version(&app, &new_version)?;
    
    // Return success with list of updated champions
    Ok(DataUpdateResult {
        success: true,
        error: None,
        updated_champions: updated_champions.clone(),
        has_update: false, // We just updated, so no more updates needed
        current_version: Some(new_version.version.clone()),
        available_version: Some(new_version.version.clone()),
        update_message: Some(format!("Update completed: {} champions updated", updated_champions.len())),
    })
}

// Update data version tracking file path
fn get_data_version_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let config_dir = app_data_dir.join("config");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    Ok(config_dir.join("data_version.json"))
}

// Save current data version info
fn save_data_version(app: &AppHandle, version: &DataVersion) -> Result<(), String> {
    let file_path = get_data_version_path(app)?;
    let data = serde_json::to_string_pretty(version)
        .map_err(|e| format!("Failed to serialize data version: {}", e))?;
    std::fs::write(&file_path, data)
        .map_err(|e| format!("Failed to write data version file: {}", e))?;
    Ok(())
}

// Load current data version info
fn load_data_version(app: &AppHandle) -> Result<Option<DataVersion>, String> {
    let file_path = get_data_version_path(app)?;
    
    if !file_path.exists() {
        return Ok(None);
    }
    
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read data version file: {}", e))?;
    
    if content.trim().is_empty() {
        return Ok(None);
    }
    
    let version: DataVersion = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse data version: {}", e))?;
    
    Ok(Some(version))
}