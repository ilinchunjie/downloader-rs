use std::sync::Arc;

pub struct DownloadConfiguration {
    pub url: Arc<String>,
    pub path: Arc<String>,
    pub support_range_download: bool,
    pub chunk_download: bool,
    pub chunk_size: u64,
    pub total_length: u64,
    pub remote_version: i64,
    pub remote_file_hash: u64,
    pub download_in_memory: bool,
}

impl DownloadConfiguration {
    pub fn from_url_path(url: String, path: String) -> Self {
        let url = Arc::new(url);
        let path = Arc::new(path);
        Self {
            url,
            path,
            support_range_download: false,
            chunk_download: false,
            chunk_size: 1024 * 1024 * 5,
            total_length: 0,
            remote_version: 0,
            remote_file_hash: 0,
            download_in_memory: false
        }
    }
}