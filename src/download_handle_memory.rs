use std::error::Error;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandleTrait};

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
        if let Some(bytes) = self.bytes() {
            if let Ok(text) = String::from_utf8(bytes) {
                return text;
            }
        }
        String::new()
    }

    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        if let Some(cursor) = &mut self.cursor {
            let len = cursor.get_ref().len();
            let mut buffer: Vec<u8> = Vec::with_capacity(len);
            unsafe {
                buffer.set_len(len);
            }
            cursor.set_position(0);
            match cursor.read_exact(&mut buffer) {
                Ok(_) => {
                    return Some(buffer);
                }
                Err(e) => {
                    println!("{}", e);
                }
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
        if let Some(cursor) = &mut self.cursor {
            if let Err(e) = cursor.seek(SeekFrom::Start(poition)) {
                return Err(Box::new(e));
            }
            if let Err(e) = cursor.write_all(buffer) {
                return Err(Box::new(e));
            }
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