use anyhow::{Result, Context};
use std::collections::HashMap;

use crate::cdn::server::{CdnServer, CdnServerPool};

pub struct CdnClient {
    http_client: reqwest::Client,
    server_pool: CdnServerPool,
    auth_tokens: HashMap<String, String>, // server -> token
}

impl CdnClient {
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http_client,
            server_pool: CdnServerPool::new(Vec::new()),
            auth_tokens: HashMap::new(),
        })
    }

    pub fn set_servers(&mut self, servers: Vec<CdnServer>) {
        self.server_pool = CdnServerPool::new(servers);
    }

    pub fn add_server(&mut self, server: CdnServer) {
        self.server_pool.add_server(server);
    }

    pub fn set_auth_token(&mut self, server: &str, token: String) {
        self.auth_tokens.insert(server.to_string(), token);
    }

    pub async fn fetch_server_list(&self) -> Result<Vec<CdnServer>> {
        // This would normally fetch from Steam's CDN server list
        // For now, return some default servers
        Ok(vec![
            CdnServer::new("valve1.steamcontent.com".to_string(), 80, false)
                .with_weight(100),
            CdnServer::new("valve2.steamcontent.com".to_string(), 80, false)
                .with_weight(100),
            CdnServer::new("valve3.steamcontent.com".to_string(), 80, false)
                .with_weight(100),
        ])
    }

    pub async fn download_manifest(
        &mut self,
        depot_id: u32,
        manifest_id: u64,
        output_path: &std::path::Path,
    ) -> Result<()> {
        // Get a server from the pool
        let server = self.server_pool.get_server()
            .ok_or_else(|| anyhow::anyhow!("No CDN servers available"))?;

        let url = format!("{}/depot/{}/manifest/{}/blob",
            server.url(), depot_id, manifest_id);

        tracing::info!("Downloading manifest from: {}", url);

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .context("Failed to download manifest")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Manifest download failed with status: {}", 
                response.status()
            ));
        }

        let bytes = response.bytes().await?;

        // Create output directory
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to file
        tokio::fs::write(output_path, bytes).await?;

        tracing::info!("Manifest saved to: {:?}", output_path);
        Ok(())
    }

    pub async fn download_chunk(
        &mut self,
        depot_id: u32,
        chunk_id: &str,
        output_path: &std::path::Path,
    ) -> Result<()> {
        let server = self.server_pool.get_server()
            .ok_or_else(|| anyhow::anyhow!("No CDN servers available"))?;

        let url = server.chunk_url(depot_id, chunk_id);

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .context("Failed to download chunk")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Chunk download failed with status: {}", 
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        tokio::fs::write(output_path, bytes).await?;

        Ok(())
    }

    pub async fn download_file(
        &self,
        url: &str,
        output_path: &std::path::Path,
    ) -> Result<()> {
        let response = self.http_client
            .get(url)
            .send()
            .await
            .context(format!("Failed to download file from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "File download failed with status: {}", 
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(output_path, bytes).await?;

        Ok(())
    }

    pub fn get_server(&mut self) -> Option<&CdnServer> {
        self.server_pool.get_server()
    }

    pub fn get_server_count(&self) -> usize {
        self.server_pool.server_count()
    }
}

impl Default for CdnClient {
    fn default() -> Self {
        Self::new().expect("Failed to create CdnClient")
    }
}
