use std::ops::{Deref};
use std::sync::Arc;
use futures::StreamExt;
use reqwest::header::RANGE;
use tokio::sync::Mutex;
use crate::chunk::{Chunk};
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;

pub struct DownloadTask {}

impl DownloadTask {
    pub fn new() -> DownloadTask {
        DownloadTask {}
    }

    pub async fn start_download(
        &mut self,
        url: Arc<String>,
        options: Arc<Mutex<DownloadOptions>>,
        download_chunk: Arc<Mutex<Chunk>>,
    ) -> crate::error::Result<()> {
        let mut range_str = String::new();
        {
            let range_download = download_chunk.lock().await.range_download;
            if range_download {
                let chunk = download_chunk.lock().await;
                range_str = format!("bytes={}-{}", chunk.chunk_range.position, chunk.chunk_range.end);
            }
        }

        let request = options.lock().await.client.lock().await
            .get(url.deref())
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