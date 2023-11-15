use std::ops::Deref;
use std::sync::{Arc};
use std::time::Duration;
use reqwest::Client;
use parking_lot::RwLock;
use tokio::spawn;
use tokio::sync::watch::{Receiver};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use crate::download_status::DownloadStatus;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::{chunk, chunk_hub, remote_file};
use crate::remote_file::{RemoteFile};

pub struct Downloader {
    config: Arc<DownloadConfiguration>,
    client: Arc<Client>,
    download_status: Arc<RwLock<DownloadStatus>>,
    cancel_token: CancellationToken,
    sender: Arc<DownloadSender>,
    thread_handle: Option<JoinHandle<()>>,
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
            thread_handle: None,
        };
        downloader
    }

    pub fn start_download(&mut self) {
        let handle = spawn(async_start_download(
            self.config.clone(),
            self.client.clone(),
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
    cancel_token: CancellationToken,
    sender: Arc<DownloadSender>,
    status: Arc<RwLock<DownloadStatus>>) {
    if cancel_token.is_cancelled() {
        return;
    }

    *status.write() = DownloadStatus::Head;

    let remote_file: Option<RemoteFile>;
    match remote_file::head(&client, &config).await {
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

    let remote_file = remote_file.unwrap();

    let _ = sender.download_total_size_sender.send(remote_file.total_length);

    let result = chunk_hub::validate(&config, remote_file).await;

    if let Err(e) = result {
        sender.error_sender.send(e).unwrap();
        *status.write() = DownloadStatus::Failed;
        return;
    }

    let (chunks, receivers) = result.unwrap();
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

    sync_downloaded_size(&receivers, &sender);

    let downloaded_size_handle = {
        let sender = sender.clone();
        let receivers = receivers.clone();
        let handle = spawn(async move {
            loop {
                sync_downloaded_size(&receivers, &sender);
                sleep(Duration::from_millis(500)).await;
            }
        });
        handle
    };

    for handle in handles {
        match handle.await {
            Ok(result) => {
                if let Err(e) = result {
                    downloaded_size_handle.abort();
                    sender.error_sender.send(e).unwrap();
                    *status.write() = DownloadStatus::Failed;
                    return;
                }
            }
            Err(_) => {
                downloaded_size_handle.abort();
                *status.write() = DownloadStatus::Failed;
                return;
            }
        }
    }

    downloaded_size_handle.abort();

    if cancel_token.is_cancelled() {
        return;
    }

    sync_downloaded_size(&receivers, &sender);

    *status.write() = DownloadStatus::DownloadPost;
    if let Err(e) = chunk_hub::on_download_post(&config, chunk_length).await {
        sender.error_sender.send(e).unwrap();
        *status.write() = DownloadStatus::Failed;
        return;
    }

    if cancel_token.is_cancelled() {
        return;
    }

    *status.write() = DownloadStatus::FileVerify;
    if let Err(e) = chunk_hub::calculate_file_hash(&config).await {
        sender.error_sender.send(e).unwrap();
        *status.write() = DownloadStatus::Failed;
        return;
    }

    *status.write() = DownloadStatus::Complete;
}