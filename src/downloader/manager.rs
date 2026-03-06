use anyhow::{Result, Context};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::fs;

use crate::manifest::ManifestHubFetcher;
use crate::manifest::parser::{Manifest, FileEntry, ChunkEntry};
use crate::manifest::decryption::ManifestDecryptor;
use crate::manifest::store::ManifestStore;
use crate::cdn::client::CdnClient;
use crate::downloader::progress::{DownloadProgress, ProgressCallback};

const MAX_CONCURRENT_DOWNLOADS: usize = 8;
const MAX_RETRIES: u32 = 3;

use std::collections::HashMap;

pub struct DownloadManager {
    manifest_fetcher: ManifestHubFetcher,
    cdn_client: CdnClient,
    decryptor: ManifestDecryptor,
    manifest_store: ManifestStore,
    progress_tx: mpsc::Sender<DownloadProgress>,
    semaphore: Arc<Semaphore>,
    depot_keys: HashMap<u32, String>,
}

impl DownloadManager {
    pub fn new(progress_tx: mpsc::Sender<DownloadProgress>) -> Result<Self> {
        Ok(Self {
            manifest_fetcher: ManifestHubFetcher::default(),
            cdn_client: CdnClient::new()?,
            decryptor: ManifestDecryptor::new(),
            manifest_store: ManifestStore::default(),
            progress_tx,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)),
            depot_keys: HashMap::new(),
        })
    }

    pub fn set_depot_keys(&mut self, keys: HashMap<u32, String>) {
        self.depot_keys = keys;
        
        // Add keys to decryptor
        for (id, key_hex) in &self.depot_keys {
            if let Ok(key) = hex::decode(key_hex) {
                self.decryptor.add_depot_key(*id, key);
            }
        }
    }

    pub async fn download_depot(
        &mut self,
        app_id: u32,
        depot_id: u32,
        manifest_id: Option<u64>,
        install_dir: &PathBuf,
    ) -> Result<()> {
        tracing::info!("Starting download for depot {} of app {}", depot_id, app_id);

        // Check if we have depot keys
        if self.depot_keys.is_empty() {
            return Err(anyhow::anyhow!("No depot keys available. Please fetch depot keys first."));
        }

        // Step 2: Fetch or load manifest
        // If no manifest_id provided, try to fetch the latest one
        let manifest_id = if let Some(id) = manifest_id {
            id
        } else {
            self.send_progress(DownloadProgress::message("Fetching latest manifest ID from Steam...")).await;
            match self.fetch_latest_manifest_id(depot_id).await {
                Ok(id) => {
                    tracing::info!("Found latest manifest ID: {}", id);
                    self.send_progress(DownloadProgress::message(format!("Using manifest ID: {}", id))).await;
                    id
                }
                Err(e) => {
                    tracing::error!("Failed to fetch latest manifest ID: {}", e);
                    return Err(anyhow::anyhow!("Failed to fetch latest manifest ID. Please specify a manifest ID manually. Error: {}", e));
                }
            }
        };
        
        let manifest = self.fetch_manifest(depot_id, manifest_id).await?;

        tracing::info!("Manifest contains {} files", manifest.files.len());

        // Step 3: Create install directory
        fs::create_dir_all(install_dir).await?;

        // Step 4: Download files
        let total_files = manifest.files.len();
        let mut downloaded_files = 0u64;
        let total_size = manifest.calculate_total_download_size();
        let mut downloaded_bytes = 0u64;

        for file in &manifest.files {
            downloaded_files += 1;
            
            self.send_progress(DownloadProgress::file_progress(
                &file.filename,
                downloaded_files,
                total_files as u64,
                downloaded_bytes,
                total_size,
            )).await;

            let file_path = install_dir.join(&file.filename);
            
            // Create parent directories
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Download file
            match self.download_file(depot_id, file, &file_path).await {
                Ok(bytes) => {
                    downloaded_bytes += bytes;
                    tracing::info!("Downloaded: {} ({} bytes)", file.filename, bytes);
                }
                Err(e) => {
                    tracing::error!("Failed to download {}: {}", file.filename, e);
                    return Err(e);
                }
            }
        }

        self.send_progress(DownloadProgress::complete()).await;
        tracing::info!("Download completed: {} files, {} bytes", downloaded_files, downloaded_bytes);

        Ok(())
    }

    async fn fetch_manifest(&mut self, depot_id: u32, manifest_id: u64) -> Result<Manifest> {
        // Try to load from cache first
        if let Some(manifest) = self.manifest_store.load_manifest(depot_id, manifest_id).await? {
            tracing::info!("Using cached manifest for depot {}", depot_id);
            return Ok(manifest);
        }

        // Fetch from ManifestHub
        self.send_progress(DownloadProgress::message(format!("Fetching manifest {} for depot {}...", manifest_id, depot_id))).await;
        
        let encrypted_data = match self.manifest_fetcher.fetch_manifest(depot_id, manifest_id).await {
            Ok(data) => data,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Manifest {} for depot {} not found on ManifestHub. \\n\
                    This manifest may not be cached yet. \\\n\
                    Solutions:\\\n\
                    1. Use the original DepotDownloader (C#) which can download manifests directly from Steam\\n\
                    2. Wait for the manifest to be added to ManifestHub\\n\
                    3. Check if the depot ID is correct\\n\
                    \\\n\
                    Technical error: {}",
                    manifest_id, depot_id, e
                ));
            }
        };

        // Decrypt manifest
        let decrypted_data = self.decryptor.decrypt_manifest(depot_id, &encrypted_data)?;

        // Parse manifest
        let manifest = Manifest::from_bytes(&decrypted_data)?;

        // Cache manifest
        self.manifest_store.save_manifest(depot_id, manifest_id, &encrypted_data).await?;

        Ok(manifest)
    }

    async fn fetch_latest_manifest_id(&self, depot_id: u32) -> Result<u64> {
        // For now, we cannot automatically detect the latest manifest ID without SteamKit2
        // Return an error with instructions on how to get the manifest ID manually
        Err(anyhow::anyhow!(
            "Cannot automatically detect manifest ID for depot {}. \
             Please specify a manifest ID manually in the download dialog. \
             You can find manifest IDs on SteamDB.gg or similar sites. \
             Look for the depot ID {} and copy the latest manifest ID.",
            depot_id, depot_id
        ))
    }

    async fn download_file(
        &mut self,
        depot_id: u32,
        file: &FileEntry,
        output_path: &PathBuf,
    ) -> Result<u64> {
        // Open file for writing
        let mut file_handle = fs::File::create(output_path).await?;

        let mut total_bytes = 0u64;

        for chunk in &file.chunks {
            let chunk_data = self.download_chunk_with_retry(depot_id, chunk).await?;
            
            // Decrypt chunk
            let iv = [0u8; 16]; // Steam uses zero IV for chunks
            let decrypted_chunk = self.decryptor.decrypt_chunk(depot_id, &chunk_data, &iv)?;

            // Write chunk to file at correct offset
            use tokio::io::AsyncSeekExt;
            file_handle.seek(std::io::SeekFrom::Start(chunk.offset)).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file_handle, &decrypted_chunk).await?;

            total_bytes += decrypted_chunk.len() as u64;
        }

        // Flush and set executable permissions if needed
        tokio::io::AsyncWriteExt::flush(&mut file_handle).await?;
        drop(file_handle);

        if file.executable {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(output_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(output_path, perms)?;
            }
        }

        Ok(total_bytes)
    }

    async fn download_chunk_with_retry(
        &mut self,
        depot_id: u32,
        chunk: &ChunkEntry,
    ) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.download_chunk(depot_id, chunk).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    tracing::warn!("Chunk download failed (attempt {}): {}", attempt + 1, e);
                    last_error = Some(e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * (attempt + 1) as u64)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to download chunk after {} retries", MAX_RETRIES)))
    }

    async fn download_chunk(&mut self, depot_id: u32, chunk: &ChunkEntry) -> Result<Vec<u8>> {
        // Get a server from the pool
        let server = self.cdn_client.get_server()
            .ok_or_else(|| anyhow::anyhow!("No CDN servers available"))?;

        let url = server.chunk_url(depot_id, &chunk.chunk_id);

        let response = reqwest::get(&url).await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download chunk: HTTP {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn send_progress(&self, progress: DownloadProgress) {
        let _ = self.progress_tx.send(progress).await;
    }
}
