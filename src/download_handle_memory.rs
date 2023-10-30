use std::error::Error;
use std::io::{Cursor, Read, SeekFrom, Write};
use std::sync::Arc;
use tokio::io::{AsyncSeekExt};
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};

pub struct DownloadHandleMemory {
    config: Arc<Mutex<DownloadConfiguration>>,
    cursor: Option<Cursor<Vec<u8>>>,
    downloaded_size: u64,
}

impl DownloadHandleMemory {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            cursor: None,
            downloaded_size: 0,
        }
    }

    pub fn text(&mut self) -> String {
        if let Some(cursor) = &mut self.cursor {
            let mut buffer: Vec<u8> = Vec::with_capacity(cursor.get_ref().len());
            if let Ok(_) = cursor.read_exact(&mut buffer) {
                return String::from_utf8(buffer).unwrap()
            }
        }
        String::new()
    }

    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        if let Some(cursor) = &mut self.cursor {
            let mut buffer: Vec<u8> = Vec::with_capacity(cursor.get_ref().len());
            if let Ok(_) = cursor.read_exact(&mut buffer) {
                return Some(buffer);
            }
        }
        None
    }
}

#[async_trait::async_trait]
impl DownloadHandleTrait for DownloadHandleMemory {
    async fn setup(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let size = self.config.lock().await.total_length;
        let buffer: Vec<u8> = Vec::with_capacity(size as usize);
        self.cursor = Some(Cursor::new(buffer));
        Ok(())
    }

    async fn received_bytes_async(&mut self, poition: u64, buffer: &Vec<u8>) -> Result<(), Box<dyn Error + Send>> {
        self.downloaded_size += buffer.len() as u64;
        if let Err(e) = self.cursor.as_mut().unwrap().seek(SeekFrom::Start(poition)).await {
            return Err(Box::new(e));
        }
        if let Err(e) = self.cursor.as_mut().unwrap().write_all(buffer) {
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