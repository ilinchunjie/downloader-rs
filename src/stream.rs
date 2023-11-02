use std::io::{SeekFrom};
use std::path::{PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use crate::error::DownloadError;

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: PathBuf, append: bool) -> crate::error::Result<Stream> {
        match OpenOptions::new().
            create(true).
            write(true).
            append(append).
            open(&path).await {
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

    pub async fn set_length(&mut self, length: u64) -> crate::error::Result<()> {
        if let Err(_e) = self.file.set_len(length).await {
            return Err(DownloadError::FileSetLength(format!("set file length failed : {}", _e)));
        }

        Ok(())
    }

    pub async fn seek_async(&mut self, position: u64) -> crate::error::Result<()> {
        if let Err(_e) = self.file.seek(SeekFrom::Start(position)).await {
            return Err(DownloadError::FileSeek);
        }

        Ok(())
    }

    pub async fn write_async(&mut self, buffer: &Vec<u8>) -> crate::error::Result<()> {
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