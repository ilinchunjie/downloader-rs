use std::error::Error;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};
use crate::error::DownloadError;
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

    pub async fn flush_async(&mut self) -> crate::error::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.flush_async().await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl DownloadHandleTrait for DownloadHandleFile {
    async fn setup(&mut self) -> crate::error::Result<()> {
        let path = PathBuf::from(self.config.lock().await.path.as_ref().unwrap().deref());
        let stream = Stream::new(path).await?;
        self.stream = Some(stream);
        Ok(())
    }

    async fn received_bytes_async(&mut self, position: u64, buffer: &Vec<u8>) -> crate::error::Result<()> {
        self.downloaded_size += buffer.len() as u64;
        if let Some(stream) = &mut self.stream {
            stream.seek_async(position).await?;
            stream.write_async(buffer).await?;
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