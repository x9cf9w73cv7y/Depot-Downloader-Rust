use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Manifest {
    pub depot_id: u32,
    pub manifest_id: u64,
    pub files: Vec<FileEntry>,
    pub total_size: u64,
    pub file_count: u32,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub filename: String,
    pub size: u64,
    pub flags: u32,
    pub hash: Vec<u8>,
    pub chunks: Vec<ChunkEntry>,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct ChunkEntry {
    pub chunk_id: String,
    pub checksum: u32,
    pub offset: u64,
    pub compressed_length: u32,
    pub uncompressed_length: u32,
}

impl Manifest {
    pub fn new(depot_id: u32, manifest_id: u64) -> Self {
        Self {
            depot_id,
            manifest_id,
            files: Vec::new(),
            total_size: 0,
            file_count: 0,
        }
    }

    pub fn from_bytes(_data: &[u8]) -> Result<Self> {
        // This would parse the protobuf manifest format
        // For now, returning a placeholder
        // Real implementation would use prost or similar
        Err(anyhow::anyhow!("Manifest parsing not yet fully implemented"))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        // Serialize back to protobuf format
        Err(anyhow::anyhow!("Manifest serialization not yet fully implemented"))
    }

    pub fn get_file(&self, filename: &str) -> Option<&FileEntry> {
        self.files.iter().find(|f| f.filename == filename)
    }

    pub fn get_files_to_download(&self, _install_dir: &PathBuf) -> Vec<&FileEntry> {
        // Check existing files and return those that need downloading/updating
        self.files.iter().collect()
    }

    pub fn calculate_total_download_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

impl FileEntry {
    pub fn new(filename: String, size: u64) -> Self {
        Self {
            filename,
            size,
            flags: 0,
            hash: Vec::new(),
            chunks: Vec::new(),
            executable: false,
        }
    }

    pub fn with_hash(mut self, hash: Vec<u8>) -> Self {
        self.hash = hash;
        self
    }

    pub fn with_chunks(mut self, chunks: Vec<ChunkEntry>) -> Self {
        self.chunks = chunks;
        self
    }

    pub fn executable(mut self, exec: bool) -> Self {
        self.executable = exec;
        self
    }

    pub fn get_total_chunk_size(&self) -> u64 {
        self.chunks.iter().map(|c| c.compressed_length as u64).sum()
    }
}

impl ChunkEntry {
    pub fn new(
        chunk_id: String,
        checksum: u32,
        offset: u64,
        compressed_length: u32,
        uncompressed_length: u32,
    ) -> Self {
        Self {
            chunk_id,
            checksum,
            offset,
            compressed_length,
            uncompressed_length,
        }
    }
}

pub struct ManifestDiff {
    pub added: Vec<FileEntry>,
    pub modified: Vec<FileEntry>,
    pub removed: Vec<String>,
    pub unchanged: Vec<FileEntry>,
}

impl ManifestDiff {
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            modified: Vec::new(),
            removed: Vec::new(),
            unchanged: Vec::new(),
        }
    }

    pub fn compare(old: &Manifest, new: &Manifest) -> Self {
        let mut diff = Self::new();
        
        // Build filename index for old manifest
        let old_files: HashMap<&str, &FileEntry> = old.files.iter()
            .map(|f| (f.filename.as_str(), f))
            .collect();

        // Compare new files
        for new_file in &new.files {
            if let Some(old_file) = old_files.get(new_file.filename.as_str()) {
                if old_file.hash == new_file.hash {
                    diff.unchanged.push(new_file.clone());
                } else {
                    diff.modified.push(new_file.clone());
                }
            } else {
                diff.added.push(new_file.clone());
            }
        }

        // Find removed files
        let new_filenames: std::collections::HashSet<&str> = new.files.iter()
            .map(|f| f.filename.as_str())
            .collect();
        
        for old_file in &old.files {
            if !new_filenames.contains(old_file.filename.as_str()) {
                diff.removed.push(old_file.filename.clone());
            }
        }

        diff
    }
}
