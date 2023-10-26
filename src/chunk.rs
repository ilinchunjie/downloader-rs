use std::fs;
use std::fs::OpenOptions;
use std::io::{Cursor, Read};
use std::sync::Arc;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};

pub struct Chunk {
    pub file_path: String,
    pub valid: bool,
    pub version: u64,
    pub start: u64,
    pub end: u64,
}

impl Chunk {
    pub fn new(file_path: String, start: u64, end: u64) -> Self {
        Self {
            file_path,
            valid: false,
            version: 0,
            start,
            end,
        }
    }

    pub async fn start_download(&mut self, url: Arc<String>) {
        let mut task = DownloadTask::new(self.file_path.clone(), DownloadTaskConfiguration {
            range_download : true,
            range_start: self.start,
            range_end: self.end,
            url
        });
        task.start_download().await;
    }

    pub async fn save_chunk_version(&mut self) {

    }

    pub fn validate(&mut self, remote_version: u64) {
        let chunk_file_result = fs::metadata(&self.file_path);
        if let Err(e) = chunk_file_result {
            self.valid = false;
            return;
        }
        let version_path = format!("{}.version", self.file_path);
        let mut version_file_result = OpenOptions::new().open(version_path);
        let mut version = 0u64;
        if let Ok(version_file) = &mut version_file_result {
            let mut buffer: Vec<u8> = vec![];
            let result = version_file.read_to_end(&mut buffer);
            if let Ok(length) = result {
                let mut rdr = Cursor::new(buffer);
                version = rdr.read_u64::<LittleEndian>().unwrap();
            }
        }
        if version == 0 || version != remote_version {
            self.valid = false;
            return;
        }
        let chunk_file_metadata = chunk_file_result.unwrap();
        let chunk_length = chunk_file_metadata.len();
        let remote_length = self.end - self.start + 1;
        if chunk_length > remote_length {
            self.valid = false;
            return;
        }
        self.start = self.start + chunk_length;
        self.valid = self.start == self.end;
    }
}