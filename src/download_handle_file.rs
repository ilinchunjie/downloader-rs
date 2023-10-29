use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::DownloadHandle;
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

    pub async fn setup(&mut self) -> Result<(), std::io::Error> {
        let path = PathBuf::from(self.config.lock().await.path.as_ref().unwrap().deref());
        match Stream::new(path).await {
            Ok(stream) => {
                self.stream = Some(stream);
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }

    pub async fn received_bytes_async(&mut self, buffer: &Vec<u8>) -> Result<(), std::io::Error> {
        self.downloaded_size += buffer.len() as u64;
        if let Some(stream) = &mut self.stream {
            return stream.write_async(buffer).await
        }
        Ok(())
    }

    pub fn get_downloaded_size(&mut self) -> u64 {
        return self.downloaded_size;
    }
}