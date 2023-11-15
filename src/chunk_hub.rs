use std::sync::{Arc};
use tokio::{fs};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch::{Receiver, channel};
#[allow(unused_imports)]
use crate::{chunk, chunk_metadata, file_verify};
use crate::chunk::{Chunk};
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;
use crate::error::DownloadError;
use crate::remote_file::RemoteFile;

pub async fn on_download_post(config: &Arc<DownloadConfiguration>, chunk_length: usize) -> crate::error::Result<()> {
    match chunk_length {
        1 => {
            let chunk_path = format!("{}.chunk{}", config.get_file_path(), 0);
            if let Err(e) = fs::rename(&chunk_path, config.get_file_path()).await {
                return Err(DownloadError::FileRename(format!("文件重命名失败 {}", e)));
            }
        }
        _ => {
            let mut output = OpenOptions::new().create(true).write(true).open(config.get_file_path()).await;
            if let Ok(file) = &mut output {
                let mut buffer = vec![0; 8192];
                for i in 0..chunk_length {
                    let chunk_path = format!("{}.chunk{}", config.get_file_path(), i);
                    if let Ok(chunk_file) = &mut tokio::fs::File::open(chunk_path).await {
                        loop {
                            if let Ok(len) = chunk_file.read(&mut buffer).await {
                                if len == 0 {
                                    break;
                                }
                                if let Err(_e) = file.write(&buffer[0..len]).await {
                                    return Err(DownloadError::FileWrite);
                                }
                            } else {
                                return Err(DownloadError::FileWrite);
                            }
                        }
                    }
                }

                if let Err(_e) = file.flush().await {
                    return Err(DownloadError::FileFlush);
                }

                for i in 0..chunk_length {
                    let chunk_path = format!("{}.chunk{}", config.get_file_path(), i);
                    if let Err(_e) = fs::remove_file(chunk_path).await {
                        return Err(DownloadError::DeleteFile);
                    }
                }
            }
        }
    }

    {
        chunk_metadata::delete_metadata(config.get_file_path()).await?;
    }

    Ok(())
}

pub async fn validate(config: &Arc<DownloadConfiguration>, remote_file: RemoteFile) -> crate::error::Result<(Vec<Chunk>, Vec<Receiver<u64>>)> {
    let mut chunk_count = 1;
    if config.range_download && remote_file.support_range_download && config.chunk_download {
        chunk_count = (remote_file.total_length as f64 / config.chunk_size as f64).ceil() as usize;
        chunk_count = chunk_count.max(1);
    }

    let version = chunk_metadata::get_local_version(config.get_file_path()).await;
    let remote_version = match config.remote_version {
        0 => remote_file.last_modified_time,
        _ => config.remote_version
    };

    let chunk_ranges = ChunkRange::from_chunk_count(remote_file.total_length, chunk_count as u64, config.chunk_size);

    let mut chunks = Vec::with_capacity(chunk_count);
    let mut receivers: Vec<Receiver<u64>> = Vec::with_capacity(chunk_count);

    for i in 0..chunk_count {
        let file_path = format!("{}.chunk{}", config.get_file_path(), i);
        let mut chunk = Chunk::new(
            file_path,
            chunk_ranges.get(i).unwrap().clone(),
            config.range_download && remote_file.support_range_download,
        );
        let (sender, receiver) = match version != 0 && version == remote_version {
            true => {
                match chunk.validate().await {
                    2 => {
                        chunk.delete_chunk_file().await?;
                        channel(0)
                    }
                    _ => {
                        channel(chunk.get_downloaded_size())
                    }
                }
            }
            false => {
                chunk.delete_chunk_file().await?;
                channel(0)
            }
        };
        chunk.set_downloaded_size_sender(sender);
        receivers.push(receiver);
        chunks.push(chunk);
    }

    save_local_version(config.get_file_path(), remote_version).await?;

    Ok((chunks, receivers))
}

async fn save_local_version(path: impl AsRef<str>, version: i64) -> crate::error::Result<()> {
    chunk_metadata::save_local_version(path, version).await
}