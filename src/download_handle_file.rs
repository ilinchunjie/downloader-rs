use std::ops::Deref;
use std::path::{PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandleTrait};
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
        let config = self.config.lock().await;
        let path: PathBuf;
        if config.create_temp_file {
            path = PathBuf::from(format!("{}.temp", config.path.as_ref().unwrap().deref()));
        } else {
            path = PathBuf::from(config.path.as_ref().unwrap().deref());
        }
        let append = !config.chunk_download && config.support_range_download || config.total_length == 0;
        let create_dir = config.create_dir;
        let mut stream = Stream::new(path, append).await?;
        if config.set_file_length && config.total_length > 0 {
            let total_length = config.total_length;
            let _ = &stream.set_length(total_length).await?;
        }
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