use anyhow::{Result, Context};
use reqwest;
use std::collections::HashMap;

pub struct ManifestHubFetcher {
    client: reqwest::Client,
    base_url: String,
    depot_keys_cache: HashMap<u32, String>,
}

impl ManifestHubFetcher {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            depot_keys_cache: HashMap::new(),
        }
    }

    pub fn default() -> Self {
        Self::new("https://raw.githubusercontent.com/SteamAutoCracks/ManifestHub/refs/heads/main")
    }

    /// Load all depot keys from the centralized depotkeys.json file
    pub async fn load_all_depot_keys(&mut self) -> Result<()> {
        let url = format!("{}/depotkeys.json", self.base_url);
        
        tracing::info!("Loading depot keys from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch depot keys JSON")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch depot keys: HTTP {}",
                response.status()
            ));
        }

        let json_text = response.text().await?;
        tracing::debug!("Loaded {} bytes of depot keys JSON", json_text.len());

        // Parse JSON - format is {"depot_id": "key", ...}
        let keys: HashMap<String, String> = serde_json::from_str(&json_text)
            .context("Failed to parse depot keys JSON")?;

        // Convert to u32 keys
        self.depot_keys_cache.clear();
        for (depot_id_str, key) in keys {
            if let Ok(depot_id) = depot_id_str.parse::<u32>() {
                self.depot_keys_cache.insert(depot_id, key);
            }
        }

        tracing::info!("Loaded {} depot keys from JSON", self.depot_keys_cache.len());
        Ok(())
    }

    /// Get depot keys for a specific app
    /// Note: The depotkeys.json doesn't organize by app_id, but by depot_id
    /// Common pattern: For app X, depot is usually X, X+1, etc.
    /// This method tries to find all depots that might belong to this app
    pub async fn fetch_depot_keys(&mut self, app_id: u32) -> Result<HashMap<u32, String>> {
        // First, ensure we have the cache loaded
        if self.depot_keys_cache.is_empty() {
            self.load_all_depot_keys().await?;
        }

        // Try to find depots that might belong to this app
        let mut found_keys = HashMap::new();

        // Common pattern: For app X, depots might be:
        // - X (the main depot)
        // - X + 1 (Windows depot)
        // - X + 2 (Linux depot)
        // - X + 3 (Mac depot)
        // - And other related depots
        
        // Check the exact app_id as depot_id
        if let Some(key) = self.depot_keys_cache.get(&app_id) {
            found_keys.insert(app_id, key.clone());
            tracing::info!("Found depot key for depot {}", app_id);
        }

        // Check X + 1 through X + 10
        for offset in 1..=10 {
            let depot_id = app_id + offset;
            if let Some(key) = self.depot_keys_cache.get(&depot_id) {
                found_keys.insert(depot_id, key.clone());
                tracing::info!("Found depot key for depot {} (+{})", depot_id, offset);
            }
        }

        // Check if app_id ends in 0, try X + 1 (common for Windows games)
        // e.g., app 440 -> depot 441
        if app_id % 10 == 0 {
            let depot_id = app_id + 1;
            if let Some(key) = self.depot_keys_cache.get(&depot_id) {
                if !found_keys.contains_key(&depot_id) {
                    found_keys.insert(depot_id, key.clone());
                    tracing::info!("Found Windows depot key for depot {} (app {})", depot_id, app_id);
                }
            }
        }

        tracing::info!("Found {} depot keys for app {}", found_keys.len(), app_id);
        
        if found_keys.is_empty() {
            tracing::warn!("No depot keys found for app {}. Available depot keys: {}", 
                app_id, self.depot_keys_cache.len());
        }

        Ok(found_keys)
    }

    /// Get a specific depot key by depot ID
    pub fn get_depot_key(&self, depot_id: u32) -> Option<&String> {
        self.depot_keys_cache.get(&depot_id)
    }

    /// Fetch manifest from ManifestHub
    pub async fn fetch_manifest(&self, depot_id: u32, manifest_id: u64) -> Result<Vec<u8>> {
        let url = format!("{}/{}_{}.manifest", self.base_url, depot_id, manifest_id);
        
        tracing::info!("Fetching manifest from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch manifest")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch manifest: HTTP {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Check if manifest exists on ManifestHub
    pub async fn manifest_exists(&self, depot_id: u32, manifest_id: u64) -> Result<bool> {
        let url = format!("{}/{}_{}.manifest", self.base_url, depot_id, manifest_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }
}

impl Default for ManifestHubFetcher {
    fn default() -> Self {
        Self::new("https://raw.githubusercontent.com/SteamAutoCracks/ManifestHub/refs/heads/main")
    }
}
