use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufReader};
use crate::chunk_metadata;
use crate::chunk::Chunk;
use crate::chunk_range::ChunkRange;
use crate::download_configuration::DownloadConfiguration;
use crate::error::DownloadError;
use crate::remote_file::RemoteFile;

pub async fn on_download_post(config: &Arc<DownloadConfiguration>, chunk_length: usize) -> crate::error::Result<()> {
    if config.download_in_memory {
        return Ok(());
    }
    if chunk_length > 1 {
        let mut output = OpenOptions::new().create(true).write(true).open(config.get_file_temp_path()).await;
        if let Ok(file) = &mut output {
            for i in 0..chunk_length {
                let chunk_path = chunk_file_path(config.get_file_path(), i);
                if let Ok(chunk_file) = tokio::fs::File::open(&chunk_path).await {
                    // Use 64KB BufReader + tokio::io::copy instead of manual 8KB loop
                    let mut reader = BufReader::with_capacity(64 * 1024, chunk_file);
                    if let Err(_e) = tokio::io::copy(&mut reader, file).await {
                        return Err(DownloadError::FileWrite);
                    }
                }
            }

            if let Err(_e) = file.flush().await {
                return Err(DownloadError::FileFlush);
            }

            for i in 0..chunk_length {
                let chunk_path = chunk_file_path(config.get_file_path(), i);
                if let Err(_e) = fs::remove_file(chunk_path).await {
                    return Err(DownloadError::DeleteFile);
                }
            }
        }
    }

    let _ = chunk_metadata::delete_metadata(config.get_file_path()).await;

    Ok(())
}

/// Validates existing chunks and sets up the shared downloaded_size counter.
/// The counter is the same `Arc<AtomicU64>` from the sender, so the receiver
/// can read progress at any time without polling.
pub async fn validate(
    config: &Arc<DownloadConfiguration>,
    remote_file: RemoteFile,
    downloaded_size_counter: Arc<AtomicU64>,
) -> crate::error::Result<Vec<Chunk>> {
    let mut chunk_count = 1;
    if config.range_download && remote_file.support_range_download && config.chunk_download && !config.download_in_memory {
        chunk_count = (remote_file.total_length as f64 / config.chunk_size as f64).ceil() as usize;
        chunk_count = chunk_count.max(1);
    }

    let version = match config.download_in_memory {
        true => 0,
        false => chunk_metadata::get_local_version(config.get_file_path()).await,
    };
    let remote_version = match config.remote_version {
        0 => remote_file.last_modified_time,
        _ => config.remote_version
    };

    let chunk_ranges = ChunkRange::from_chunk_count(remote_file.total_length, chunk_count as u64, config.chunk_size);

    let mut chunks = Vec::with_capacity(chunk_count);
    let mut initial_downloaded_total = 0u64;

    for i in 0..chunk_count {
        let mut chunk = match config.download_in_memory {
            true => {
                Chunk::from_memory(chunk_ranges.get(i).unwrap().clone())
            }
            false => {
                let file_path = match chunk_count {
                    1 => config.get_file_temp_path().to_path_buf(),
                    _ => chunk_file_path(config.get_file_path(), i),
                };
                Chunk::from_file(
                    file_path,
                    chunk_ranges.get(i).unwrap().clone(),
                    config.range_download && remote_file.support_range_download,
                )
            }
        };

        if !config.download_in_memory {
            match version != 0 && version == remote_version {
                true => {
                    match chunk.validate().await {
                        2 => {
                            chunk.delete_chunk_file().await?;
                        }
                        _ => {
                            initial_downloaded_total += chunk.get_downloaded_size();
                        }
                    }
                }
                false => {
                    chunk.delete_chunk_file().await?;
                }
            }
        }

        chunk.set_downloaded_size_counter(downloaded_size_counter.clone());
        chunks.push(chunk);
    }

    // Set the initial downloaded total (from already-validated chunks)
    downloaded_size_counter.store(initial_downloaded_total, Ordering::Relaxed);

    if !config.download_in_memory {
        chunk_metadata::save_local_version(config.get_file_path(), remote_version).await?;
    }

    Ok(chunks)
}

/// Build the path for a numbered chunk file, e.g. `/tmp/file.bin.chunk0`.
fn chunk_file_path(base: &Path, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.chunk{}", base.display(), index))
}