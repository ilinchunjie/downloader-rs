use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::Instant;
use crate::stream::Stream;

pub struct DownloadHandle {
    file_path: Arc<PathBuf>,
    downloaded_size : i64,
    download_size : i64,
    download_speed : f64,
    last_update_time : Option<Instant>,
    stream: Option<Stream>,
}

impl DownloadHandle {
    pub fn new(file_path: Arc<PathBuf>) -> Self {
        Self {
            file_path,
            stream: None,
            downloaded_size : 0,
            download_size : 0,
            download_speed : 0.,
            last_update_time : None,
        }
    }

    pub async fn setup(&mut self) -> Result<(), std::io::Error> {
        match Stream::new(self.file_path.as_path()).await {
            Ok(stream) => {
                self.stream = Some(stream);
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }

    pub async fn received_bytes_async(&mut self, buffer : &Vec<u8>) {
        self.downloaded_size += buffer.len() as i64;
        self.download_size = buffer.len() as i64;
        if let Some(stream) = &mut self.stream {
            stream.write_async(buffer).await;
        }
        self.update_download_speed();
        self.last_update_time = Some(Instant::now());
    }

    pub async fn flush_async(&mut self) -> Result<(), Error> {
        if let Some(stream) = &mut self.stream {
            return stream.flush_async().await;
        }
        Ok(())
    }

    pub fn get_download_speed(&self) -> f64 {
        self.download_speed
    }

    pub fn get_downloaded_size(&self) -> u64 {
        self.downloaded_size as u64
    }

    fn update_download_speed(&mut self) {
        if self.last_update_time == None {
            self.download_speed = 0.;
        } else {
            let delta_time = Instant::now().duration_since(self.last_update_time.unwrap());
            self.download_speed = self.download_size as f64 / delta_time.as_secs() as f64;
        }
    }
}