use std::io::{Error, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWriteExt};

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: PathBuf) -> Result<Stream, Error> {
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
                Err(e)
            }
        }
    }

    pub async fn seek_async(&mut self, position: u64) -> Result<u64, Error> {
        self.file.seek(SeekFrom::Start(position)).await
    }

    pub async fn write_async(&mut self, buffer: &Vec<u8>) -> Result<(), Error> {
        self.file.write_all(buffer).await
    }

    pub async fn flush_async(&mut self) -> Result<(), Error> {
        self.file.flush().await
    }
}