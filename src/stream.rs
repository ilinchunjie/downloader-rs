use std::io::Error;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

pub struct Stream {
    file: File,
}

impl Stream {
    pub async fn new(path: &String) -> Stream {
        let file = OpenOptions::new().create(true).write(true).append(true).open(&path).await.expect("文件创建失败");
        Stream {
            file,
        }
    }

    pub async fn write_async(&mut self, buffer: &Vec<u8>) {
        self.file.write_all(buffer).await.expect("TODO: panic message");
    }

    pub async fn flush_async(&mut self) -> Result<(), Error> {
        self.file.flush().await
    }
}