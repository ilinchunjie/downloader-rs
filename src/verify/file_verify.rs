use std::path::Path;
use xxhash_rust::xxh64;
use tokio::io::{AsyncReadExt, BufReader};
use crate::error::DownloadError;

#[derive(PartialEq)]
pub enum FileVerify {
    None,
    #[allow(non_camel_case_types)]
    xxHash(u64),
}

pub async fn calculate_file_xxhash(file_path: impl AsRef<Path>, seed: u64) -> crate::error::Result<u64> {
    match tokio::fs::File::open(file_path.as_ref()).await {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut hasher = xxh64::Xxh64::new(seed);
            let mut buffer = [0u8; 4096];
            loop {
                if let Ok(bytes_read) = reader.read(&mut buffer).await {
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[0..bytes_read]);
                } else {
                    break;
                }
            }
            Ok(hasher.digest())
        }
        Err(_) => {
            return Err(DownloadError::FileOpen);
        }
    }
}

pub async fn file_validate(file_verify: &FileVerify, file_path: impl AsRef<Path>) -> crate::error::Result<()> {
    match file_verify {
        FileVerify::None => Ok(()),
        FileVerify::xxHash(value) => {
            let hash = calculate_file_xxhash(file_path, 0).await?;
            if !hash.eq(&value) {
                return Err(DownloadError::FileVerify);
            }
            Ok(())
        }
    }
}