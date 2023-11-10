use std::path::Path;
use tokio::fs;
use tokio::fs::{OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::error::DownloadError;

pub async fn get_local_version(path: impl AsRef<str>) -> i64 {
    let meta_file_path = format!("{}.metadata", path.as_ref());
    if let Ok(exist) = tokio::fs::try_exists(&meta_file_path).await {
        if exist {
            if let Ok(meta_file) = &mut OpenOptions::new().read(true).open(&meta_file_path).await {
                if let Ok(version) = meta_file.read_i64_le().await {
                    return version;
                }
            }
        }
    }
    0
}

pub async fn save_local_version(path: impl AsRef<str>, version: i64) -> crate::error::Result<()> {
    let meta_file_path = format!("{}.metadata", path.as_ref());
    let path = Path::new(&meta_file_path);
    if let Some(parent) = path.parent() {
        if parent.symlink_metadata().is_err() {
            let _ = fs::create_dir_all(parent).await;
        }
    }
    if let Ok(meta_file) = &mut OpenOptions::new().write(true).create(true).open(&meta_file_path).await {
        if let Err(_) = meta_file.write_i64_le(version).await {
            return Err(DownloadError::FileWrite);
        }
    }
    Ok(())
}

pub async fn delete_metadata(path: impl AsRef<str>) -> crate::error::Result<()> {
    let meta_file_path = format!("{}.metadata", path.as_ref());
    if let Err(_) = tokio::fs::remove_file(meta_file_path).await {
        return Err(DownloadError::DeleteFile);
    };
    Ok(())
}