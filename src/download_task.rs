use std::fmt::{Debug};
use crate::download_configuration::DownloadConfiguration;
use futures::StreamExt;
use reqwest::header::RANGE;
use crate::download_handle::DownloadHandle;

pub struct DownloadTask {
    config: DownloadConfiguration,
    handle: DownloadHandle,
}

impl DownloadTask {
    pub fn new(file_path: String, config: DownloadConfiguration) -> DownloadTask {
        DownloadTask {
            config,
            handle: DownloadHandle::new(file_path),
        }
    }

    pub async fn start_download(&mut self) {
        let mut range_str = String::new();
        if self.config.range_download {
            if self.config.range_start < self.config.range_end {
                range_str = format!("bytes={}-{}", self.config.range_start, self.config.range_end);
            } else {
                range_str = format!("bytes={}-", self.config.range_start);
            }
        }

        let request = reqwest::Client::new().
            get(&self.config.url).
            header(RANGE, range_str);

        let result = request.send().await;
        if let Ok(response) = result {
            if response.status().is_success() {

                self.handle.setup().await;

                let mut body = response.bytes_stream();
                while let Some(chunk) = body.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let buffer = bytes.to_vec() as Vec<u8>;
                            self.handle.received_bytes_async(&buffer).await;
                        }
                        Err(e) => {
                            println!("{}", e);
                        }
                    }
                }
            }
        }
    }
}