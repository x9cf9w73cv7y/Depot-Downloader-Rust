use anyhow::{Result, Context};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use std::path::PathBuf;

use crate::cdn::server::CdnServer;

const MAX_CONCURRENT_DOWNLOADS: usize = 8;
const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks for file downloads

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64, // bytes per second
    pub current_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub output_path: PathBuf,
    pub file_name: String,
    pub file_size: u64,
}

pub struct DownloadManager {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
    progress_tx: mpsc::Sender<DownloadProgress>,
}

impl DownloadManager {
    pub fn new(progress_tx: mpsc::Sender<DownloadProgress>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)),
            progress_tx,
        }
    }

    pub async fn download_file(
        &self,
        url: &str,
        output_path: &PathBuf,
    ) -> Result<()> {
        let permit = self.semaphore.clone().acquire_owned().await
            .context("Failed to acquire download permit")?;

        let response = self.client
            .get(url)
            .send()
            .await
            .context(format!("Failed to download from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download failed with status: {}", 
                response.status()
            ));
        }

        // Get total size
        let total_size = response
            .content_length()
            .unwrap_or(0);

        // Create output directory
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Download with progress tracking
        let mut file = tokio::fs::File::create(output_path).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let start_time = std::time::Instant::now();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;

            // Calculate speed
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };

            // Send progress update
            let progress = DownloadProgress {
                downloaded,
                total: total_size,
                speed,
                current_file: Some(output_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()),
            };
            let _ = self.progress_tx.send(progress).await;
        }

        // Ensure file is flushed
        tokio::io::AsyncWriteExt::flush(&mut file).await?;
        drop(file);
        drop(permit);

        Ok(())
    }

    pub async fn download_chunk(
        &self,
        server: &CdnServer,
        depot_id: u32,
        chunk_id: &str,
        output_path: &PathBuf,
    ) -> Result<Vec<u8>> {
        let url = server.chunk_url(depot_id, chunk_id);
        
        let permit = self.semaphore.clone().acquire_owned().await
            .context("Failed to acquire download permit")?;

        let response = self.client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to download chunk from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Chunk download failed with status: {}", 
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        drop(permit);

        Ok(bytes.to_vec())
    }

    pub async fn download_manifest(
        &self,
        server: &CdnServer,
        depot_id: u32,
        manifest_id: u64,
    ) -> Result<Vec<u8>> {
        let url = format!("{}/depot/{}/manifest/{}/blob",
            server.url(), depot_id, manifest_id);
        
        let permit = self.semaphore.clone().acquire_owned().await
            .context("Failed to acquire download permit")?;

        let response = self.client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to download manifest from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Manifest download failed with status: {}", 
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        drop(permit);

        Ok(bytes.to_vec())
    }

    pub async fn download_multiple(
        &self,
        tasks: Vec<DownloadTask>,
    ) -> Result<Vec<Result<()>>> {
        let mut handles = Vec::new();

        for task in tasks {
            let this = self.clone();
            let handle = tokio::spawn(async move {
                this.download_file(&task.url, &task.output_path).await
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await?);
        }

        Ok(results)
    }
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
            progress_tx: self.progress_tx.clone(),
        }
    }
}
