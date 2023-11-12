use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use reqwest::Client;
use reqwest::header::RANGE;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use crate::chunk::{Chunk};
use crate::download_configuration::DownloadConfiguration;
use crate::error::DownloadError;

pub struct DownloadTask {}

impl DownloadTask {
    pub fn new() -> DownloadTask {
        DownloadTask {}
    }

    pub async fn start_download(
        &mut self,
        config: Arc<DownloadConfiguration>,
        client: Arc<Client>,
        cancel_token: CancellationToken,
        download_chunk: Arc<Mutex<Chunk>>,
    ) -> crate::error::Result<()> {
        let retry_count_limit = config.retry_times_on_failure;
        let mut retry_count = 0;

        'r: loop {
            let mut range_str = String::new();
            {
                let range_download = download_chunk.lock().await.range_download;
                if range_download {
                    let chunk = download_chunk.lock().await;
                    range_str = format!("bytes={}-{}", chunk.chunk_range.position, chunk.chunk_range.end);
                }
            }

            let request = client
                .get(config.url())
                .header(RANGE, range_str);

            let result = request.send().await;

            if cancel_token.is_cancelled() {
                return Ok(());
            }

            if let Err(_) = result {
                if retry_count >= retry_count_limit {
                    return Err(DownloadError::Request);
                } else {
                    retry_count += 1;
                    continue 'r;
                }
            }

            let response = result.unwrap();

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

            download_chunk.lock().await.setup().await?;
            let mut body = response.bytes_stream();
            while let Some(chunk) = body.next().await {
                if cancel_token.is_cancelled() {
                    return Ok(());
                }
                match chunk {
                    Ok(bytes) => {
                        download_chunk.lock().await.received_bytes_async(&bytes).await?;
                        if config.receive_bytes_per_second > 0 {
                            let delay_duration = Duration::from_secs_f64(bytes.len() as f64 / config.receive_bytes_per_second as f64);
                            sleep(delay_duration).await;
                        }
                    }
                    Err(_e) => {
                        if retry_count >= retry_count_limit {
                            download_chunk.lock().await.flush_async().await?;
                            return Err(DownloadError::ResponseChunk);
                        } else {
                            retry_count += 1;
                            continue 'r;
                        }
                    }
                }
            }
            return Ok(());
        }
    }
}