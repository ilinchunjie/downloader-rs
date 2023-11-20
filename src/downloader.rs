use std::ops::Deref;
use std::sync::{Arc};
use std::time::Duration;
use reqwest::Client;
use parking_lot::RwLock;
use tokio::{fs, spawn};
use tokio::sync::watch::{Receiver};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use crate::download_status::{DownloadFile, DownloadStatus};
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::{chunk, chunk_hub, file_verify, remote_file};
use crate::error::DownloadError;
use crate::file_verify::FileVerify;

pub struct Downloader {
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    download_status: Arc<RwLock<DownloadStatus>>,
    cancel_token: CancellationToken,
    sender: Arc<DownloadSender>,
    thread_handle: RwLock<Option<JoinHandle<()>>>,
}

impl Downloader {
    pub fn new(config: DownloadConfiguration, client: Arc<Client>, sender: Arc<DownloadSender>) -> Downloader {
        let config = Arc::new(config);
        let downloader = Downloader {
            config: config.clone(),
            client,
            download_status: Arc::new(RwLock::new(DownloadStatus::None)),
            cancel_token: CancellationToken::new(),
            sender,
            thread_handle: RwLock::new(None),
        };
        downloader
    }

    pub fn start_download(&self) {
        let config = self.config.clone();
        let client = self.client.clone();
        let cancel_token = self.cancel_token.clone();
        let sender = self.sender.clone();
        let download_status = self.download_status.clone();
        let handle = spawn(async move {
            #[cfg(feature = "patch")]
            if config.enable_diff_patch {
                let patch_file_path = format!("{}.patch", config.get_file_path());
                let download_patch_config = DownloadConfiguration::new()
                    .set_url(config.url())
                    .set_file_path(&patch_file_path)
                    .set_download_speed_limit(config.receive_bytes_per_second)
                    .set_remote_version(config.remote_version)
                    .build();
                if let Ok(_) = start_download_file(Arc::new(download_patch_config),
                                                   client.clone(),
                                                   cancel_token.clone(),
                                                   sender.clone(),
                                                   download_status.clone()).await {
                    if cancel_token.is_cancelled() {
                        return;
                    }
                    if let Ok(_) = start_apply_patch(&patch_file_path, &config).await {
                        if config.file_verify != FileVerify::None {
                            *download_status.write() = DownloadStatus::FileVerify;
                            if let Ok(()) = file_verify::file_validate(&config.file_verify, config.get_file_temp_path()).await {
                                if let Ok(_) = fs::rename(config.get_file_temp_path(), config.get_file_path()).await {
                                    *download_status.write() = DownloadStatus::Complete;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            if let Err(e) = start_download_file(config.clone(),
                                                client.clone(),
                                                cancel_token.clone(),
                                                sender.clone(),
                                                download_status.clone()).await {
                sender.error_sender.send(e).unwrap();
                *download_status.write() = DownloadStatus::Failed;
                return;
            }

            if cancel_token.is_cancelled() {
                return;
            }

            if config.file_verify != FileVerify::None {
                *download_status.write() = DownloadStatus::FileVerify;
                if let Err(e) = file_verify::file_validate(&config.file_verify, config.get_file_temp_path()).await {
                    sender.error_sender.send(e).unwrap();
                    *download_status.write() = DownloadStatus::Failed;
                    return;
                }
            }

            if let Err(e) = fs::rename(config.get_file_temp_path(), config.get_file_path()).await {
                sender.error_sender.send(DownloadError::FileRename(format!("file rename failed {}", e))).unwrap();
                *download_status.write() = DownloadStatus::Failed;
                return;
            }

            *download_status.write() = DownloadStatus::Complete;
        });
        *self.thread_handle.write() = Some(handle);
    }

    pub fn is_done(&self) -> bool {
        if let Some(handle) = self.thread_handle.read().as_ref() {
            return handle.is_finished();
        }
        return false;
    }

    pub fn status(&self) -> DownloadStatus {
        *self.download_status.read()
    }

    pub async fn is_pending_async(&self) -> bool {
        return *self.download_status.read() == DownloadStatus::Pending;
    }

    pub fn pending(&mut self) {
        *self.download_status.write() = DownloadStatus::Pending;
    }

    pub async fn pending_async(&self) {
        *self.download_status.write() = DownloadStatus::Pending;
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
        *self.download_status.write() = DownloadStatus::Stop;
    }

    pub async fn stop_async(&self) {
        self.cancel_token.cancel();
        *self.download_status.write() = DownloadStatus::Stop;
    }
}

#[cfg(feature = "patch")]
async fn start_apply_patch(patch_file_path: &str, config: &Arc<DownloadConfiguration>) -> crate::error::Result<()> {
    if let Err(_) = download_patch::patch::patch(config.get_file_path(), patch_file_path, config.get_file_temp_path()).await {
        return Err(DownloadError::Patch);
    }
    Ok(())
}

async fn start_download_file(
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    cancel_token: CancellationToken,
    sender: Arc<DownloadSender>,
    status: Arc<RwLock<DownloadStatus>>) -> crate::error::Result<()> {
    if cancel_token.is_cancelled() {
        return Ok(());
    }

    *status.write() = DownloadStatus::Head;

    let remote_file = remote_file::head(&client, &config).await?;

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    let download_file = match config.download_patch {
        true => DownloadFile::Patch,
        false => DownloadFile::File
    };
    *status.write() = DownloadStatus::Download(download_file);

    let _ = sender.download_total_size_sender.send(remote_file.total_length);

    let (chunks, receivers) = chunk_hub::validate(&config, remote_file).await?;

    let mut handles = Vec::with_capacity(chunks.len());
    let chunk_length = chunks.len();
    for chunk in chunks {
        if chunk.valid {
            continue;
        }
        let handle = spawn(
            chunk::start_download(
                config.clone(),
                client.clone(),
                chunk,
                cancel_token.clone())
        );
        handles.push(handle);
    }

    fn sync_downloaded_size(receivers: &Arc<Vec<Receiver<u64>>>, sender: &DownloadSender) {
        let mut downloaded_size = 0u64;
        for receiver in receivers.deref() {
            downloaded_size += *receiver.borrow();
        }
        let _ = sender.downloaded_size_sender.send(downloaded_size);
    }

    let receivers = Arc::new(receivers);

    let downloaded_size_handle = {
        sync_downloaded_size(&receivers, &sender);
        let sender = sender.clone();
        let receivers = receivers.clone();
        let handle = spawn(async move {
            loop {
                sync_downloaded_size(&receivers, &sender);
                sleep(Duration::from_millis(100)).await;
            }
        });
        handle
    };


    for handle in handles {
        match handle.await {
            Ok(result) => {
                if let Err(e) = result {
                    downloaded_size_handle.abort();
                    return Err(e);
                }
            }
            Err(_) => {
                downloaded_size_handle.abort();
                return Err(DownloadError::ChunkDownloadHandle);
            }
        }
    }

    downloaded_size_handle.abort();

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    sync_downloaded_size(&receivers, &sender);

    *status.write() = DownloadStatus::DownloadPost;
    chunk_hub::on_download_post(&config, chunk_length).await?;

    Ok(())
}