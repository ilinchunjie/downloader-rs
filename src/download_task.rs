use std::ops::{Deref};
use std::sync::Arc;
use futures::StreamExt;
use reqwest::header::RANGE;
use tokio::sync::Mutex;
use crate::chunk::{Chunk};
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;

#[derive(Clone)]
pub struct DownloadTaskConfiguration {
    pub url: Arc<String>,
    pub range_download: bool,
    pub range_start: u64,
    pub range_end: u64,
}

pub struct DownloadTask {
    config: DownloadTaskConfiguration,
}

impl DownloadTask {
    pub fn new(config: DownloadTaskConfiguration) -> DownloadTask {
        DownloadTask {
            config,
        }
    }

    pub async fn start_download(
        &mut self,
        options: Arc<Mutex<DownloadOptions>>,
        download_chunk: Arc<Mutex<Chunk>>
    ) -> crate::error::Result<()> {
        let mut range_str = String::new();
        if self.config.range_download {
            if self.config.range_start < self.config.range_end {
                range_str = format!("bytes={}-{}", self.config.range_start, self.config.range_end);
            } else {
                range_str = format!("bytes={}-", self.config.range_start);
            }
        }

        let request = options.lock().await.client.lock().await
            .get(self.config.url.deref())
            .header(RANGE, range_str);

        let result = request.send().await;

        if options.lock().await.cancel {
            return Ok(());
        }

        match result {
            Ok(response) => {
                match response.error_for_status() {
                    Ok(response) => {
                        download_chunk.lock().await.setup().await?;
                        let mut body = response.bytes_stream();
                        while let Some(chunk) = body.next().await {
                            if options.lock().await.cancel {
                                return Ok(());
                            }
                            match chunk {
                                Ok(bytes) => {
                                    download_chunk.lock().await.received_bytes_async(&bytes).await?;
                                }
                                Err(_e) => {
                                    return Err(DownloadError::ResponseChunk);
                                }
                            }
                        }
                        download_chunk.lock().await.flush_async().await?;
                    }
                    Err(_e) => {
                        return Err(DownloadError::Response);
                    }
                }
                Ok(())
            }
            Err(_e) => {
                return Err(DownloadError::Request);
            }
        }
    }
}