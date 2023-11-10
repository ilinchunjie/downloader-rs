use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::{Arc};
use reqwest::Client;
use tokio::fs;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::chunk_hub::ChunkHub;
use crate::download_configuration::DownloadConfiguration;
use crate::download_sender::DownloadSender;
use crate::remote_file;
use crate::remote_file::{RemoteFile};

#[derive(PartialEq, Clone, Copy)]
pub enum DownloaderStatus {
    None,
    Pending,
    Head,
    Download,
    DownloadPost,
    FileVerify,
    Complete,
    Failed,
    Stop,
}

impl Display for DownloaderStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloaderStatus::None => write!(f, "None"),
            DownloaderStatus::Pending => write!(f, "Pending"),
            DownloaderStatus::Head => write!(f, "Head"),
            DownloaderStatus::Download => write!(f, "Download"),
            DownloaderStatus::FileVerify => write!(f, "FileVerify"),
            DownloaderStatus::DownloadPost => write!(f, "DownloadPost"),
            DownloaderStatus::Complete => write!(f, "Complete"),
            DownloaderStatus::Failed => write!(f, "Failed"),
            DownloaderStatus::Stop => write!(f, "Stop"),
        }
    }
}

impl Into<u8> for DownloaderStatus {
    fn into(self) -> u8 {
        match self {
            DownloaderStatus::None => 0,
            DownloaderStatus::Pending => 1,
            DownloaderStatus::Head => 2,
            DownloaderStatus::Download => 3,
            DownloaderStatus::DownloadPost => 4,
            DownloaderStatus::FileVerify => 5,
            DownloaderStatus::Complete => 6,
            DownloaderStatus::Failed => 7,
            DownloaderStatus::Stop => 8
        }
    }
}

impl From<u8> for DownloaderStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => DownloaderStatus::None,
            1 => DownloaderStatus::Pending,
            2 => DownloaderStatus::Head,
            3 => DownloaderStatus::Download,
            4 => DownloaderStatus::DownloadPost,
            5 => DownloaderStatus::FileVerify,
            6 => DownloaderStatus::Complete,
            7 => DownloaderStatus::Failed,
            8 => DownloaderStatus::Stop,
            _ => DownloaderStatus::None,
        }
    }
}

pub struct DownloadOptions {
    pub client: Arc<Mutex<Client>>,
    pub cancel: bool,
}

pub struct Downloader {
    config: Arc<Mutex<DownloadConfiguration>>,
    download_status: Arc<Mutex<DownloaderStatus>>,
    chunk_hub: Arc<Mutex<ChunkHub>>,
    options: Arc<Mutex<DownloadOptions>>,
    sender: Arc<Mutex<DownloadSender>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl Downloader {
    pub fn new(config: DownloadConfiguration, client: Arc<Mutex<Client>>, sender: Arc<Mutex<DownloadSender>>) -> Downloader {
        let config = Arc::new(Mutex::new(config));
        let downloader = Downloader {
            config: config.clone(),
            chunk_hub: Arc::new(Mutex::new(ChunkHub::new(config.clone()))),
            download_status: Arc::new(Mutex::new(DownloaderStatus::None)),
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

    pub fn get_total_size(&self) -> u64 {
        self.config.blocking_lock().total_length
    }

    pub fn is_done(&self) -> bool {
        if let Some(handle) = &self.thread_handle {
            return handle.is_finished();
        }
        return false;
    }

    pub fn is_stop(&self) -> bool {
        return *self.download_status.blocking_lock() == DownloaderStatus::Stop;
    }

    pub async fn is_stop_async(&self) -> bool {
        return *self.download_status.lock().await == DownloaderStatus::Stop;
    }

    pub fn is_error(&self) -> bool {
        return *self.download_status.blocking_lock() == DownloaderStatus::Failed;
    }

    pub fn is_pending(&self) -> bool {
        return *self.download_status.blocking_lock() == DownloaderStatus::Pending;
    }

    pub async fn is_pending_async(&self) -> bool {
        return *self.download_status.lock().await == DownloaderStatus::Pending;
    }

    pub fn pending(&mut self) {
        *self.download_status.blocking_lock() = DownloaderStatus::Pending;
    }

    pub fn stop(&mut self) {
        self.options.blocking_lock().cancel = true;
        *self.download_status.blocking_lock() = DownloaderStatus::Stop;
    }
}

async fn change_download_status(status: &Arc<Mutex<DownloaderStatus>>, sender: &Arc<Mutex<DownloadSender>>, to_status: DownloaderStatus) {
    *status.lock().await = to_status;
    sender.lock().await.status_sender.send(to_status.into()).unwrap();
}

async fn async_start_download(
    config: Arc<Mutex<DownloadConfiguration>>,
    chunk_hub: Arc<Mutex<ChunkHub>>,
    options: Arc<Mutex<DownloadOptions>>,
    sender: Arc<Mutex<DownloadSender>>,
    status: Arc<Mutex<DownloaderStatus>>) {
    if options.lock().await.cancel {
        return;
    }

    {
        change_download_status(&status, &sender, DownloaderStatus::Head).await;
    }

    let remote_file: Option<RemoteFile>;
    match remote_file::head(&options.lock().await.client, config.lock().await.url()).await {
        Ok(value) => {
            remote_file = Some(value);
        }
        Err(_) => {
            change_download_status(&status, &sender, DownloaderStatus::Failed).await;
            return;
        }
    }

    if options.lock().await.cancel {
        return;
    }

    {
        change_download_status(&status, &sender, DownloaderStatus::Download).await;
    }

    if let Some(remote_file) = &remote_file {
        let mut config = config.lock().await;
        config.remote_version = remote_file.last_modified_time;
        config.support_range_download = remote_file.support_range_download;
        config.total_length = remote_file.total_length;
        sender.lock().await.download_total_size_sender.send(remote_file.total_length).unwrap();
    }

    drop(remote_file);

    if options.lock().await.cancel {
        return;
    }

    {
        let mut config = config.lock().await;
        if config.create_dir {
            let path = Path::new(config.get_file_path());
            if let Some(directory) = path.parent() {
                if !directory.exists() {
                    let _ = fs::create_dir_all(directory).await;
                }
            }
        }
    }

    {
        let receivers = chunk_hub.lock().await.validate().await;
        let handles = chunk_hub.lock().await.start_download(options.clone());
        let cancel = Arc::new(Mutex::new(false));

        {
            let chunk_hub = chunk_hub.clone();
            let sender = sender.clone();
            let cancel = cancel.clone();
            spawn(async move {
                loop {
                    let mut downloaded_size_changed = false;
                    for receiver in &receivers {
                        if let Ok(changed) = receiver.has_changed() {
                            downloaded_size_changed = true;
                            break;
                        }
                    }
                    if downloaded_size_changed {
                        let downloaded_size = chunk_hub.lock().await.get_downloaded_size().await;
                        sender.lock().await.downloaded_size_sender.send(downloaded_size).unwrap();
                    }

                    if *cancel.lock().await {
                        break
                    }
                }
            });
        }

        for handle in handles {
            match handle.await {
                Ok(result) => {
                    if let Err(e) = result {
                        *cancel.lock().await = true;
                        change_download_status(&status, &sender, DownloaderStatus::Failed).await;
                        return;
                    }
                }
                Err(_) => {
                    *cancel.lock().await = true;
                    change_download_status(&status, &sender, DownloaderStatus::Failed).await;
                    return;
                }
            }
        }

        *cancel.lock().await = true;
    }

    {
        change_download_status(&status, &sender, DownloaderStatus::DownloadPost).await;
        if let Err(e) = chunk_hub.lock().await.on_download_post().await {
            change_download_status(&status, &sender, DownloaderStatus::Failed).await;
            return;
        }
    }

    {
        change_download_status(&status, &sender, DownloaderStatus::FileVerify).await;
        if let Err(_) = chunk_hub.lock().await.calculate_file_hash().await {
            change_download_status(&status, &sender, DownloaderStatus::Failed).await;
            return;
        }
    }

    if options.lock().await.cancel {
        return;
    }

    {
        change_download_status(&status, &sender, DownloaderStatus::Complete).await;
    }
}