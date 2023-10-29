use std::error::Error;
use std::fmt::{Debug};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures::StreamExt;
use reqwest::header::RANGE;
use tokio::sync::Mutex;
use crate::chunk::ChunkMetadata;
use crate::download_handle::DownloadHandle;
use crate::download_handle_file::DownloadHandleFile;
use crate::downloader::DownloadOptions;

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
        download_handle: Arc<Mutex<DownloadHandle>>,
        chunk_metadata: Arc<Mutex<ChunkMetadata>>,
        chunk_index: u16
    ) -> Result<(), Box<dyn Error + Send>> {
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
                        match download_handle.lock().await.deref_mut() {
                            DownloadHandle::DownloadHandleFile(download_handle) => {
                                if let Err(e) = download_handle.setup().await {
                                    return Err(Box::new(e));
                                }
                            }
                            DownloadHandle::DownloadHandleMemory(download_handle) => {
                                if let Err(e) = download_handle.setup().await {
                                    return Err(Box::new(e));
                                }
                            }
                        }


                        let mut body = response.bytes_stream();
                        while let Some(chunk) = body.next().await {
                            if options.lock().await.cancel {
                                return Ok(());
                            }
                            match chunk {
                                Ok(bytes) => {
                                    let buffer = bytes.to_vec() as Vec<u8>;
                                    match download_handle.lock().await.deref_mut() {
                                        DownloadHandle::DownloadHandleFile(download_handle) => {
                                            if let Err(e) = download_handle.received_bytes_async(&buffer).await {
                                                return Err(Box::new(e));
                                            }
                                            chunk_metadata.lock().await.save_and_flush_chunk_metadata(download_handle.get_downloaded_size(), chunk_index).await;
                                        }
                                        DownloadHandle::DownloadHandleMemory(download_handle) => {
                                            if let Err(e) = download_handle.received_bytes_async(&buffer).await {
                                                return Err(Box::new(e));
                                            }
                                        }
                                    }

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