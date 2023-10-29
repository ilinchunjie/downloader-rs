use std::error::Error;
use std::io::SeekFrom;
use std::mem;
use std::mem::size_of;
use std::ops::Deref;
use std::os::unix::raw::ino_t;
use std::path::{Path, PathBuf};
use std::sync::{Arc};
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use crate::download_handle::DownloadHandle;
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};
use crate::downloader::DownloadOptions;

pub struct ChunkMetadata {
    pub path: Arc<PathBuf>,
    pub file: Option<File>,
    pub version: i64,
    pub chunk_count: u16,
    pub chunk_positions: Vec<u64>,
}

impl ChunkMetadata {
    pub async fn get_chunk_metadata(path: Arc<PathBuf>, chunk_count: u16) -> ChunkMetadata {
        let mut chunk_positions = Vec::with_capacity(chunk_count as usize);
        let file_path = path.as_path();
        let mut chunk_metadata = ChunkMetadata {
            path: path.clone(),
            version: 0,
            file: None,
            chunk_count,
            chunk_positions,
        };

        if let Ok(_) = fs::metadata(file_path).await {
            if let Ok(mut meta_file) = OpenOptions::new().
                read(true).
                write(true).
                open(file_path).await {
                let mut chunk_count = 0u16;
                if let Ok(version) = meta_file.read_i64_le().await {
                    chunk_metadata.version = version;
                }
                if let Ok(count) = meta_file.read_u16_le().await {
                    if count != chunk_count {
                        return chunk_metadata;
                    }
                }
                for i in 0..chunk_count {
                    let mut chunk_position = 0u64;
                    if let Ok(position) = meta_file.read_u64_le().await {
                        chunk_position = position;
                    }
                    chunk_metadata.chunk_positions.push(chunk_position);
                }
                let length = (size_of::<i64>() + size_of::<u16>() + chunk_count as usize * size_of::<u64>()) as u64;
                meta_file.set_len(length).await;
                chunk_metadata.file = Some(meta_file);
            }
        }
        chunk_metadata
    }

    pub async fn save_and_flush_chunk_metadata(&mut self, position: u64, chunk_index: u16) {
        if let Some(meta_file) = &mut self.file {
            let version_length = mem::size_of::<i64>();
            let count_length = mem::size_of::<u16>();
            let seek_position = version_length + count_length + chunk_index as usize * size_of::<u64>();
            meta_file.seek(SeekFrom::Start(seek_position as u64)).await;
            meta_file.write(&position.to_le_bytes()).await;
            meta_file.flush().await;
        }
    }
}

pub struct Chunk {
    pub range_download: bool,
    pub start: u64,
    pub end: u64,
    pub index: u16,
    pub version: i64,
    pub valid: bool,
}

impl Chunk {
    pub async fn start_download(
        &mut self,
        url: Arc<String>,
        options: Arc<Mutex<DownloadOptions>>,
        chunk_metadata: Arc<Mutex<ChunkMetadata>>,
        download_handle: Arc<Mutex<DownloadHandle>>,
    ) -> Result<(), Box<dyn Error + Send>> {
        let config = DownloadTaskConfiguration {
            range_download: self.range_download,
            range_start: self.start,
            range_end: self.end,
            url,
        };
        let mut task = DownloadTask::new(config);
        task.start_download(options, download_handle, chunk_metadata, self.index).await
    }

    pub async fn validate(&mut self, chunk_metadata: &ChunkMetadata, chunk_index: usize) {
        if chunk_metadata.version == 0 || chunk_metadata.version != self.version {
            self.valid = false;
            return;
        }

        if chunk_metadata.chunk_positions.len() <= chunk_index {
            self.valid = false;
            return;
        }

        let position = chunk_metadata.chunk_positions.get(chunk_index).unwrap();
        if position == &0 {
            self.valid = false;
            return;
        }

        let chunk_length = position - self.start;
        let remote_length = self.end - self.start + 1;
        if chunk_length > remote_length {
            self.valid = false;
            return;
        }
        self.start = self.start + chunk_length;
        self.valid = self.start == self.end + 1;
    }
}