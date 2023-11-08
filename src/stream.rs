use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt};
use crate::error::DownloadError;

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: impl AsRef<str>, append: bool) -> crate::error::Result<Stream> {
        match OpenOptions::new().
            create(true).
            write(true).
            append(append).
            open(path.as_ref()).await {
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