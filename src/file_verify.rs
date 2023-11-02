#[cfg(feature = "xxhash-rust")]
use xxhash_rust::xxh64;
#[cfg(feature = "md5")]
use md5::Context;

use tokio::io::{AsyncReadExt, BufReader};
use tokio::fs::File;
use crate::error::DownloadError;

#[derive(PartialEq)]
pub enum FileVerify {
    None,
    #[allow(non_camel_case_types)]
    #[cfg(feature = "xxhash-rust")]
    xxHash(u64),
    #[cfg(feature = "md5")]
    MD5(String),
}

#[cfg(feature = "xxhash-rust")]
pub async fn calculate_file_xxhash(file_path: &str, seed: u64) -> crate::error::Result<u64> {
    match File::open(file_path).await {
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

#[cfg(feature = "md5")]
pub async fn calculate_file_md5(file_path: &str) -> crate::error::Result<String> {
    match File::open(file_path).await {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut context = Context::new();
            let mut buffer = [0u8; 4096];
            loop {
                if let Ok(bytes_read) = reader.read(&mut buffer).await {
                    if bytes_read == 0 {
                        break;
                    }
                    context.write(&buffer[0..bytes_read]);
                } else {
                    break;
                }
            }
            Ok(format!("{:x}", context.compute()))
        }
        Err(_) => {
            return Err(DownloadError::FileOpen);
        }
    }
}