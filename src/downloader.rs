use std::future::Future;
use std::sync::{Arc};
use reqwest::Client;
use parking_lot::RwLock;
use tokio::spawn;
use tokio::sync::{Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use crate::download_status::DownloadStatus;
use crate::chunk_hub::ChunkHub;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::remote_file;
use crate::remote_file::{RemoteFile};

pub struct Downloader {
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    download_status: Arc<RwLock<DownloadStatus>>,
    chunk_hub: Arc<Mutex<ChunkHub>>,
    cancel_token: CancellationToken,
    sender: Arc<DownloadSender>,
    thread_handle: Option<JoinHandle<()>>,
}

impl Downloader {
    pub fn new(config: DownloadConfiguration, client: Arc<Client>, sender: Arc<DownloadSender>) -> Downloader {
        let config = Arc::new(config);
        let downloader = Downloader {
            config: config.clone(),
            client: client,
            chunk_hub: Arc::new(Mutex::new(ChunkHub::new(config.clone()))),
            download_status: Arc::new(RwLock::new(DownloadStatus::None)),
            cancel_token: CancellationToken::new(),
            sender,
            thread_handle: None,
        };
        downloader
    }

    pub fn start_download(&mut self) {
        let handle = spawn(async_start_download(
            self.config.clone(),
            self.client.clone(),
            self.chunk_hub.clone(),
            self.cancel_token.clone(),
            self.sender.clone(),
            self.download_status.clone()));
        self.thread_handle = Some(handle);
    }

    pub fn is_done(&self) -> bool {
        if let Some(handle) = &self.thread_handle {
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

    pub async fn pending_async(&mut self) {
        *self.download_status.write() = DownloadStatus::Pending;
    }

    pub fn stop(&mut self) {
        self.cancel_token.cancel();
        *self.download_status.write() = DownloadStatus::Stop;
    }

    pub async fn stop_async(&mut self) {
        self.cancel_token.cancel();
        *self.download_status.write() = DownloadStatus::Stop;
    }
}

async fn async_start_download(
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    chunk_hub: Arc<Mutex<ChunkHub>>,
    cancel_token: CancellationToken,
    sender: Arc<DownloadSender>,
    status: Arc<RwLock<DownloadStatus>>) {
    if cancel_token.is_cancelled() {
        return;
    }

    *status.write() = DownloadStatus::Head;

    let remote_file: Option<RemoteFile>;
    match remote_file::head(&client, config).await {
        Ok(value) => {
            remote_file = Some(value);
        }
        Err(e) => {
            sender.error_sender.send(e).unwrap();
            *status.write() = DownloadStatus::Failed;
            return;
        }
    }

    if cancel_token.is_cancelled() {
        return;
    }

    *status.write() = DownloadStatus::Download;

    {
        let remote_file = remote_file.unwrap();

        let _ = sender.download_total_size_sender.send(remote_file.total_length);

        let receivers = chunk_hub.lock().await.validate(remote_file).await;

        if let Err(e) = receivers {
            sender.error_sender.send(e).unwrap();
            *status.write() = DownloadStatus::Failed;
            return;
        }
        let receivers = receivers.unwrap();
        let handles = chunk_hub.lock().await.start_download(client.clone(), cancel_token.clone());
        let cancel = Arc::new(Mutex::new(false));

        {
            let chunk_hub = chunk_hub.clone();
            let sender = sender.clone();
            let cancel = cancel.clone();
            spawn(async move {
                'r: loop {
                    let mut downloaded_size_changed = false;
                    for receiver in &receivers {
                        if let Ok(changed) = receiver.has_changed() {
                            if changed {
                                downloaded_size_changed = true;
                                break;
                            }
                        }
                    }
                    if downloaded_size_changed {
                        let downloaded_size = chunk_hub.lock().await.get_downloaded_size().await;
                        if *cancel.lock().await {
                            break 'r;
                        }
                        let _ = sender.downloaded_size_sender.send(downloaded_size);
                    }

                    if *cancel.lock().await {
                        break 'r;
                    }
                }
            });
        }

        for handle in handles {
            match handle.await {
                Ok(result) => {
                    if let Err(e) = result {
                        *cancel.lock().await = true;
                        sender.error_sender.send(e).unwrap();
                        *status.write() = DownloadStatus::Failed;
                        return;
                    }
                }
                Err(_) => {
                    *cancel.lock().await = true;
                    *status.write() = DownloadStatus::Failed;
                    return;
                }
            }
        }

        *cancel.lock().await = true;

        if cancel_token.is_cancelled() {
            return;
        }

        let downloaded_size = chunk_hub.lock().await.get_downloaded_size().await;
        let _ = sender.downloaded_size_sender.send(downloaded_size);
    }

    *status.write() = DownloadStatus::DownloadPost;
    if let Err(e) = chunk_hub.lock().await.on_download_post().await {
        sender.error_sender.send(e).unwrap();
        *status.write() = DownloadStatus::Failed;
        return;
    }

    if cancel_token.is_cancelled() {
        return;
    }

    *status.write() = DownloadStatus::FileVerify;
    if let Err(e) = chunk_hub.lock().await.calculate_file_hash().await {
        sender.error_sender.send(e).unwrap();
        *status.write() = DownloadStatus::Failed;
        return;
    }

    *status.write() = DownloadStatus::Complete;
}