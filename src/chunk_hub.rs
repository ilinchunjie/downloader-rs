use std::cell::RefCell;
use std::ops::Deref;
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
    config: Rc<RefCell<DownloadConfiguration>>,
    file_paths: Option<Arc<Vec<Arc<String>>>>,
    chunks: Option<Vec<Arc<Mutex<Chunk>>>>,
    download_handles: Option<Vec<JoinHandle<()>>>,
    archive_handle: Option<JoinHandle<()>>,
}

impl ChunkHub {
    pub fn new(config: Rc<RefCell<DownloadConfiguration>>) -> Self {
        Self {
            config,
            file_paths: None,
            chunks: None,
            download_handles: None,
            archive_handle: None,
        }
    }

    pub fn start_download(&mut self, options: &Arc<Mutex<DownloadOptions>>) {
        if let Some(chunks) = &mut self.chunks {
            self.download_handles = Some(vec![]);
            for chunk in chunks {
                println!("start_download_chunks");
                let handle = spawn(start_download_chunks(chunk.clone(), options.clone()));
                self.download_handles.as_mut().unwrap().push(handle);
            }
        }
    }

    pub fn start_archive(&mut self) {
        if let Some(file_paths) = &self.file_paths {
            let handle = spawn(start_archive_chunks(self.config.borrow().path.clone(), file_paths.clone()));
            self.archive_handle = Some(handle);
        }
    }

    pub fn is_download_done(&self) -> bool {
        let mut complete = true;
        if let Some(handles) = &self.download_handles {
            for handle in handles {
                if !handle.is_finished() {
                    complete = false;
                }
            }
        }
        complete
    }

    pub fn is_archive_done(&self) -> bool {
        if let Some(archive_handle) = &self.archive_handle {
            return archive_handle.is_finished()
        }
        return false
    }

    pub fn set_file_chunks(&mut self) {
        self.chunks = None;
        let mut chunk_count = (self.config.borrow().total_length as f64 / self.config.borrow().chunk_size as f64).ceil() as u64;
        let mut chunks: Vec<Arc<Mutex<Chunk>>> = Vec::with_capacity(chunk_count as usize);
        if !self.config.borrow().support_range_download || !self.config.borrow().chunk_download {
            chunk_count = 1;
        }
        let mut file_paths: Vec<Arc<String>> = Vec::with_capacity(chunk_count as usize);
        match chunk_count {
            1 => {
                let chunk_file_path = Arc::new(format!("{}.chunk", self.config.borrow().path));
                let chunk = Chunk {
                    file_path: chunk_file_path.clone(),
                    range_download: self.config.borrow().support_range_download,
                    start: 0,
                    end: self.config.borrow().total_length - 1,
                    valid: false,
                    version: self.config.borrow().remote_version,
                    url: self.config.borrow().url.clone(),
                };
                let chunk = Arc::new(Mutex::new(chunk));
                chunks.push(chunk);
                file_paths.push(chunk_file_path.clone());
            }
            _ => {
                for i in 0..chunk_count {
                    let start_position = i * self.config.borrow().chunk_size;
                    let mut end_position = start_position + self.config.borrow().chunk_size - 1;
                    if i == chunk_count - 1 {
                        end_position = start_position + self.config.borrow().total_length % self.config.borrow().chunk_size - 1;
                    }
                    let chunk_file_path = Arc::new(format!("{}.chunk{}", self.config.borrow().path, i));
                    let chunk = Chunk {
                        file_path: chunk_file_path.clone(),
                        range_download: true,
                        start: start_position,
                        end: end_position,
                        valid: false,
                        version: self.config.borrow().remote_version,
                        url: self.config.borrow().url.clone(),
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

async fn start_download_chunks(chunk: Arc<Mutex<Chunk>>, options: Arc<Mutex<DownloadOptions>>) {
    let mut chunk = chunk.lock().await;
    chunk.validate().await;
    if !chunk.valid {
        chunk.start_download(options).await;
    }
}

async fn start_archive_chunks(output: Arc<String>, chunks: Arc<Vec<Arc<String>>>) {
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
            let meta_path = format!("{}.meta", chunks[i].deref());
            fs::remove_file(meta_path).await;
        }
    }
}