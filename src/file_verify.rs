use std::path::Path;
#[cfg(feature = "xxhash-rust")]
use xxhash_rust::xxh64;
#[cfg(feature = "md5")]
use md5::Context;

#[allow(unused)]
use tokio::io::{AsyncReadExt, BufReader};
#[allow(unused)]
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
async fn calculate_file_xxhash(file_path: impl AsRef<Path>, seed: u64) -> crate::error::Result<u64> {
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

#[cfg(feature = "md5")]
async fn calculate_file_md5(file_path: impl AsRef<Path>) -> crate::error::Result<String> {
    match tokio::fs::File::open(file_path.as_ref()).await {
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

#[allow(unused_variables)]
pub async fn file_validate(file_verify: &FileVerify, file_path: impl AsRef<Path>) -> crate::error::Result<()> {
    if *file_verify == FileVerify::None {
        return Ok(());
    }

    match file_verify {
        FileVerify::None => {
            return Ok(());
        }
        #[cfg(feature = "xxhash-rust")]
        FileVerify::xxHash(value) => {
            {
                let hash = calculate_file_xxhash(file_path, 0).await?;
                if !hash.eq(&value) {
                    return Err(DownloadError::FileVerify);
                }
            }
        }
        #[cfg(feature = "md5")]
        FileVerify::MD5(value) => {
            {
                let hash = calculate_file_md5(file_path).await?;
                if !hash.eq(value) {
                    return Err(DownloadError::FileVerify);
                }
            }
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}