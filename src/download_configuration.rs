use std::sync::Arc;

pub struct DownloadConfiguration {
    pub url: Option<Arc<String>>,
    pub path: Option<Arc<String>>,
    pub range_download: bool,
    pub chunk_size: u64,
    pub total_length: u64,
    pub remote_version: u64,
}

impl DownloadConfiguration {
    pub fn default() -> Self {
        Self {
            url: None,
            path: None,
            range_download: true,
            chunk_size: 1024 * 1024,
            total_length: 0,
            remote_version: 0
        }
    }
}