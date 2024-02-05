use std::io::Cursor;
use std::sync::{Arc};
use reqwest::Client;
use tokio::sync::watch::Sender;
use tokio_util::sync::CancellationToken;
use crate::download_task::{DownloadTask};
use crate::error::DownloadError;
use crate::stream::Stream;
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;

pub struct Chunk {
    pub file_path: Option<String>,
    pub stream: Option<Stream>,
    pub bytes: Option<Cursor<Vec<u8>>>,
    pub chunk_range: ChunkRange,
    pub range_download: bool,
    pub download_in_memory: bool,
    pub downloaded_size_sender: Option<Sender<u64>>,
    pub valid: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            file_path: None,
            stream: None,
            bytes: None,
            chunk_range: ChunkRange::default(),
            range_download: false,
            download_in_memory: false,
            downloaded_size_sender: None,
            valid: false,
        }
    }
}

impl Chunk {
    pub fn from_file(file_path: String, chunk_range: ChunkRange, range_download: bool) -> Self {
        Self {
            file_path: Some(file_path),
            chunk_range,
            range_download,
            ..Default::default()
        }
    }

    pub fn from_memory(chunk_range: ChunkRange) -> Self {
        Self {
            chunk_range,
            download_in_memory: true,
            ..Default::default()
        }
    }

    pub fn set_downloaded_size_sender(&mut self, sender: Sender<u64>) {
        self.downloaded_size_sender = Some(sender);
    }

    pub fn get_downloaded_size(&self) -> u64 {
        return self.chunk_range.length();
    }

    pub async fn setup(&mut self) -> crate::error::Result<()> {
        match self.download_in_memory {
            true => {
                let bytes = vec![0u8; self.chunk_range.length() as usize];
                self.bytes = Some(Cursor::new(bytes));
            }
            false => {
                let stream = Stream::new(self.file_path.as_ref().unwrap(), self.range_download).await?;
                self.stream = Some(stream);
            }
        }

        Ok(())
    }

    pub fn bytes(self) -> Option<Vec<u8>> {
        if self.download_in_memory {
            return Some(self.bytes.unwrap().into_inner());
        }
        None
    }

    pub async fn received_bytes_async(&mut self, buffer: &[u8]) -> crate::error::Result<()> {
        match self.download_in_memory {
            true => {
                if let Some(cursor) = &mut self.bytes {
                    if let Err(e) = std::io::Write::write_all(cursor, buffer) {
                        return Err(DownloadError::MemoryWrite);
                    }
                    self.chunk_range.position += buffer.len() as u64;
                    if let Some(sender) = &self.downloaded_size_sender {
                        let _ = sender.send(self.chunk_range.length());
                    }
                }
            }
            false => {
                if let Some(stream) = &mut self.stream {
                    stream.write_async(buffer).await?;
                    self.chunk_range.position += buffer.len() as u64;
                    if let Some(sender) = &self.downloaded_size_sender {
                        let _ = sender.send(self.chunk_range.length());
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn flush_async(&mut self) -> crate::error::Result<()> {
        if !self.download_in_memory {
            if let Some(stream) = &mut self.stream {
                stream.flush_async().await?;
            }
        }
        Ok(())
    }

    pub async fn delete_chunk_file(&self) -> crate::error::Result<()> {
        if let Ok(exist) = tokio::fs::try_exists(self.file_path.as_ref().unwrap()).await {
            if exist {
                if let Err(_e) = tokio::fs::remove_file(self.file_path.as_ref().unwrap()).await {
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

        let metadata = tokio::fs::metadata(self.file_path.as_ref().unwrap()).await;
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
    mut chunk: Chunk,
    sender: Arc<DownloadSender>,
    cancel_token: CancellationToken,
) -> crate::error::Result<()> {
    let mut task = DownloadTask::new();
    if let Err(e) = task.start_download(config, client, cancel_token, &mut chunk).await {
        return Err(e);
    }
    if chunk.download_in_memory {
        let _ = sender.memory_sender.as_ref().unwrap().send(chunk.bytes().unwrap());
    }
    Ok(())
}