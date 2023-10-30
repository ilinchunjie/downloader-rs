use std::error::Error;
use std::ops::{DerefMut};
use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::chunk_metadata::ChunkMetadata;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};
use crate::downloader::DownloadOptions;

pub struct Chunk {
    pub download_handle: Arc<Mutex<DownloadHandle>>,
    pub chunk_metadata: Arc<Mutex<ChunkMetadata>>,
    pub range_download: bool,
    pub start: u64,
    pub end: u64,
    pub position: u64,
    pub index: u16,
    pub version: i64,
    pub valid: bool,
}

impl Chunk {
    pub async fn setup(&mut self) -> Result<(), Box<dyn Error + Send>> {
        self.chunk_metadata.lock().await.update_chunk_version(self.version).await;
        match self.download_handle.lock().await.deref_mut() {
            DownloadHandle::File(download_handle) => {
                if let Err(e) = download_handle.setup().await {
                    return Err(e);
                }
            }
            DownloadHandle::Memory(download_handle) => {
                if let Err(e) = download_handle.setup().await {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub async fn received_bytes_async(&mut self, buffer: &Vec<u8>) -> Result<(), Box<dyn Error + Send>> {
        match self.download_handle.lock().await.deref_mut() {
            DownloadHandle::File(download_handle) => {
                if let Err(e) = download_handle.received_bytes_async(self.position, buffer).await {
                    return Err(e);
                }
                self.position += buffer.len() as u64;
                self.chunk_metadata.lock().await.update_chunk_position(self.position, self.index).await;
            }
            DownloadHandle::Memory(download_handle) => {
                if let Err(e) = download_handle.received_bytes_async(self.position, buffer).await {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub async fn validate(&mut self) {
        let chunk_metadata = self.chunk_metadata.lock().await;
        if chunk_metadata.version == 0 || chunk_metadata.version != self.version {
            self.valid = false;
            println!("version != self.version");
            return;
        }

        if chunk_metadata.chunk_positions.len() <= self.index as usize {
            self.valid = false;
            println!("chunk_positions.len() <= chunk_index");
            return;
        }

        let position = chunk_metadata.chunk_positions.get(self.index as usize).unwrap();
        if position == &0 {
            self.valid = false;
            println!("position == 0");
            return;
        }

        let chunk_length = position - self.start;
        let remote_length = self.end - self.start + 1;
        if chunk_length > remote_length {
            self.valid = false;
            println!("chunk_length > remote_length");
            return;
        }
        self.position = self.start + chunk_length;
        self.valid = self.position == self.end + 1;

        println!("valid {}", self.valid);
    }
}

pub async fn start_download(
    url: Arc<String>,
    chunk: Arc<Mutex<Chunk>>,
    options: Arc<Mutex<DownloadOptions>>,
) -> Result<(), Box<dyn Error + Send>> {
    let lock_chunk = chunk.lock().await;
    let config = DownloadTaskConfiguration {
        range_download: lock_chunk.range_download,
        range_start: lock_chunk.position,
        range_end: lock_chunk.end,
        url,
    };
    drop(lock_chunk);

    let mut task = DownloadTask::new(config);
    task.start_download(options, chunk.clone()).await
}