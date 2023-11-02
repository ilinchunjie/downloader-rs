use std::cmp::max;
use std::error::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use tokio::{fs, spawn};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::{chunk, file_verify};
use crate::chunk::{Chunk, ChunkRange};
use crate::chunk_metadata::ChunkMetadata;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::DownloadHandle;
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::file_verify::FileVerify;

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

    pub fn get_chunk_count(&self) -> usize {
        if let Some(chunks) = &self.chunks {
            return chunks.len()
        }
        return 0;
    }

    pub fn get_chunk_range(&self, index: usize) -> Option<ChunkRange> {
        if let Some(chunks) = &self.chunks {
            if let Some(chunk) = chunks.get(index) {
                return Some(chunk.blocking_lock().chunk_range());
            }
        }
        return None;
    }

    fn from_file_chunk(&self, download_handle: Arc<Mutex<DownloadHandle>>, chunk_metadata: Arc<Mutex<ChunkMetadata>>, range_download: bool, chunk_range: ChunkRange, index: u16, version: i64, valid: bool) -> Chunk {
        Chunk {
            download_handle,
            chunk_metadata: Some(chunk_metadata),
            range_download,
            chunk_range,
            index,
            version,
            valid,
        }
    }

    fn from_memory_chunk(&self, download_handle: Arc<Mutex<DownloadHandle>>, range_download: bool, chunk_range: ChunkRange, index: u16, version: i64, valid: bool) -> Chunk {
        Chunk {
            download_handle,
            chunk_metadata: None,
            range_download,
            chunk_range,
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
            chunk_count = chunk_count.max(1);
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
                let mut end = 0u64;
                if config.total_length > 0 {
                    end = config.total_length - 1;
                }
                let chunk_range = ChunkRange::from_start_end(0, end);
                if config.download_in_memory {
                    chunk = self.from_memory_chunk(download_handle.clone(), config.support_range_download, chunk_range, 0, config.remote_version, false);
                } else {
                    let chunk_metadata = Arc::new(Mutex::new(chunk_metadata.unwrap()));
                    chunk = self.from_file_chunk(download_handle.clone(), chunk_metadata, config.support_range_download, chunk_range, 0, config.remote_version, false);
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
                    let chunk_range = ChunkRange::from_start_end(start_position, end_position);
                    let mut chunk: Chunk;
                    if config.download_in_memory {
                        chunk = self.from_memory_chunk(download_handle.clone(), true, chunk_range, i, config.remote_version, false);
                    } else {
                        chunk = self.from_file_chunk(download_handle.clone(), chunk_metadata.clone(), true, chunk_range, i, config.remote_version, false);
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

    pub async fn calculate_file_hash(&self) -> crate::error::Result<()> {
        let config = self.config.lock().await;

        if config.file_verify == FileVerify::None {
            return Ok(());
        }

        let mut temp_file_path: String;
        if config.create_temp_file {
            temp_file_path = format!("{}.temp", config.path.as_ref().unwrap().deref());
        } else {
            temp_file_path = config.path.as_ref().unwrap().deref().to_string();
        }

        match &config.file_verify {
            FileVerify::None => {
                return Ok(());
            }
            #[cfg(feature = "xxhash-rust")]
            FileVerify::xxHash(value) => {
                {
                    let hash = file_verify::calculate_file_xxhash(&temp_file_path, 0).await?;
                    if !hash.eq(&value) {
                        return Err(DownloadError::FileVerify);
                    }
                }
            }
            #[cfg(feature = "md5")]
            FileVerify::MD5(value) => {
                {
                    let hash = file_verify::calculate_file_md5(&temp_file_path).await?;
                    if !hash.eq(value) {
                        return Err(DownloadError::FileVerify);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn on_download_post(&self) -> crate::error::Result<()> {
        let config = self.config.lock().await;
        if config.create_temp_file {
            let temp_file = format!("{}.temp", config.path.as_ref().unwrap().deref());
            if let Err(e) = fs::rename(temp_file, config.path.as_ref().unwrap().deref()).await {
                return Err(DownloadError::FileRename);
            };
        }

        {
            let meta_file = format!("{}.temp.meta", config.path.as_ref().unwrap().deref());
            if let Err(e) = fs::remove_file(meta_file).await {
                return Err(DownloadError::RemoveMetaFile);
            };
        }

        Ok(())
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