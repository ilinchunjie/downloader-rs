use std::io::{Error, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWriteExt};
use crate::error::DownloadError;

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: PathBuf) -> crate::error::Result<Stream> {
        match OpenOptions::new().
            create(true).
            write(true).
            append(true).
            open(&path).await {
            Ok(file) => {
                Ok(Stream {
                    file,
                })
            }
            Err(e) => {
                Err(DownloadError::OpenOrCreateFile)
            }
        }
    }

    pub async fn seek_async(&mut self, position: u64) -> crate::error::Result<()> {
        if let Err(e) = self.file.seek(SeekFrom::Start(position)).await {
            return Err(DownloadError::Seek);
        }

        Ok(())
    }

    pub async fn write_async(&mut self, buffer: &Vec<u8>) -> crate::error::Result<()> {
        if let Err(e) = self.file.write_all(buffer).await {
            return Err(DownloadError::Write);
        }

        Ok(())
    }

    pub async fn flush_async(&mut self) -> crate::error::Result<()> {
        if let Err(e) = self.file.flush().await {
            return Err(DownloadError::FlushToDisk);
        }

        Ok(())
    }
}