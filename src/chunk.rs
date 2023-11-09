use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::chunk_operation::ChunkOperation;
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};
use crate::downloader::DownloadOptions;
use crate::error::DownloadError;
use crate::stream::Stream;
use crate::chunk_range::ChunkRange;

pub struct Chunk {
    pub file_path: String,
    pub stream: Option<Stream>,
    pub chunk_range: ChunkRange,
    pub range_download: bool,
    pub valid: bool,
    pub chunk_operation: Option<Arc<Mutex<ChunkOperation>>>
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            file_path: String::new(),
            stream: None,
            chunk_range: ChunkRange::default(),
            range_download: false,
            valid: false,
            chunk_operation: None
        }
    }
}

impl Chunk {
    pub fn new(file_path: String, chunk_range: ChunkRange, range_download: bool) -> Self {
        Self {
            file_path,
            chunk_range,
            range_download,
            ..Default::default()
        }
    }

    pub fn from(total_length: u64, chunk_size: u64) -> Vec<Chunk> {

    }

    pub fn get_downloaded_size(& self) -> u64 {
        return  self.chunk_range.length();
    }

    pub fn set_chunk_operation(&mut self, operation: Arc<Mutex<ChunkOperation>>) {
        self.chunk_operation = Some(operation);
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
            if let Some(operation) = &self.chunk_operation {
                operation.lock().await.set_downloaded_size(self.chunk_range.length());
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
        if let Err(e) = tokio::fs::remove_file(&self.file_path).await {
            return Err(DownloadError::DeleteFile);
        }

        Ok(())
    }

    pub async fn validate(&mut self) {
        if self.chunk_range.end == 0 {
            self.valid = false;
            return;
        }

        let metadata = tokio::fs::metadata(&self.file_path).await;
        if let Ok(metadata) = metadata {
            if metadata.len() > self.chunk_range.chunk_length() {
                self.valid = false;
                return;
            }

            self.chunk_range.set_position(self.chunk_range.start + metadata.len());
            self.valid = self.chunk_range.eof();
        }

        self.valid = false;
        return;
    }

    pub fn chunk_range(&self) -> ChunkRange {
        return self.chunk_range;
    }
}

pub async fn start_download(
    url: Arc<String>,
    chunk: Chunk,
    options: Arc<Mutex<DownloadOptions>>,
) -> crate::error::Result<()> {
    let config = DownloadTaskConfiguration {
        range_download: chunk.range_download,
        range_start: chunk.chunk_range.position,
        range_end: chunk.chunk_range.end,
        url,
    };

    let mut task = DownloadTask::new(config);
    task.start_download(options, chunk).await
}