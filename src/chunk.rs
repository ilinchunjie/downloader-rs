use std::sync::{Arc};
use reqwest::Client;
use tokio::sync::Mutex;
use tokio::sync::watch::Sender;
use crate::download_task::{DownloadTask};
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::stream::Stream;
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;

pub struct Chunk {
    pub file_path: String,
    pub stream: Option<Stream>,
    pub chunk_range: ChunkRange,
    pub range_download: bool,
    pub downloaded_size_sender: Option<Sender<u64>>,
    pub valid: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            file_path: String::new(),
            stream: None,
            chunk_range: ChunkRange::default(),
            range_download: false,
            downloaded_size_sender: None,
            valid: false,
        }
    }
}

impl Chunk {
    pub fn new(file_path: String, chunk_range: ChunkRange, range_download: bool, downloaded_size_sender: Sender<u64>) -> Self {
        Self {
            file_path,
            chunk_range,
            range_download,
            downloaded_size_sender: Some(downloaded_size_sender),
            ..Default::default()
        }
    }

    pub fn get_downloaded_size(&self) -> u64 {
        return self.chunk_range.length();
    }

    pub async fn setup(&mut self) -> crate::error::Result<()> {
        let stream = Stream::new(&self.file_path, self.range_download).await?;
        self.stream = Some(stream);
        Ok(())
    }

    pub async fn received_bytes_async(&mut self, buffer: &[u8]) -> crate::error::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.write_async(buffer).await?;
            self.chunk_range.position += buffer.len() as u64;
            if let Some(sender) = &self.downloaded_size_sender {
                sender.send(self.chunk_range.length()).unwrap();
            }
        }
        Ok(())
    }

    pub async fn flush_async(&mut self) -> crate::error::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.flush_async().await?;
        }
        Ok(())
    }

    pub async fn delete_chunk_file(&self) -> crate::error::Result<()> {
        if let Ok(exist) = tokio::fs::try_exists(&self.file_path).await {
            if exist {
                if let Err(_e) = tokio::fs::remove_file(&self.file_path).await {
                    return Err(DownloadError::DeleteFile);
                }
            }
        }

        Ok(())
    }

    pub async fn validate(&mut self) -> u8 {
        if self.chunk_range.end == 0 {
            self.valid = false;
            return 1;
        }

        let metadata = tokio::fs::metadata(&self.file_path).await;
        if let Ok(metadata) = metadata {
            if metadata.len() > self.chunk_range.chunk_length() {
                self.valid = false;
                return 2;
            }

            self.chunk_range.set_position(self.chunk_range.start + metadata.len());
            self.valid = self.chunk_range.eof();
            return 0;
        }

        self.valid = false;
        return 3;
    }
}

pub async fn start_download(
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    chunk: Arc<Mutex<Chunk>>,
    options: Arc<Mutex<DownloadOptions>>,
) -> crate::error::Result<()> {
    let mut task = DownloadTask::new();
    task.start_download(config, client, options, chunk).await
}