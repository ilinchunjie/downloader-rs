use std::error::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};
use crate::stream::Stream;

pub struct DownloadHandleFile {
    config: Arc<Mutex<DownloadConfiguration>>,
    stream: Option<Stream>,
    downloaded_size: u64,
}

impl DownloadHandleFile {
    pub fn new(config: Arc<Mutex<DownloadConfiguration>>) -> Self {
        Self {
            config,
            stream: None,
            downloaded_size: 0,
        }
    }

    pub async fn flush_async(&mut self) -> Result<(), std::io::Error> {
        if let Some(stream) = &mut self.stream {
            return stream.flush_async().await;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl DownloadHandleTrait for DownloadHandleFile {
    async fn setup(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let path = PathBuf::from(self.config.lock().await.path.as_ref().unwrap().deref());
        match Stream::new(path).await {
            Ok(stream) => {
                self.stream = Some(stream);
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
        Ok(())
    }

    async fn received_bytes_async(&mut self, position: u64, buffer: &Vec<u8>) -> Result<(), Box<dyn Error + Send>> {
        self.downloaded_size += buffer.len() as u64;
        if let Some(stream) = &mut self.stream {
            if let Err(e) = stream.seek_async(position).await {
                return Err(Box::new(e));
            }
            if let Err(e) = stream.write_async(buffer).await {
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