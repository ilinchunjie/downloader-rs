use std::fmt::format;
use std::ops::Deref;
use std::sync::{Arc};
use tokio::spawn;
use tokio::sync::Mutex;
use crate::chunk::Chunk;
use crate::download_configuration::DownloadConfiguration;

pub struct ChunkHub {
    config: Arc<DownloadConfiguration>,
    chunks: Option<Vec<Arc<Mutex<Chunk>>>>,
}

impl ChunkHub {
    pub fn new(config: Arc<DownloadConfiguration>) -> Self {
        Self {
            config,
            chunks: None,
        }
    }

    pub fn start_download(&mut self) {
        if let Some(chunks) = &mut self.chunks {
            for chunk in chunks {
                if let Some(url) = &self.config.url {
                    spawn(start_download_chunks(chunk.clone(), url.clone(), self.config.remote_version));
                }
            }
        }
    }

    pub fn set_file_chunks(&mut self) {
        self.chunks = None;
        let chunk_count = self.config.total_length / self.config.chunk_size;
        let mut chunks: Vec<Arc<Mutex<Chunk>>> = Vec::with_capacity(chunk_count as usize);
        for i in 0..chunk_count {
            let start_position = i * self.config.chunk_size;
            let mut end_position = start_position + self.config.chunk_size - 1;
            if i == chunk_count - 1 {
                end_position = end_position + self.config.total_length % chunk_count;
            }
            let chunk_file_path = format!("{}.chunk{}", self.config.path.unwrap(), i);
            let chunk = Chunk::new(chunk_file_path, start_position, end_position);
            let chunk = Arc::new(Mutex::new(chunk));
            chunks[i as usize] = chunk;
        }
        self.chunks = Some(chunks);
    }
}

async fn start_download_chunks(
    chunk: Arc<Mutex<Chunk>>,
    url: Arc<String>,
    version: u64) {
    let mut chunk = chunk.lock().await;
    chunk.validate(version);
    chunk.start_download(url.clone()).await;
}