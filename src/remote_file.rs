use std::ops::Deref;
use std::sync::Arc;
use chrono::DateTime;
use reqwest::header::{HeaderMap, HeaderValue};

pub struct RemoteFileInfo {
    pub total_length: u64,
    pub support_range_download: bool,
    pub last_modified_time: i64
}

impl RemoteFileInfo {
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
            last_modified_time
        }
    }
}

pub struct RemoteFile {
    url: Arc<String>,
    pub remote_file_info: Option<RemoteFileInfo>,
}

impl RemoteFile {
    pub fn new(url: Arc<String>) -> RemoteFile {
        RemoteFile {
            url,
            remote_file_info: None,
        }
    }

    pub async fn head(&mut self) {
        let client = reqwest::Client::new();
        let request = client.head(self.url.deref());
        if let Ok(response) = request.send().await {
            let headers = response.headers();
            self.remote_file_info = Some(RemoteFileInfo::new(headers));
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use crate::remote_file::RemoteFile;

    #[tokio::test]
    pub async fn test_remote_file() {
        let url = "https://lan.sausage.xd.com/servers.txt";
        let mut remote_file = RemoteFile::new(Arc::new(url.to_string()));
        remote_file.head().await;
    }
}