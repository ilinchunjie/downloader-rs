use std::sync::{Arc};
use tokio::{fs, spawn};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::sync::watch::{Receiver, channel};
use tokio::task::JoinHandle;
use crate::{chunk, chunk_metadata, file_verify};
use crate::chunk::{Chunk};
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::file_verify::FileVerify;

pub struct ChunkHub {
    config: Arc<Mutex<DownloadConfiguration>>,
    chunks: Vec<Arc<Mutex<Chunk>>>,
    chunk_length: usize,
}

impl ChunkHub {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            chunk_length: 0,
            chunks: vec![],
        }
    }

    pub fn start_download(
        &mut self,
        options: Arc<Mutex<DownloadOptions>>,
    ) -> Vec<JoinHandle<crate::error::Result<()>>> {
        let mut handles: Vec<JoinHandle<crate::error::Result<()>>> = Vec::with_capacity(self.chunk_length);
        for chunk in &self.chunks {
            let handle = spawn(start_download_chunks(
                self.config.clone(),
                chunk.clone(),
                options.clone(),
            ));
            handles.push(handle);
        }
        handles
    }

    pub async fn get_downloaded_size(&self) -> u64 {
        let mut downloaded_size = 0u64;
        for chunk in &self.chunks {
            downloaded_size += chunk.lock().await.get_downloaded_size();
        }
        downloaded_size
    }

    pub async fn validate(&mut self) -> crate::error::Result<Vec<Receiver<u64>>> {
        let config = self.config.lock().await;
        let mut chunk_count = 1;
        if config.support_range_download && config.chunk_download {
            chunk_count = (config.total_length as f64 / config.chunk_size as f64).ceil() as usize;
            chunk_count = chunk_count.max(1);
        }

        let version = chunk_metadata::get_local_version(config.get_file_path()).await;

        let chunk_ranges = ChunkRange::from_chunk_count(config.total_length, chunk_count as u64, config.chunk_size);

        let mut downloaded_size_receivers : Vec<Receiver<u64>> = Vec::with_capacity(chunk_count);

        for i in 0..chunk_count {
            let file_path = format!("{}.chunk{}", config.get_file_path(), i);
            let (sender, receiver) = channel(0u64);
            let mut chunk = Chunk::new(
                file_path,
                chunk_ranges.get(i).unwrap().clone(),
                 config.support_range_download,
                sender,
            );
            if version == 0 ||version != config.remote_version {
                chunk.delete_chunk_file().await?;
            } else {
                chunk.validate().await;
            }
            downloaded_size_receivers.push(receiver);
            self.chunks.push(Arc::new(Mutex::new(chunk)));
        }
        self.chunk_length = chunk_count as usize;

        drop(config);

        self.save_local_version().await?;

        Ok(downloaded_size_receivers)
    }

    async fn save_local_version(&mut self) -> crate::error::Result<()> {
        let config = self.config.lock().await;
        chunk_metadata::save_local_version(config.get_file_path(), config.remote_version).await
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
        match self.chunk_length {
            1 => {
                let chunk_path = format!("{}.chunk{}", config.get_file_path(), 0);
                if let Err(e) = fs::rename(&chunk_path, config.get_file_path()).await {
                    return Err(DownloadError::FileRename(format!("文件重命名失败 {}", e)));
                }
            }
            _ => {
                let mut output = OpenOptions::new().create(true).write(true).open(config.get_file_path()).await;
                if let Ok(file) = &mut output {
                    let mut buffer = vec![0; 8192];
                    for i in 0..self.chunk_length {
                        let chunk_path = format!("{}.chunk{}", config.get_file_path(), i);
                        if let Ok(chunk_file) = &mut tokio::fs::File::open(chunk_path).await {
                            loop {
                                if let Ok(len) = chunk_file.read(&mut buffer).await {
                                    if len == 0 {
                                        break;
                                    }
                                    if let Err(_e) = file.write(&buffer[0..len]).await {
                                        return Err(DownloadError::FileWrite);
                                    }
                                } else {
                                    return Err(DownloadError::FileWrite);
                                }
                            }
                        }
                    }

                    if let Err(_e) = file.flush().await {
                        return Err(DownloadError::FileFlush);
                    }

                    for i in 0..self.chunk_length {
                        let chunk_path = format!("{}.chunk{}", config.get_file_path(), i);
                        if let Err(_e) = fs::remove_file(chunk_path).await {
                            return Err(DownloadError::DeleteFile);
                        }
                    }
                }
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
    if chunk.lock().await.valid {
        return Ok(());
    }
    let url = config.lock().await.url.as_ref().unwrap().clone();
    chunk::start_download(url, chunk, options).await?;
    Ok(())
}