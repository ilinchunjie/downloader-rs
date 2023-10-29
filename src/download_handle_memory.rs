use std::io::{Cursor, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::DownloadHandle;

pub struct DownloadHandleMemory {
    config: Arc<Mutex<DownloadConfiguration>>,
    bytes: Option<Cursor<Vec<u8>>>,
    downloaded_size: u64,
}

impl DownloadHandleMemory {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            bytes: None,
            downloaded_size: 0,
        }
    }

    pub async fn setup(&mut self) -> Result<(), std::io::Error> {
        let size = self.config.lock().await.total_length;
        let buffer: Vec<u8> = Vec::with_capacity(size as usize);
        self.bytes = Some(Cursor::new(buffer));
        Ok(())
    }

    pub async fn received_bytes_async(&mut self, buffer: &Vec<u8>) -> Result<(), std::io::Error> {
        self.downloaded_size += buffer.len() as u64;
        self.bytes.as_mut().unwrap().write_all(buffer)
    }

    pub fn get_downloaded_size(&mut self) -> u64 {
        return self.downloaded_size;
    }
}