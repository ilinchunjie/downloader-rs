use std::cell::RefCell;
use std::error::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{fs, spawn};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::chunk::Chunk;
use crate::download_configuration::DownloadConfiguration;
use crate::downloader::DownloadOptions;

pub struct ChunkHub {
    config: Arc<Mutex<DownloadConfiguration>>,
    file_paths: Option<Arc<Vec<Arc<PathBuf>>>>,
    chunks: Option<Vec<Arc<Mutex<Chunk>>>>,
}

impl ChunkHub {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            file_paths: None,
            chunks: None,
        }
    }

    pub fn start_download(&mut self, options: Arc<Mutex<DownloadOptions>>) -> Vec<JoinHandle<Result<(), Box<dyn Error + Send>>>> {
        let mut handles: Vec<JoinHandle<Result<(), Box<dyn Error + Send>>>> = vec![];
        if let Some(chunks) = &mut self.chunks {
            for chunk in chunks {
                let handle = spawn(start_download_chunks(chunk.clone(), options.clone()));
                handles.push(handle);
            }
        }
        return handles;
    }

    pub async fn start_archive(&mut self) -> Option<JoinHandle<()>> {
        if let Some(file_paths) = &self.file_paths {
            return Some(spawn(start_archive_chunks(self.config.lock().await.path.as_ref().unwrap().clone(), file_paths.clone())));
        }
        None
    }

    pub async fn set_file_chunks(&mut self) {
        self.chunks = None;
        let config = self.config.lock().await;
        let mut chunk_count = 1;
        if config.support_range_download && config.chunk_download {
            chunk_count = (config.total_length as f64 / config.chunk_size as f64).ceil() as u64;
        }

        let mut chunks: Vec<Arc<Mutex<Chunk>>> = Vec::with_capacity(chunk_count as usize);
        let mut file_paths: Vec<Arc<PathBuf>> = Vec::with_capacity(chunk_count as usize);
        match chunk_count {
            1 => {
                let chunk_file_path = Arc::new(PathBuf::from(format!("{}.chunk", config.path.as_ref().unwrap())));
                let chunk = Chunk {
                    file_path: chunk_file_path.clone(),
                    range_download: config.support_range_download,
                    start: 0,
                    end: config.total_length - 1,
                    valid: false,
                    version: config.remote_version,
                    url: config.url.as_ref().unwrap().clone(),
                };
                let chunk = Arc::new(Mutex::new(chunk));
                chunks.push(chunk);
                file_paths.push(chunk_file_path.clone());
            }
            _ => {
                for i in 0..chunk_count {
                    let start_position = i * config.chunk_size;
                    let mut end_position = start_position + config.chunk_size - 1;
                    if i == chunk_count - 1 {
                        end_position = start_position + config.total_length % config.chunk_size - 1;
                    }
                    let chunk_file_path = Arc::new(PathBuf::from(format!("{}.chunk{}", config.path.as_ref().unwrap(), i)));
                    let chunk = Chunk {
                        file_path: chunk_file_path.clone(),
                        range_download: true,
                        start: start_position,
                        end: end_position,
                        valid: false,
                        version: config.remote_version,
                        url: config.url.as_ref().unwrap().clone(),
                    };
                    let chunk = Arc::new(Mutex::new(chunk));
                    chunks.push(chunk);
                    file_paths.push(chunk_file_path.clone());
                }
            }
        }
        self.chunks = Some(chunks);
        self.file_paths = Some(Arc::new(file_paths));
    }
}

async fn start_download_chunks(chunk: Arc<Mutex<Chunk>>, options: Arc<Mutex<DownloadOptions>>) -> Result<(), Box<dyn Error + Send>> {
    let mut chunk = chunk.lock().await;
    chunk.validate(options.clone()).await;
    if !chunk.valid {
        return chunk.start_download(options.clone()).await
    }
    Ok(())
}

async fn start_archive_chunks(output: Arc<String>, chunks: Arc<Vec<Arc<PathBuf>>>) {
    let mut output_result = OpenOptions::new().create(true).write(true).open(output.deref()).await;
    if let Ok(output_file) = &mut output_result {
        for i in 0..chunks.len() {
            let mut chunk_file = File::open(chunks[i].deref()).await;
            if let Ok(chunk_file) = &mut chunk_file {
                let mut buffer = Vec::new();
                chunk_file.read_to_end(&mut buffer).await;
                output_file.write_all(&buffer).await;
            }
        }
        output_file.flush().await;

        for i in 0..chunks.len() {
            fs::remove_file(chunks[i].deref()).await;
            let meta_path = format!("{}.meta", chunks[i].deref().to_str().unwrap());
            fs::remove_file(meta_path).await;
        }
    }
}