use std::ops::Deref;
use std::sync::{Arc};
use tokio::{fs, spawn};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::{chunk, chunk_metadata};
use crate::chunk::{Chunk, ChunkRange};
use crate::download_configuration::DownloadConfiguration;
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::file_verify::FileVerify;

pub struct ChunkHub {
    config: Arc<Mutex<DownloadConfiguration>>,
    chunk_length: usize,
    chunks: Option<Vec<Arc<Mutex<Chunk>>>>,
}

impl ChunkHub {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            chunk_length: 0,
            chunks: None,
        }
    }

    pub async fn save_local_version(&self) {
        let config = self.config.lock().await;
        chunk_metadata::save_local_version(config.get_file_path(), config.remote_version).await;
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
        handles
    }

    pub fn get_chunk_count(&self) -> usize {
        return self.chunk_length;
    }

    pub fn get_chunk_range(&self, index: usize) -> Option<ChunkRange> {
        if let Some(chunks) = &self.chunks {
            if let Some(chunk) = chunks.get(index) {
                return Some(chunk.blocking_lock().chunk_range());
            }
        }
        return None;
    }

    pub fn get_downloaded_size(&self) -> u64 {
        let mut downloaded_size = 0u64;
        if let Some(chunks) = &self.chunks {
            for chunk in chunks {
                downloaded_size += chunk.blocking_lock().get_downloaded_size();
            }
        }
        return downloaded_size;
    }

    pub async fn validate(&mut self) {
        self.chunks = None;
        let config = self.config.lock().await;
        let mut chunk_count = 1;
        if config.support_range_download && config.chunk_download {
            chunk_count = (config.total_length as f64 / config.chunk_size as f64).ceil() as u16;
            chunk_count = chunk_count.max(1);
        }

        let version = chunk_metadata::get_local_version(config.get_file_path()).await;

        let mut chunks: Vec<Arc<Mutex<Chunk>>> = Vec::with_capacity(chunk_count as usize);

        for i in 0..chunk_count {
            let start_position = (i as u64 * config.chunk_size) as u64;
            let mut end_position = start_position + config.chunk_size - 1;
            if i == chunk_count - 1 {
                if config.total_length == 0 {
                    end_position = 0;
                } else {
                    end_position = start_position + config.total_length % config.chunk_size - 1;
                }
            }
            let file_path = format!("{}.chunk{}", config.get_file_path(), i);
            let mut chunk = Chunk::new(
                file_path,
                ChunkRange::from_start_end(start_position, end_position),
                config.support_range_download,
            );
            if version != config.remote_version {
                chunk.delete_chunk_file().await;
            } else {
                chunk.validate().await;
            }

            let chunk = Arc::new(Mutex::new(chunk));
            chunks.push(chunk);
        }
        self.chunk_length = chunk_count as usize;
        self.chunks = Some(chunks);
    }

    pub async fn calculate_file_hash(&self) -> crate::error::Result<()> {
        let config = self.config.lock().await;

        if config.file_verify == FileVerify::None {
            return Ok(());
        }

        match &config.file_verify {
            FileVerify::None => {
                return Ok(());
            }
            #[cfg(feature = "xxhash-rust")]
            FileVerify::xxHash(value) => {
                {
                    let hash = file_verify::calculate_file_xxhash(config.get_file_path(), 0).await?;
                    if !hash.eq(&value) {
                        return Err(DownloadError::FileVerify);
                    }
                }
            }
            #[cfg(feature = "md5")]
            FileVerify::MD5(value) => {
                {
                    let hash = file_verify::calculate_file_md5(config.get_file_path()).await?;
                    if !hash.eq(value) {
                        return Err(DownloadError::FileVerify);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn on_download_post(&mut self) -> crate::error::Result<()> {
        let config = self.config.lock().await;
        let mut output = OpenOptions::new().create(true).write(true).open(config.get_file_path()).await;
        if let Ok(file) = &mut output {
            let chunk_count = self.get_chunk_count();
            let mut buffer = vec![0; 8192];
            for i in 0..chunk_count {
                let chunk_path = format!("{}.chunk{}", config.get_file_path(), i);
                if let Ok(chunk_file) = &mut tokio::fs::File::open(chunk_path).await {
                    loop {
                        if let Ok(len) = chunk_file.read(&mut buffer).await {
                            if len == 0 {
                                break;
                            }
                            file.write(&buffer[0..len]).await;
                        } else {
                            return Err(DownloadError::FileWrite);
                        }
                    }
                }
            }
            file.flush().await;

            for i in 0..chunk_count {
                let chunk_path = format!("{}.chunk{}", config.get_file_path(), i);
                fs::remove_file(chunk_path).await;
            }
        }

        {
            chunk_metadata::delete_metadata(config.get_file_path()).await?;
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
        let chunk = chunk.lock().await;
        if chunk.valid {
            return Ok(());
        }
    }

    let url = config.lock().await.url.as_ref().unwrap().clone();
    chunk::start_download(url, chunk, options).await?;
    Ok(())
}