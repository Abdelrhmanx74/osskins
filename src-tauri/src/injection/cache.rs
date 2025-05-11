use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use once_cell::sync::Lazy;

// Add overlay cache to optimize injection performance
pub static OVERLAY_CACHE: Lazy<Arc<Mutex<OverlayCache>>> = Lazy::new(|| {
    Arc::new(Mutex::new(OverlayCache::new()))
});

// Structure to cache pre-built overlays
#[derive(Debug, Default)]
pub struct OverlayCache {
    // Map mod directory hash to pre-built overlay directory
    overlays: HashMap<String, (PathBuf, Instant)>,
    // Last time the cache was cleaned up
    last_cleanup: Option<Instant>,
}

impl OverlayCache {
    // Create a new empty cache
    pub fn new() -> Self {
        Self {
            overlays: HashMap::new(),
            last_cleanup: None,
        }
    }
    
    // Generate a hash key for the set of mods
    fn generate_hash(&self, mods: &[String]) -> String {
        let mut sorted_mods = mods.to_vec();
        sorted_mods.sort();
        
        // Create a simple hash by joining all mod names
        let combined = sorted_mods.join("_");
        format!("{:x}", md5::compute(combined))
    }
    
    // Check if we have a valid cached overlay for the given mods
    pub fn get_cached_overlay(&mut self, mods: &[String]) -> Option<PathBuf> {
        // Clean up old entries first if needed
        self.cleanup_old_entries();
        
        let hash = self.generate_hash(mods);
        
        // Check if we have a cached overlay
        if let Some((path, time)) = self.overlays.get_mut(&hash) {
            // Check if the overlay exists and is not too old (30 minutes max)
            if path.exists() && time.elapsed() < Duration::from_secs(30 * 60) {
                // Update the access time
                *time = Instant::now();
                return Some(path.clone());
            }
        }
        
        None
    }
    
    // Add a newly built overlay to the cache
    pub fn add_overlay(&mut self, mods: &[String], path: PathBuf) {
        let hash = self.generate_hash(mods);
        self.overlays.insert(hash, (path, Instant::now()));
        
        // Maybe clean up old entries if we haven't in a while
        self.cleanup_old_entries();
    }
    
    // Clean up old cache entries
    fn cleanup_old_entries(&mut self) {
        // Only clean up once per hour
        if let Some(last_time) = self.last_cleanup {
            if last_time.elapsed() < Duration::from_secs(3600) {
                return;
            }
        }
        
        // Remove entries older than 2 hours
        let max_age = Duration::from_secs(2 * 3600);
        let now = Instant::now();
        
        self.overlays.retain(|_, (path, time)| {
            let keep = time.elapsed() < max_age && path.exists();
            // Try to remove the directory if we're discarding it
            if !keep && path.exists() {
                let _ = std::fs::remove_dir_all(path);
            }
            keep
        });
        
        self.last_cleanup = Some(now);
    }
}