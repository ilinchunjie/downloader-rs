use std::error::Error;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandleTrait};

pub struct DownloadHandleMemory {
    config: Arc<Mutex<DownloadConfiguration>>,
    bytes: Option<Vec<u8>>,
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

    pub fn text(&mut self) -> String {
        if let Some(bytes) = &mut self.bytes {
            if let Ok(text) = String::from_utf8(bytes.clone()) {
                return text;
            }
        }
        String::new()
    }

    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        if let Some(bytes) = &mut self.bytes {
            return Some(bytes.clone());
        }
        None
    }
}

#[async_trait::async_trait]
impl DownloadHandleTrait for DownloadHandleMemory {
    async fn setup(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let size = self.config.lock().await.total_length;
        let buffer: Vec<u8> = Vec::with_capacity(size as usize);
        self.bytes = Some(buffer);
        Ok(())
    }

    async fn received_bytes_async(&mut self, poition: u64, buffer: &Vec<u8>) -> Result<(), Box<dyn Error + Send>> {
        self.downloaded_size += buffer.len() as u64;
        if let Some(bytes) = &mut self.bytes {
            bytes.extend_from_slice(&buffer);
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