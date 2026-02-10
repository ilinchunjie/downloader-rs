use std::sync::{Arc};
use std::time::Duration;
use chrono::DateTime;
use reqwest::{Client};
use reqwest::header::{HeaderMap};
use crate::download_configuration::DownloadConfiguration;
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

pub async fn head(client: &Arc<Client>, config: &Arc<DownloadConfiguration>) -> crate::error::Result<RemoteFile> {
    let retry_count_limit = config.retry_times_on_failure;
    let mut retry_count = 0;

    'r: loop {
        let request = client.head(config.url());

        let send_future = request.send();
        let result = if config.timeout > 0 {
            tokio::time::timeout(
                Duration::from_secs(config.timeout),
                send_future,
            ).await
        } else {
            Ok(send_future.await)
        };

        // Timeout or request error → retry
        let response = match result {
            Ok(Ok(resp)) => resp,
            _ => {
                if retry_count >= retry_count_limit {
                    return Err(DownloadError::Head);
                }
                retry_count += 1;
                continue 'r;
            }
        };

        if let Err(e) = response.error_for_status_ref() {
            if retry_count >= retry_count_limit {
                if let Some(status_code) = e.status() {
                    return Err(DownloadError::Response(e.url().as_ref().unwrap().to_string(), status_code.into()));
                }
            } else {
                retry_count += 1;
                continue 'r;
            }
        }

        let headers = response.headers();
        return Ok(RemoteFile::new(headers));
    }
}