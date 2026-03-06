use anyhow::{Result, Context};
use reqwest;
use std::collections::HashMap;

pub struct ManifestHubFetcher {
    client: reqwest::Client,
    base_url: String,
}

impl ManifestHubFetcher {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    pub fn default() -> Self {
        Self::new("https://raw.githubusercontent.com/SteamAutoCracks/ManifestHub/refs/heads/main")
    }

    /// Fetch depot keys from ManifestHub
    /// Returns a map of depot_id -> hex_key
    pub async fn fetch_depot_keys(&self, app_id: u32) -> Result<HashMap<u32, String>> {
        let url = format!("{}/{}/key.vdf", self.base_url, app_id);
        
        tracing::info!("Fetching depot keys from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch depot keys")?;

        if response.status().as_u16() == 404 {
            return Ok(HashMap::new()); // No keys found
        }

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch depot keys: HTTP {}",
                response.status()
            ));
        }

        let vdf_content = response.text().await?;
        
        // Parse VDF format: "depotId" { "DecryptionKey" "hexKey" }
        let mut keys = HashMap::new();
        
        // Simple VDF parsing
        let re = regex::Regex::new(r#""(\d+)"\s*\{\s*"DecryptionKey"\s*"([a-fA-F0-9]+)"\s*\}"#)?;
        
        for cap in re.captures_iter(&vdf_content) {
            if let (Ok(depot_id), Some(key)) = (cap[1].parse::<u32>(), cap.get(2)) {
                keys.insert(depot_id, key.as_str().to_string());
            }
        }
        
        tracing::info!("Found {} depot keys", keys.len());
        Ok(keys)
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
        Self::default()
    }
}
