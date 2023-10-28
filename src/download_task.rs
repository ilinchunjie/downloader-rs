use std::error::Error;
use std::fmt::{Debug};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures::StreamExt;
use reqwest::header::RANGE;
use tokio::sync::Mutex;
use crate::download_handle::DownloadHandle;
use crate::downloader::DownloadOptions;

#[derive(Clone)]
pub struct DownloadTaskConfiguration {
    pub file_path: Arc<PathBuf>,
    pub url: Arc<String>,
    pub range_download: bool,
    pub range_start: u64,
    pub range_end: u64,
}

pub struct DownloadTask {
    config: DownloadTaskConfiguration,
    handle: DownloadHandle,
}

impl DownloadTask {
    pub fn new(config: DownloadTaskConfiguration) -> DownloadTask {
        let file_path_clone = config.file_path.clone();
        DownloadTask {
            config,
            handle: DownloadHandle::new(file_path_clone),
        }
    }

    pub fn get_downloaded_size(&self) -> u64 {
        self.handle.get_downloaded_size()
    }

    pub async fn start_download(&mut self, options: Arc<Mutex<DownloadOptions>>) -> Result<(), Box<dyn Error + Send>> {
        let mut range_str = String::new();
        if self.config.range_download {
            if self.config.range_start < self.config.range_end {
                range_str = format!("bytes={}-{}", self.config.range_start, self.config.range_end);
            } else {
                range_str = format!("bytes={}-", self.config.range_start);
            }
        }

        let request = reqwest::Client::new().
            get(self.config.url.deref()).
            header(RANGE, range_str);

        let result = request.send().await;

        if options.lock().await.cancel {
            return Ok(());
        }

        match result {
            Ok(response) => {
                match response.error_for_status() {
                    Ok(response) => {
                        if let Err(e) = self.handle.setup().await {
                            return Err(Box::new(e));
                        }

                        let mut body = response.bytes_stream();
                        while let Some(chunk) = body.next().await {
                            if options.lock().await.cancel {
                                return Ok(());
                            }
                            match chunk {
                                Ok(bytes) => {
                                    let buffer = bytes.to_vec() as Vec<u8>;
                                    options.lock().await.downloaded_size += buffer.len() as u64;
                                    self.handle.received_bytes_async(&buffer).await;
                                }
                                Err(e) => {
                                    return Err(Box::new(e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(Box::new(e));
                    }
                }
                Ok(())
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }
}