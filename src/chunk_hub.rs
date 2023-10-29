use std::cell::RefCell;
use std::error::Error;
use std::fmt::format;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc};
use futures::stream::try_unfold;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{fs, spawn};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::chunk::{Chunk, ChunkMetadata};
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::DownloadHandle;
use crate::downloader::DownloadOptions;

pub struct ChunkHub {
    config: Arc<Mutex<DownloadConfiguration>>,
    chunk_metadata: Option<Arc<Mutex<ChunkMetadata>>>,
    chunks: Option<Vec<Arc<Mutex<Chunk>>>>,
}

impl ChunkHub {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            chunk_metadata: None,
            chunks: None,
        }
    }

    pub fn start_download(
        &mut self,
        options: Arc<Mutex<DownloadOptions>>,
        download_handle: Arc<Mutex<DownloadHandle>>,
    ) -> Vec<JoinHandle<Result<(), Box<dyn Error + Send>>>> {
        let mut handles: Vec<JoinHandle<Result<(), Box<dyn Error + Send>>>> = vec![];
        if let Some(chunks) = &mut self.chunks {
            let chunk_metadata = self.chunk_metadata.as_ref().unwrap();
            for chunk in chunks {
                let handle = spawn(start_download_chunks(
                    self.config.clone(),
                    chunk.clone(),
                    options.clone(),
                    download_handle.clone(),
                    chunk_metadata.clone()
                ));
                handles.push(handle);
            }
        }
        return handles;
    }

    pub async fn validate(&mut self) {
        self.chunks = None;
        let config = self.config.lock().await;
        let mut chunk_count = 1;
        if config.support_range_download && config.chunk_download {
            chunk_count = (config.total_length as f64 / config.chunk_size as f64).ceil() as u16;
        }

        let chunk_metadata_path = Arc::new(PathBuf::from(format!("{}.chunk.meta", config.path.as_ref().unwrap().deref())));
        let chunk_metadata = ChunkMetadata::get_chunk_metadata(chunk_metadata_path, chunk_count).await;

        let mut chunks: Vec<Arc<Mutex<Chunk>>> = Vec::with_capacity(chunk_count as usize);
        match chunk_count {
            1 => {
                let mut chunk = Chunk {
                    range_download: config.support_range_download,
                    start: 0,
                    end: config.total_length - 1,
                    index: 0,
                    version: config.remote_version,
                    valid: false,
                };
                chunk.validate(&chunk_metadata, 0).await;
                let chunk = Arc::new(Mutex::new(chunk));
                chunks.push(chunk);
            }
            _ => {
                for i in 0..chunk_count {
                    let start_position = (i as u64 * config.chunk_size) as u64;
                    let mut end_position = start_position + config.chunk_size - 1;
                    if i == chunk_count - 1 {
                        end_position = start_position + config.total_length % config.chunk_size - 1;
                    }
                    let mut chunk = Chunk {
                        range_download: true,
                        start: start_position,
                        end: end_position,
                        index: i,
                        version: config.remote_version,
                        valid: false,
                    };
                    chunk.validate(&chunk_metadata, i as usize).await;
                    let chunk = Arc::new(Mutex::new(chunk));
                    chunks.push(chunk);
                }
            }
        }
        self.chunks = Some(chunks);
        self.chunk_metadata = Some(Arc::new(Mutex::new(chunk_metadata)));
    }
}

async fn start_download_chunks(
    config: Arc<Mutex<DownloadConfiguration>>,
    chunk: Arc<Mutex<Chunk>>,
    options: Arc<Mutex<DownloadOptions>>,
    download_handle: Arc<Mutex<DownloadHandle>>,
    chunk_metadata: Arc<Mutex<ChunkMetadata>>,
) -> Result<(), Box<dyn Error + Send>> {
    let mut chunk = chunk.lock().await;
    if !chunk.valid {
        return chunk.start_download(
            config.lock().await.url.as_ref().unwrap().clone(),
            options.clone(),
            chunk_metadata.clone(),
            download_handle.clone(),
        ).await;
    }
    Ok(())
}