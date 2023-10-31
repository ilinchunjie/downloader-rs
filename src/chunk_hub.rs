use std::error::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use tokio::{fs, spawn};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::chunk;
use crate::chunk::{Chunk};
use crate::chunk_metadata::ChunkMetadata;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::DownloadHandle;
use crate::downloader::DownloaderStatus::Archive;
use crate::downloader::DownloadOptions;

pub struct ChunkHub {
    config: Arc<Mutex<DownloadConfiguration>>,
    chunks: Option<Vec<Arc<Mutex<Chunk>>>>,
}

impl ChunkHub {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            chunks: None,
        }
    }

    pub fn start_download(
        &mut self,
        options: Arc<Mutex<DownloadOptions>>,
    ) -> Vec<JoinHandle<crate::error::Result<()>>> {
        let mut handles: Vec<JoinHandle<crate::error::Result<()>>> = vec![];
        if let Some(chunks) = &mut self.chunks {
            for chunk in chunks {
                let handle = spawn(start_download_chunks(
                    self.config.clone(),
                    chunk.clone(),
                    options.clone(),
                ));
                handles.push(handle);
            }
        }
        return handles;
    }

    fn from_file_chunk(
        &self,
        download_handle: Arc<Mutex<DownloadHandle>>,
        chunk_metadata: Arc<Mutex<ChunkMetadata>>,
        range_download: bool,
        start: u64,
        end: u64,
        position: u64,
        index: u16,
        version: i64,
        valid: bool) -> Chunk {
        Chunk {
            download_handle,
            chunk_metadata: Some(chunk_metadata),
            range_download,
            start,
            end,
            position,
            index,
            version,
            valid,
        }
    }

    fn from_memory_chunk(
        &self,
        download_handle: Arc<Mutex<DownloadHandle>>,
        range_download: bool,
        start: u64,
        end: u64,
        position: u64,
        index: u16,
        version: i64,
        valid: bool) -> Chunk {
        Chunk {
            download_handle,
            chunk_metadata: None,
            range_download,
            start,
            end,
            position,
            index,
            version,
            valid,
        }
    }

    pub async fn validate(&mut self, download_handle: Arc<Mutex<DownloadHandle>>) {
        self.chunks = None;
        let config = self.config.lock().await;
        let mut chunk_count = 1;
        if config.support_range_download && config.chunk_download {
            chunk_count = (config.total_length as f64 / config.chunk_size as f64).ceil() as u16;
        }

        let mut chunk_metadata: Option<ChunkMetadata>;
        if config.download_in_memory {
            chunk_metadata = None;
        } else {
            let chunk_metadata_path = Arc::new(PathBuf::from(format!("{}.temp.meta", config.path.as_ref().unwrap().deref())));
            let metadata = ChunkMetadata::get_chunk_metadata(chunk_metadata_path, chunk_count).await;
            chunk_metadata = Some(metadata);
        }

        let mut chunks: Vec<Arc<Mutex<Chunk>>> = Vec::with_capacity(chunk_count as usize);
        match chunk_count {
            1 => {
                let mut chunk: Chunk;
                if config.download_in_memory {
                    chunk = self.from_memory_chunk(download_handle.clone(), config.support_range_download, 0, config.total_length - 1, 0, 0, config.remote_version, false);
                } else {
                    let chunk_metadata = Arc::new(Mutex::new(chunk_metadata.unwrap()));
                    chunk = self.from_file_chunk(download_handle.clone(), chunk_metadata, config.support_range_download, 0, config.total_length - 1, 0, 0, config.remote_version, false);
                    chunk.validate().await;
                }
                chunk.set_downloaded_size().await;
                let chunk = Arc::new(Mutex::new(chunk));
                chunks.push(chunk);
            }
            _ => {
                let chunk_metadata = Arc::new(Mutex::new(chunk_metadata.unwrap()));
                for i in 0..chunk_count {
                    let start_position = (i as u64 * config.chunk_size) as u64;
                    let mut end_position = start_position + config.chunk_size - 1;
                    if i == chunk_count - 1 {
                        end_position = start_position + config.total_length % config.chunk_size - 1;
                    }

                    let mut chunk: Chunk;
                    if config.download_in_memory {
                        chunk = self.from_memory_chunk(download_handle.clone(), true, start_position, end_position, start_position, i, config.remote_version, false);
                    } else {
                        chunk = self.from_file_chunk(download_handle.clone(), chunk_metadata.clone(), true, start_position, end_position, start_position, i, config.remote_version, false);
                        chunk.validate().await;
                    }
                    chunk.set_downloaded_size().await;
                    let chunk = Arc::new(Mutex::new(chunk));
                    chunks.push(chunk);
                }
            }
        }
        self.chunks = Some(chunks);
    }
}

async fn start_download_chunks(
    config: Arc<Mutex<DownloadConfiguration>>,
    chunk: Arc<Mutex<Chunk>>,
    options: Arc<Mutex<DownloadOptions>>,
) -> crate::error::Result<()> {
    {
        let mut chunk = chunk.lock().await;
        if chunk.valid {
            return Ok(());
        }
    }

    let url = config.lock().await.url.as_ref().unwrap().clone();
    return chunk::start_download(url, chunk.clone(), options.clone()).await;
}