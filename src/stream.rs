use std::io::Error;
use std::path::Path;
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: &Path) -> Result<Stream, Error> {
        if let Some(directory) = path.parent() {
            if !directory.exists() {
                let result = fs::create_dir(directory).await;
                if let Err(e) = result {
                    return Err(e);
                }
            }
        }
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

    pub async fn write_async(&mut self, buffer: &Vec<u8>) {
        self.file.write_all(buffer).await.expect("TODO: panic message");
    }

    pub async fn flush_async(&mut self) -> Result<(), Error> {
        self.file.flush().await
    }
}