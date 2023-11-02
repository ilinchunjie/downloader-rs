use std::io::SeekFrom;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use crate::error::DownloadError;

pub struct ChunkMetadata {
    pub path: Arc<PathBuf>,
    pub file: Option<File>,
    pub version: i64,
    pub chunk_count: u16,
    pub chunk_positions: Vec<u64>,
}

impl ChunkMetadata {
    fn get_metadata_length(&self) -> u64 {
        let length = (mem::size_of::<i64>() + mem::size_of::<u16>() + self.chunk_count as usize * mem::size_of::<u64>()) as u64;
        length
    }

    pub async fn create_chunk_metadata(&mut self) -> crate::error::Result<()> {
        match OpenOptions::new().write(true).create(true).open(&self.path.as_path()).await {
            Ok(mut meta_file) => {
                let _ =meta_file.write(&self.version.to_le_bytes()).await;
                let _ =meta_file.write(&self.chunk_count.to_le_bytes()).await;
                for _ in 0..self.chunk_count {
                    let _ =meta_file.write(&(0 as u64).to_le_bytes()).await;
                }
                let _ =meta_file.set_len(self.get_metadata_length());
                let _ =meta_file.flush().await;
                self.file = Some(meta_file);
            }
            Err(e) => {
                return Err(DownloadError::CreateMetaFile(format!("{} open failed : {}", self.path.to_str().unwrap(), e)));
            }
        }

        Ok(())
    }

    pub async fn get_chunk_metadata(path: Arc<PathBuf>, chunk_count: u16) -> ChunkMetadata {
        let chunk_positions = Vec::with_capacity(chunk_count as usize);
        let file_path = path.as_path();
        let mut chunk_metadata = ChunkMetadata {
            path: path.clone(),
            version: 0,
            file: None,
            chunk_count,
            chunk_positions,
        };

        if let Ok(_) = fs::metadata(file_path).await {
            if let Ok(mut meta_file) = OpenOptions::new().
                read(true).
                write(true).
                open(file_path).await {
                if let Ok(version) = meta_file.read_i64_le().await {
                    chunk_metadata.version = version;
                }
                if let Ok(count) = meta_file.read_u16_le().await {
                    if count != chunk_metadata.chunk_count {
                        return chunk_metadata;
                    }
                }

                for _ in 0..chunk_metadata.chunk_count {
                    let mut chunk_position = 0u64;
                    if let Ok(position) = meta_file.read_u64_le().await {
                        chunk_position = position;
                    }
                    chunk_metadata.chunk_positions.push(chunk_position);
                }
                let _ =meta_file.set_len(chunk_metadata.get_metadata_length()).await;
                let _ =meta_file.flush().await;
                chunk_metadata.file = Some(meta_file);
            }
        }
        chunk_metadata
    }

    pub async fn update_chunk_version(&mut self, version: i64) -> crate::error::Result<()> {
        if let None = self.file {
            self.create_chunk_metadata().await?;
        }
        if let Some(meta_file) = &mut self.file {
            let _ =meta_file.seek(SeekFrom::Start(0)).await;
            let _ =meta_file.write(&version.to_le_bytes()).await;
            let _ =meta_file.flush().await;
        }

        Ok(())
    }

    pub async fn update_chunk_position(&mut self, position: u64, chunk_index: u16) -> crate::error::Result<()> {
        if let None = self.file {
            self.create_chunk_metadata().await?;
        }
        if let Some(meta_file) = &mut self.file {
            let version_length = mem::size_of::<i64>();
            let count_length = mem::size_of::<u16>();
            let seek_position = version_length + count_length + chunk_index as usize * mem::size_of::<u64>();
            let _ =meta_file.seek(SeekFrom::Start(seek_position as u64)).await;
            let _ =meta_file.write(&position.to_le_bytes()).await;
            let _ = meta_file.flush().await;
        }

        Ok(())
    }
}