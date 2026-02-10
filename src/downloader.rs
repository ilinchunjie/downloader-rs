use std::sync::Arc;
use reqwest::Client;
use parking_lot::RwLock;
use tokio::{fs, spawn};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use crate::download_status::DownloadStatus;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::{chunk, chunk_hub, remote_file};
use crate::error::DownloadError;
use crate::verify::file_verify::FileVerify;
use crate::verify::file_verify;
use crate::rate_limiter::RateLimiter;
use tracing;

pub struct Downloader {
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    download_status: Arc<RwLock<DownloadStatus>>,
    cancel_token: RwLock<CancellationToken>,
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
            cancel_token: RwLock::new(CancellationToken::new()),
            sender,
            thread_handle: RwLock::new(None),
        };
        downloader
    }

    pub fn start_download(&self) {
        let config = self.config.clone();
        let client = self.client.clone();
        let new_token = CancellationToken::new();
        *self.cancel_token.write() = new_token.clone();
        let cancel_token = new_token;
        let sender = self.sender.clone();
        let download_status = self.download_status.clone();
        let handle = spawn(async move {
            if let Err(e) = start_download_file(config.clone(),
                                                client.clone(),
                                                cancel_token.clone(),
                                                sender.clone(),
                                                download_status.clone()).await {
                tracing::error!(error = %e, "download failed");
                let _ = sender.error_sender.send(e);
                *download_status.write() = DownloadStatus::Failed;
                return;
            }

            if cancel_token.is_cancelled() {
                return;
            }

            if !config.download_in_memory {
                if config.file_verify != FileVerify::None {
                    *download_status.write() = DownloadStatus::FileVerify;
                    tracing::info!("verifying downloaded file");
                    if let Err(e) = file_verify::file_validate(&config.file_verify, config.get_file_temp_path()).await {
                        tracing::error!(error = %e, "file verification failed");
                        let _ = sender.error_sender.send(e);
                        *download_status.write() = DownloadStatus::Failed;
                        return;
                    }
                }

                if let Err(e) = fs::rename(config.get_file_temp_path(), config.get_file_path()).await {
                    let _ = sender.error_sender.send(DownloadError::FileRename(format!("file rename failed {}", e)));
                    *download_status.write() = DownloadStatus::Failed;
                    return;
                }
            }

            *download_status.write() = DownloadStatus::Complete;
            tracing::info!("download complete");
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
        self.cancel_token.read().cancel();
        *self.download_status.write() = DownloadStatus::Stop;
    }

    pub async fn stop_async(&self) {
        self.cancel_token.read().cancel();
        *self.download_status.write() = DownloadStatus::Stop;
    }
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
    tracing::info!(url = config.url(), "sending HEAD request");

    let remote_file = remote_file::head(&client, &config).await?;

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    *status.write() = DownloadStatus::Download;

    let _ = sender.download_total_size_sender.send(remote_file.total_length);
    tracing::info!(total_size = remote_file.total_length, "starting download");

    // Pass the shared AtomicU64 counter to chunk_hub::validate.
    // Each chunk will atomically increment this counter as data arrives.
    // The receiver side reads the same counter for instant progress.
    let chunks = chunk_hub::validate(&config, remote_file, sender.downloaded_size.clone()).await?;

    // Create global rate limiter from config
    let rate_limiter = RateLimiter::new(config.receive_bytes_per_second);

    let mut handles = Vec::with_capacity(chunks.len());
    let chunk_length = chunks.len();
    for chunk in chunks {
        if chunk.valid {
            continue;
        }
        let sender = sender.clone();
        let rl = rate_limiter.clone();
        let handle = spawn(
            chunk::start_download(
                config.clone(),
                client.clone(),
                chunk,
                sender,
                cancel_token.clone(),
                rl)
        );
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(result) => {
                if let Err(e) = result {
                    return Err(e);
                }
            }
            Err(_) => {
                return Err(DownloadError::ChunkDownloadHandle);
            }
        }
    }

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    *status.write() = DownloadStatus::DownloadPost;
    chunk_hub::on_download_post(&config, chunk_length).await?;

    Ok(())
}