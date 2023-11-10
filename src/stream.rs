use std::path::Path;
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt};
use crate::error::DownloadError;

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: &str, append: bool) -> crate::error::Result<Stream> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            if parent.symlink_metadata().is_err() {
                let _ = fs::create_dir_all(parent).await;
            }
        }
        match OpenOptions::new().
            create(true).
            write(true).
            append(append).
            open(path).await {
            Ok(file) => {
                Ok(Stream {
                    file,
                })
            }
            Err(_e) => {
                Err(DownloadError::OpenOrCreateFile)
            }
        }
    }

    pub async fn write_async(&mut self, buffer: &[u8]) -> crate::error::Result<()> {
        if let Err(_e) = self.file.write_all(buffer).await {
            return Err(DownloadError::FileWrite);
        }

        Ok(())
    }

    pub async fn flush_async(&mut self) -> crate::error::Result<()> {
        if let Err(_e) = self.file.flush().await {
            return Err(DownloadError::FileFlush);
        }

        Ok(())
    }
}