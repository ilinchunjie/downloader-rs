use std::sync::{Arc};
use reqwest::Client;
use tokio::{fs, spawn};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::sync::watch::{Receiver, channel};
use tokio::task::JoinHandle;
#[allow(unused_imports)]
use crate::{chunk, chunk_metadata, file_verify};
use crate::chunk::{Chunk};
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::file_verify::FileVerify;
use crate::remote_file::RemoteFile;

pub struct ChunkHub {
    config: Arc<DownloadConfiguration>,
    chunks: Vec<Arc<Mutex<Chunk>>>,
    chunk_length: usize,
}

impl ChunkHub {
    pub fn new(config: Arc<DownloadConfiguration>) -> Self {
        Self {
            config,
            chunk_length: 0,
            chunks: vec![],
        }
    }

    pub fn start_download(
        &mut self,
        client: Arc<Client>,
        options: Arc<Mutex<DownloadOptions>>,
    ) -> Vec<JoinHandle<crate::error::Result<()>>> {
        let mut handles: Vec<JoinHandle<crate::error::Result<()>>> = Vec::with_capacity(self.chunk_length);
        for chunk in &self.chunks {
            let handle = spawn(start_download_chunks(
                self.config.clone(),
                client.clone(),
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

    pub async fn validate(&mut self, remote_file: RemoteFile) -> crate::error::Result<Vec<Receiver<u64>>> {
        let mut chunk_count = 1;
        if self.config.range_download && remote_file.support_range_download && self.config.chunk_download {
            chunk_count = (remote_file.total_length as f64 / self.config.chunk_size as f64).ceil() as usize;
            chunk_count = chunk_count.max(1);
        }

        let version = chunk_metadata::get_local_version(self.config.get_file_path()).await;
        let remote_version = match self.config.remote_version {
            0 => remote_file.last_modified_time,
            _ => self.config.remote_version
        };

        let chunk_ranges = ChunkRange::from_chunk_count(remote_file.total_length, chunk_count as u64, self.config.chunk_size);

        let mut downloaded_size_receivers: Vec<Receiver<u64>> = Vec::with_capacity(chunk_count);

        for i in 0..chunk_count {
            let file_path = format!("{}.chunk{}", self.config.get_file_path(), i);
            let (sender, receiver) = channel(0u64);
            let mut chunk = Chunk::new(
                file_path,
                chunk_ranges.get(i).unwrap().clone(),
                self.config.range_download && remote_file.support_range_download,
                sender,
            );
            if version == 0 || version != remote_version {
                chunk.delete_chunk_file().await?;
            } else {
                match chunk.validate().await {
                    2 => chunk.delete_chunk_file().await?,
                    _ => {},
                }
            }
            downloaded_size_receivers.push(receiver);
            self.chunks.push(Arc::new(Mutex::new(chunk)));
        }
        self.chunk_length = chunk_count as usize;

        self.save_local_version(remote_version).await?;

        Ok(downloaded_size_receivers)
    }

    async fn save_local_version(&mut self, version: i64) -> crate::error::Result<()> {
        chunk_metadata::save_local_version(self.config.get_file_path(), version).await
    }

    pub async fn calculate_file_hash(&self) -> crate::error::Result<()> {
        if self.config.file_verify == FileVerify::None {
            return Ok(());
        }

        match &self.config.file_verify {
            FileVerify::None => {
                return Ok(());
            }
            #[cfg(feature = "xxhash-rust")]
            FileVerify::xxHash(value) => {
                {
                    let hash = file_verify::calculate_file_xxhash(self.config.get_file_path(), 0).await?;
                    if !hash.eq(&value) {
                        return Err(DownloadError::FileVerify);
                    }
                }
            }
            #[cfg(feature = "md5")]
            FileVerify::MD5(value) => {
                {
                    let hash = file_verify::calculate_file_md5(self.config.get_file_path()).await?;
                    if !hash.eq(value) {
                        return Err(DownloadError::FileVerify);
                    }
                }
            }
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn on_download_post(&mut self) -> crate::error::Result<()> {
        match self.chunk_length {
            1 => {
                let chunk_path = format!("{}.chunk{}", self.config.get_file_path(), 0);
                if let Err(e) = fs::rename(&chunk_path, self.config.get_file_path()).await {
                    return Err(DownloadError::FileRename(format!("文件重命名失败 {}", e)));
                }
            }
            _ => {
                let mut output = OpenOptions::new().create(true).write(true).open(self.config.get_file_path()).await;
                if let Ok(file) = &mut output {
                    let mut buffer = vec![0; 8192];
                    for i in 0..self.chunk_length {
                        let chunk_path = format!("{}.chunk{}", self.config.get_file_path(), i);
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
                        let chunk_path = format!("{}.chunk{}", self.config.get_file_path(), i);
                        if let Err(_e) = fs::remove_file(chunk_path).await {
                            return Err(DownloadError::DeleteFile);
                        }
                    }
                }
            }
        }

        {
            chunk_metadata::delete_metadata(self.config.get_file_path()).await?;
        }

        Ok(())
    }
}

async fn start_download_chunks(
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    chunk: Arc<Mutex<Chunk>>,
    options: Arc<Mutex<DownloadOptions>>,
) -> crate::error::Result<()> {
    if chunk.lock().await.valid {
        return Ok(());
    }
    chunk::start_download(config, client, chunk, options).await?;
    Ok(())
}