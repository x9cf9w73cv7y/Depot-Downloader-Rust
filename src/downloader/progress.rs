use std::fmt;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub message: String,
    pub current_file: Option<String>,
    pub files_downloaded: u64,
    pub files_total: u64,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub complete: bool,
}

impl DownloadProgress {
    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            current_file: None,
            files_downloaded: 0,
            files_total: 0,
            bytes_downloaded: 0,
            bytes_total: 0,
            complete: false,
        }
    }

    pub fn file_progress(
        filename: &str,
        files_done: u64,
        files_total: u64,
        bytes_done: u64,
        bytes_total: u64,
    ) -> Self {
        Self {
            message: format!("Downloading {}...", filename),
            current_file: Some(filename.to_string()),
            files_downloaded: files_done,
            files_total: files_total,
            bytes_downloaded: bytes_done,
            bytes_total: bytes_total,
            complete: false,
        }
    }

    pub fn complete() -> Self {
        Self {
            message: "Download complete!".to_string(),
            current_file: None,
            files_downloaded: 0,
            files_total: 0,
            bytes_downloaded: 0,
            bytes_total: 0,
            complete: true,
        }
    }

    pub fn progress_percent(&self) -> f32 {
        if self.bytes_total == 0 {
            0.0
        } else {
            (self.bytes_downloaded as f32 / self.bytes_total as f32).min(1.0)
        }
    }

    pub fn file_progress_percent(&self) -> f32 {
        if self.files_total == 0 {
            0.0
        } else {
            (self.files_downloaded as f32 / self.files_total as f32).min(1.0)
        }
    }

    pub fn speed_formatted(&self, elapsed_secs: f64) -> String {
        if elapsed_secs <= 0.0 {
            return "0 B/s".to_string();
        }
        let bytes_per_sec = self.bytes_downloaded as f64 / elapsed_secs;
        Self::format_bytes_per_sec(bytes_per_sec)
    }

    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    pub fn format_bytes_per_sec(bytes_per_sec: f64) -> String {
        format!("{}/s", Self::format_bytes(bytes_per_sec as u64))
    }
}

impl fmt::Display for DownloadProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.complete {
            write!(f, "{}", self.message)
        } else if let Some(ref file) = self.current_file {
            write!(
                f,
                "{} (File {} of {})",
                self.message, self.files_downloaded, self.files_total
            )
        } else {
            write!(f, "{}", self.message)
        }
    }
}

pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send>;
