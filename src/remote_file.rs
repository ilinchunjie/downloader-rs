use std::sync::{Arc};
use tokio::sync::Mutex;
use chrono::DateTime;
use reqwest::{Client};
use reqwest::header::{HeaderMap};
use crate::error::DownloadError;

pub struct RemoteFile {
    pub total_length: u64,
    pub support_range_download: bool,
    pub last_modified_time: i64,
}

impl RemoteFile {
    pub fn new(head_map: &HeaderMap) -> Self {
        let mut total_length = 0u64;
        let mut support_range_download = false;
        let mut last_modified_time = 0i64;
        if let Some(value) = head_map.get("accept-ranges") {
            support_range_download = value.as_bytes().eq(b"bytes");
        }
        if let Some(content_length) = head_map.get("content-length") {
            if let Ok(content_length_str) = content_length.to_str() {
                if let Ok(length) = content_length_str.parse() {
                    total_length = length;
                }
            }
        }
        if let Some(last_modified) = head_map.get("last-modified") {
            if let Ok(last_modified_str) = last_modified.to_str() {
                if let Ok(last_modified_datetime) = DateTime::parse_from_rfc2822(last_modified_str) {
                    last_modified_time = last_modified_datetime.timestamp();
                }
            }
        }

        Self {
            total_length,
            support_range_download,
            last_modified_time,
        }
    }
}

pub async fn head(client: &Arc<Mutex<Client>>, url: &str) -> crate::error::Result<RemoteFile> {
    let request = client.lock().await.head(url);
    match request.send().await {
        Ok(response) => {
            let headers = response.headers();
            Ok(RemoteFile::new(headers))
        }
        Err(e) => {
            return Err(DownloadError::Head);
        }
    }
}