use std::ops::Deref;
use std::sync::{Arc};
use tokio::{fs, spawn};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::{chunk, chunk_metadata, file_verify};
use crate::chunk::{Chunk};
use crate::chunk_range::ChunkRange;
use crate::chunk_operation::ChunkOperation;
use crate::download_configuration::DownloadConfiguration;
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::file_verify::FileVerify;

pub struct ChunkHub {
    config: Arc<Mutex<DownloadConfiguration>>,
    chunk_operations: Vec<Arc<Mutex<ChunkOperation>>>,
    chunk_length: usize,
}

impl ChunkHub {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            chunk_length: 0,
            chunk_operations: vec![],
        }
    }

    pub fn start_download(
        &mut self,
        options: Arc<Mutex<DownloadOptions>>,
        chunks: Vec<Chunk>,
    ) -> Vec<JoinHandle<crate::error::Result<()>>> {
        let mut handles: Vec<JoinHandle<crate::error::Result<()>>> = vec![];
        for chunk in chunks {
            let handle = spawn(start_download_chunks(
                self.config.clone(),
                chunk,
                options.clone(),
            ));
            handles.push(handle);
        }
        handles
    }

    pub fn get_chunk_count(&self) -> usize {
        return self.chunk_length;
    }

    pub fn get_chunk_download_progress(&self, index: usize) -> f64 {
        if let Some(operation) = self.chunk_operations.get(index) {
            let downloaded_size = operation.blocking_lock().get_downloaded_size();
            let total_size = operation.blocking_lock().get_total_size();
            return (downloaded_size as f64 / total_size as f64).clamp(0f64, 1f64);
        }
        return 0f64;
    }

    pub fn get_downloaded_size(&self) -> u64 {
        let mut downloaded_size = 0u64;
        for operation in &self.chunk_operations {
            downloaded_size += operation.blocking_lock().get_downloaded_size();
        }
        return downloaded_size;
    }

    pub async fn validate(&mut self) -> Vec<Chunk> {
        let config = self.config.lock().await;
        let mut chunk_count = 1;
        if config.support_range_download && config.chunk_download {
            chunk_count = (config.total_length as f64 / config.chunk_size as f64).ceil() as usize;
            chunk_count = chunk_count.max(1);
        }

        let version = chunk_metadata::get_local_version(config.get_file_path()).await;

        let mut chunks: Vec<Chunk> = Vec::with_capacity(chunk_count);
        let chunk_ranges = ChunkRange::from_chunk_count(config.total_length, chunk_count as u64, config.chunk_size);

        for i in 0..chunk_count {
            let file_path = format!("{}.chunk{}", config.get_file_path(), i);
            let mut chunk = Chunk::new(
                file_path,
                chunk_ranges.get(i).unwrap().clone(),
                 config.support_range_download,
            );
            if version == 0 ||version != config.remote_version {
                chunk.delete_chunk_file().await;
            } else {
                chunk.validate().await;
            }
            let chunk_operation = Arc::new(Mutex::new(ChunkOperation::with_chunk(&chunk.chunk_range)));
            chunk.set_chunk_operation(chunk_operation.clone());
            self.chunk_operations.push(chunk_operation);
            chunks.push(chunk);
        }
        self.chunk_length = chunk_count as usize;

        drop(config);

        self.save_local_version().await;

        chunks
    }

    async fn save_local_version(&mut self) {
        let config = self.config.lock().await;
        chunk_metadata::save_local_version(config.get_file_path(), config.remote_version).await;
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
    chunk: Chunk,
    options: Arc<Mutex<DownloadOptions>>,
) -> crate::error::Result<()> {
    if chunk.valid {
        return Ok(());
    }
    let url = config.lock().await.url.as_ref().unwrap().clone();
    chunk::start_download(url, chunk, options).await?;
    Ok(())
}