use std::io::{Error};
use std::ops::Deref;
use std::sync::{Arc};
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};
use crate::downloader::DownloadOptions;

pub struct Chunk {
    pub file_path: Arc<String>,
    pub url: Arc<String>,
    pub range_download: bool,
    pub start: u64,
    pub end: u64,
    pub valid: bool,
    pub version: i64,
}

impl Chunk {
    pub async fn start_download(&mut self, options: Arc<Mutex<DownloadOptions>>) {
        if let Err(e) = self.save_chunk_version().await {
            println!("{}", e);
        }

        let config = DownloadTaskConfiguration {
            file_path: self.file_path.clone(),
            range_download : self.range_download,
            range_start : self.start,
            range_end : self.end,
            url : self.url.clone(),
        };
        let mut task = DownloadTask::new(config);
        task.start_download(options).await;
    }

    pub async fn save_chunk_version(&mut self) -> Result<(), Error> {
        let meta_path = format!("{}.meta", self.file_path);
        let mut version_file_result = OpenOptions::new().create(true).write(true).open(meta_path).await;
        if let Ok(version_file) = &mut version_file_result {
            let bytes = self.version.to_le_bytes();
            if let Err(e) = version_file.write_all(&bytes).await {
                return Err(e);
            }
            return version_file.flush().await;
        }

        Ok(())
    }

    pub async fn get_local_version(&self) -> i64 {
        let meta_path = format!("{}.meta", self.file_path);
        let mut meta_file_result = OpenOptions::new().read(true).open(meta_path).await;
        let mut version = 0i64;
        match &mut meta_file_result {
            Ok(meta_file) => {
                let mut buffer: Vec<u8> = vec![];
                let result = meta_file.read_to_end(&mut buffer).await;
                if let Ok(length) = result {
                    let mut i64_bytes = [0u8; 8];
                    i64_bytes.copy_from_slice(&buffer);
                    version = i64::from_le_bytes(i64_bytes);
                }
            }
            Err(e) => {
                println!("{}", e);
            }
        }
        version
    }

    pub async fn validate(&mut self, options: Arc<Mutex<DownloadOptions>>) {
        let chunk_file_result = fs::metadata(self.file_path.deref()).await;
        if let Err(e) = chunk_file_result {
            self.valid = false;
            return;
        }

        let local_version = self.get_local_version().await;
        if local_version == 0 || local_version != self.version {
            self.valid = false;
            return;
        }

        let chunk_file_metadata = chunk_file_result.unwrap();
        let chunk_length = chunk_file_metadata.len();
        let remote_length = self.end - self.start + 1;
        if chunk_length > remote_length {
            self.valid = false;
            return;
        }
        self.start = self.start + chunk_length;
        self.valid = self.start == self.end + 1;

        options.lock().await.downloaded_size += chunk_length;
    }
}