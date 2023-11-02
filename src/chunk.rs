use std::ops::{DerefMut};
use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::chunk_metadata::ChunkMetadata;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};
use crate::downloader::DownloadOptions;

#[derive(Copy, Clone)]
pub struct ChunkRange {
    pub start: u64,
    pub end: u64,
    pub position: u64,
}

impl ChunkRange {
    pub fn from_start_end(start: u64, end: u64) -> ChunkRange {
        ChunkRange {
            start,
            end,
            position: start,
        }
    }

    pub fn chunk_length(&self) -> u64 {
        if self.end <= self.start {
            return 0u64;
        }
        return self.end - self.start + 1;
    }

    pub fn length(&self) -> u64 {
        return self.position - self.start;
    }

    pub fn set_position(&mut self, position: u64) {
        self.position = position;
    }

    pub fn end(&self) -> bool {
        return self.position == self.end + 1;
    }
}

pub struct Chunk {
    pub download_handle: Arc<Mutex<DownloadHandle>>,
    pub chunk_metadata: Option<Arc<Mutex<ChunkMetadata>>>,
    pub chunk_range: ChunkRange,
    pub range_download: bool,
    pub index: u16,
    pub version: i64,
    pub valid: bool,
}

impl Chunk {
    pub async fn setup(&mut self) -> crate::error::Result<()> {
        match self.download_handle.lock().await.deref_mut() {
            DownloadHandle::File(download_handle) => {
                self.chunk_metadata.as_mut().unwrap().lock().await.update_chunk_version(self.version).await?;
                download_handle.setup().await?;
            }
            DownloadHandle::Memory(download_handle) => {
                download_handle.setup().await?;
            }
        }
        Ok(())
    }

    pub async fn received_bytes_async(&mut self, buffer: &Vec<u8>) -> crate::error::Result<()> {
        match self.download_handle.lock().await.deref_mut() {
            DownloadHandle::File(download_handle) => {
                download_handle.received_bytes_async(self.chunk_range.position, buffer).await?;
                download_handle.flush_async().await?;
                self.chunk_range.position += buffer.len() as u64;
                self.chunk_metadata.as_mut().unwrap().lock().await.update_chunk_position(self.chunk_range.position, self.index).await?;
            }
            DownloadHandle::Memory(download_handle) => {
                download_handle.received_bytes_async(self.chunk_range.position, buffer).await?;
                self.chunk_range.position += buffer.len() as u64;
            }
        }
        Ok(())
    }

    pub async fn set_downloaded_size(&mut self) {
        match self.download_handle.lock().await.deref_mut() {
            DownloadHandle::File(download_handle) => {
                download_handle.update_downloaded_size(self.chunk_range.position - self.chunk_range.start);
            }
            DownloadHandle::Memory(download_handle) => {
                download_handle.update_downloaded_size(self.chunk_range.position - self.chunk_range.start);
            }
        }
    }

    pub async fn validate(&mut self) {
        self.chunk_range.position = self.chunk_range.start;

        if self.chunk_range.end == 0 {
            self.valid = false;
            return;
        }

        let chunk_metadata = self.chunk_metadata.as_mut().unwrap().lock().await;
        if chunk_metadata.version == 0 || chunk_metadata.version != self.version {
            self.valid = false;
            return;
        }

        if chunk_metadata.chunk_positions.len() <= self.index as usize {
            self.valid = false;
            return;
        }

        let position = chunk_metadata.chunk_positions.get(self.index as usize).unwrap();
        if position == &0 {
            self.valid = false;
            return;
        }

        let chunk_length = position - self.chunk_range.start;
        if chunk_length > self.chunk_range.chunk_length() {
            self.valid = false;
            return;
        }

        self.chunk_range.set_position(self.chunk_range.start + chunk_length);
        self.valid = self.chunk_range.end();
    }

    pub fn chunk_range(&self) -> ChunkRange {
        return self.chunk_range;
    }
}

pub async fn start_download(
    url: Arc<String>,
    chunk: Arc<Mutex<Chunk>>,
    options: Arc<Mutex<DownloadOptions>>,
) -> crate::error::Result<()> {
    let lock_chunk = chunk.lock().await;
    let config = DownloadTaskConfiguration {
        range_download: lock_chunk.range_download,
        range_start: lock_chunk.chunk_range.position,
        range_end: lock_chunk.chunk_range.end,
        url,
    };
    drop(lock_chunk);

    let mut task = DownloadTask::new(config);
    task.start_download(options, chunk.clone()).await
}