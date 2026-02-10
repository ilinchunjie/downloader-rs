use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use reqwest::Client;
use tokio_util::sync::CancellationToken;
use crate::download_task::DownloadTask;
use crate::error::DownloadError;
use crate::stream::Stream;
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::rate_limiter::RateLimiter;

/// Represents a single download chunk, either file-backed or in-memory.
pub struct Chunk {
    pub file_path: Option<PathBuf>,
    pub stream: Option<Stream>,
    /// In-memory buffer — uses `Vec::with_capacity` + `extend_from_slice`
    /// instead of zero-fill + Cursor for better performance.
    pub bytes: Option<Vec<u8>>,
    pub chunk_range: ChunkRange,
    pub range_download: bool,
    pub download_in_memory: bool,
    /// Shared global downloaded size counter (same Arc across all chunks of one download).
    pub downloaded_size: Option<Arc<AtomicU64>>,
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
            downloaded_size: None,
            valid: false,
        }
    }
}

impl Chunk {
    pub fn from_file(file_path: PathBuf, chunk_range: ChunkRange, range_download: bool) -> Self {
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

    pub fn set_downloaded_size_counter(&mut self, counter: Arc<AtomicU64>) {
        self.downloaded_size = Some(counter);
    }

    pub fn get_downloaded_size(&self) -> u64 {
        return self.chunk_range.length();
    }

    pub async fn setup(&mut self) -> crate::error::Result<()> {
        match self.download_in_memory {
            true => {
                let bytes = Vec::with_capacity(self.chunk_range.length() as usize);
                self.bytes = Some(bytes);
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
            return self.bytes;
        }
        None
    }

    pub async fn received_bytes_async(&mut self, buffer: &[u8]) -> crate::error::Result<()> {
        let len = buffer.len() as u64;
        match self.download_in_memory {
            true => {
                if let Some(vec) = &mut self.bytes {
                    vec.extend_from_slice(buffer);
                    self.chunk_range.position += len;
                    if let Some(counter) = &self.downloaded_size {
                        counter.fetch_add(len, Ordering::Relaxed);
                    }
                }
            }
            false => {
                if let Some(stream) = &mut self.stream {
                    stream.write_async(buffer).await?;
                    self.chunk_range.position += len;
                    if let Some(counter) = &self.downloaded_size {
                        counter.fetch_add(len, Ordering::Relaxed);
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
        if let Some(path) = &self.file_path {
            if let Ok(exist) = tokio::fs::try_exists(path).await {
                if exist {
                    if let Err(_e) = tokio::fs::remove_file(path).await {
                        return Err(DownloadError::DeleteFile);
                    }
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
    rate_limiter: Arc<RateLimiter>,
) -> crate::error::Result<()> {
    let mut task = DownloadTask::new();
    if let Err(e) = task.start_download(config, client, cancel_token, &mut chunk, rate_limiter).await {
        return Err(e);
    }
    if chunk.download_in_memory {
        let _ = sender.memory_sender.as_ref().unwrap().send(chunk.bytes().unwrap());
    }
    Ok(())
}