use std::sync::{Arc};
use reqwest::Client;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::download_status::DownloadStatus;
use crate::chunk_hub::ChunkHub;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::remote_file;
use crate::remote_file::{RemoteFile};

pub struct DownloadOptions {
    pub client: Arc<Mutex<Client>>,
    pub cancel: bool,
}

pub struct Downloader {
    config: Arc<DownloadConfiguration>,
    download_status: Arc<Mutex<DownloadStatus>>,
    chunk_hub: Arc<Mutex<ChunkHub>>,
    options: Arc<Mutex<DownloadOptions>>,
    sender: Arc<Mutex<DownloadSender>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl Downloader {
    pub fn new(config: DownloadConfiguration, client: Arc<Mutex<Client>>, sender: Arc<Mutex<DownloadSender>>) -> Downloader {
        let config = Arc::new(config);
        let downloader = Downloader {
            config: config.clone(),
            chunk_hub: Arc::new(Mutex::new(ChunkHub::new(config.clone()))),
            download_status: Arc::new(Mutex::new(DownloadStatus::None)),
            options: Arc::new(Mutex::new(DownloadOptions {
                cancel: false,
                client,
            })),
            sender,
            thread_handle: None,
        };
        downloader
    }

    pub fn start_download(&mut self) {
        let handle = spawn(async_start_download(
            self.config.clone(),
            self.chunk_hub.clone(),
            self.options.clone(),
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

    pub async fn is_pending_async(&self) -> bool {
        return *self.download_status.lock().await == DownloadStatus::Pending;
    }

    pub fn pending(&mut self) {
        *self.download_status.blocking_lock() = DownloadStatus::Pending;
        self.sender.blocking_lock().status_sender.send((DownloadStatus::Pending).into()).unwrap();
    }

    pub fn stop(&mut self) {
        self.options.blocking_lock().cancel = true;
        *self.download_status.blocking_lock() = DownloadStatus::Stop;
        self.sender.blocking_lock().status_sender.send((DownloadStatus::Stop).into()).unwrap();
    }
}

async fn change_download_status(status: &Arc<Mutex<DownloadStatus>>, sender: &Arc<Mutex<DownloadSender>>, to_status: DownloadStatus) {
    *status.lock().await = to_status;
    sender.lock().await.status_sender.send(to_status.into()).unwrap();
}

async fn async_start_download(
    config: Arc<DownloadConfiguration>,
    chunk_hub: Arc<Mutex<ChunkHub>>,
    options: Arc<Mutex<DownloadOptions>>,
    sender: Arc<Mutex<DownloadSender>>,
    status: Arc<Mutex<DownloadStatus>>) {
    if options.lock().await.cancel {
        return;
    }

    change_download_status(&status, &sender, DownloadStatus::Head).await;

    let remote_file: Option<RemoteFile>;
    match remote_file::head(&options.lock().await.client, config).await {
        Ok(value) => {
            remote_file = Some(value);
        }
        Err(e) => {
            change_download_status(&status, &sender, DownloadStatus::Failed).await;
            sender.lock().await.error_sender.send(e).unwrap();
            return;
        }
    }

    if options.lock().await.cancel {
        return;
    }

    change_download_status(&status, &sender, DownloadStatus::Download).await;

    {
        let remote_file = remote_file.unwrap();

        let _ = sender.lock().await.download_total_size_sender.send(remote_file.total_length);

        let receivers = chunk_hub.lock().await.validate(remote_file).await;

        if let Err(e) = receivers {
            sender.lock().await.error_sender.send(e).unwrap();
            change_download_status(&status, &sender, DownloadStatus::Failed).await;
            return;
        }
        let receivers = receivers.unwrap();
        let handles = chunk_hub.lock().await.start_download(options.clone());
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
                        let sender = sender.lock().await;
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
                        sender.lock().await.error_sender.send(e).unwrap();
                        change_download_status(&status, &sender, DownloadStatus::Failed).await;
                        return;
                    }
                }
                Err(_) => {
                    *cancel.lock().await = true;
                    change_download_status(&status, &sender, DownloadStatus::Failed).await;
                    return;
                }
            }
        }

        *cancel.lock().await = true;

        if options.lock().await.cancel {
            return;
        }

        let downloaded_size = chunk_hub.lock().await.get_downloaded_size().await;
        let _ = sender.lock().await.downloaded_size_sender.send(downloaded_size);
    }

    {
        change_download_status(&status, &sender, DownloadStatus::DownloadPost).await;
        if let Err(e) = chunk_hub.lock().await.on_download_post().await {
            sender.lock().await.error_sender.send(e).unwrap();
            change_download_status(&status, &sender, DownloadStatus::Failed).await;
            return;
        }
    }

    {
        change_download_status(&status, &sender, DownloadStatus::FileVerify).await;
        if let Err(e) = chunk_hub.lock().await.calculate_file_hash().await {
            sender.lock().await.error_sender.send(e).unwrap();
            change_download_status(&status, &sender, DownloadStatus::Failed).await;
            return;
        }
    }

    if options.lock().await.cancel {
        return;
    }

    {
        change_download_status(&status, &sender, DownloadStatus::Complete).await;
    }
}