use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use reqwest::Client;
use reqwest::header::RANGE;
use tokio_util::sync::CancellationToken;
use crate::chunk::{Chunk};
use crate::download_configuration::DownloadConfiguration;
use crate::error::DownloadError;
use crate::rate_limiter::RateLimiter;

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
        download_chunk: &mut Chunk,
        rate_limiter: Arc<RateLimiter>,
    ) -> crate::error::Result<()> {
        let retry_count_limit = config.retry_times_on_failure;
        let mut retry_count = 0;

        download_chunk.setup().await?;

        'r: loop {
            let mut request = client.get(config.url());
            if download_chunk.range_download {
                let range_str = format!("bytes={}-{}", download_chunk.chunk_range.position, download_chunk.chunk_range.end);
                request = request.header(RANGE, range_str);
            }

            if config.timeout > 0 {
                request = request.timeout(Duration::from_secs(config.timeout));
            }

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


            let mut body = response.bytes_stream();
            while let Some(chunk) = body.next().await {
                if cancel_token.is_cancelled() {
                    return Ok(());
                }
                match chunk {
                    Ok(bytes) => {
                        // Apply global rate limiting (replaces per-chunk sleep delay)
                        rate_limiter.acquire(bytes.len() as u64).await;
                        download_chunk.received_bytes_async(&bytes).await?;
                    }
                    Err(_e) => {
                        if retry_count >= retry_count_limit {
                            download_chunk.flush_async().await?;
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