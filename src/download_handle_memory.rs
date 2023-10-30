use std::error::Error;
use std::io::{Cursor, SeekFrom, Write};
use std::sync::Arc;
use tokio::io::AsyncSeekExt;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};

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
}

#[async_trait::async_trait]
impl DownloadHandleTrait for DownloadHandleMemory {
    async fn setup(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let size = self.config.lock().await.total_length;
        let buffer: Vec<u8> = Vec::with_capacity(size as usize);
        self.bytes = Some(Cursor::new(buffer));
        Ok(())
    }

    async fn received_bytes_async(&mut self, poition: u64, buffer: &Vec<u8>) -> Result<(), Box<dyn Error + Send>> {
        self.downloaded_size += buffer.len() as u64;
        if let Err(e) = self.bytes.as_mut().unwrap().seek(SeekFrom::Start(poition)).await {
            return Err(Box::new(e));
        }
        if let Err(e) = self.bytes.as_mut().unwrap().write_all(buffer) {
            return Err(Box::new(e));
        }
        Ok(())
    }

    fn get_downloaded_size(&self) -> u64 {
        return self.downloaded_size;
    }

    fn update_downloaded_size(&mut self, length: u64) {
        self.downloaded_size += length;
    }
}