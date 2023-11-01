use std::any::Any;
use std::fmt::{Display, Formatter, write};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc};
use bytes::Buf;
use tokio::spawn;
use tokio::sync::Mutex;
use crate::chunk_hub::ChunkHub;
use crate::download_configuration::DownloadConfiguration;
use crate::download_handle::{DownloadHandle, DownloadHandleTrait};
use crate::download_handle_file::DownloadHandleFile;
use crate::download_handle_memory::DownloadHandleMemory;
use crate::remote_file::{RemoteFile, RemoteFileInfo};

#[derive(PartialEq, Clone, Copy)]
pub enum DownloaderStatus {
    None,
    Pendding,
    Head,
    Download,
    Archive,
    Complete,
    Failed,
    Stop,
}

impl Display for DownloaderStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloaderStatus::None => write!(f, "None"),
            DownloaderStatus::Pendding => write!(f, "Pendding"),
            DownloaderStatus::Head => write!(f, "Head"),
            DownloaderStatus::Download => write!(f, "Download"),
            DownloaderStatus::Archive => write!(f, "Archive"),
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
            DownloaderStatus::Pendding => 1,
            DownloaderStatus::Head => 2,
            DownloaderStatus::Download => 3,
            DownloaderStatus::Archive => 4,
            DownloaderStatus::Complete => 5,
            DownloaderStatus::Failed => 6,
            DownloaderStatus::Stop => 7
        }
    }
}

impl From<u8> for DownloaderStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => DownloaderStatus::None,
            1 => DownloaderStatus::Pendding,
            2 => DownloaderStatus::Head,
            3 => DownloaderStatus::Download,
            4 => DownloaderStatus::Archive,
            5 => DownloaderStatus::Complete,
            6 => DownloaderStatus::Failed,
            7 => DownloaderStatus::Stop,
            _ => DownloaderStatus::None,
        }
    }
}

pub struct DownloadOptions {
    pub cancel: bool,
}

pub struct Downloader {
    config: Arc<Mutex<DownloadConfiguration>>,
    download_status: Arc<Mutex<DownloaderStatus>>,
    pub download_handle: Arc<Mutex<DownloadHandle>>,
    options: Arc<Mutex<DownloadOptions>>,
}

impl Downloader {
    pub fn new(config: DownloadConfiguration) -> Downloader {
        let download_in_memory = config.download_in_memory;
        let config = Arc::new(Mutex::new(config));
        let mut download_handle: DownloadHandle;
        if download_in_memory {
            download_handle = DownloadHandle::Memory(DownloadHandleMemory::new(config.clone()))
        } else {
            download_handle = DownloadHandle::File(DownloadHandleFile::new(config.clone()))
        }
        let mut downloader = Downloader {
            config,
            download_handle: Arc::new(Mutex::new(download_handle)),
            download_status: Arc::new(Mutex::new(DownloaderStatus::None)),
            options: Arc::new(Mutex::new(DownloadOptions {
                cancel: false,
            })),
        };
        downloader
    }

    pub fn start_download(&mut self) {
        spawn(async_start_download(
            self.config.clone(),
            self.options.clone(),
            self.download_status.clone(),
            self.download_handle.clone()));
    }

    pub fn status(&self) -> u8 {
        let status = *self.download_status.blocking_lock();
        let status: u8 = status.into();
        return status;
    }

    pub fn get_downloaded_size(&self) -> u64 {
        return match self.download_handle.blocking_lock().deref_mut() {
            DownloadHandle::File(download_handle) => {
                download_handle.get_downloaded_size()
            }
            DownloadHandle::Memory(download_handle) => {
                download_handle.get_downloaded_size()
            }
        };
    }

    pub fn text(&self) -> String {
        return match self.download_handle.blocking_lock().deref_mut() {
            DownloadHandle::File(_) => {
                String::new()
            }
            DownloadHandle::Memory(download_handle) => {
                download_handle.text()
            }
        };
    }

    pub fn get_total_size(&self) -> u64 {
        self.config.blocking_lock().total_length
    }

    pub fn is_done(&self) -> bool {
        return *self.download_status.blocking_lock() == DownloaderStatus::Complete
            || *self.download_status.blocking_lock() == DownloaderStatus::Failed
            || *self.download_status.blocking_lock() == DownloaderStatus::Stop;
    }

    pub async fn is_done_async(&self) -> bool {
        let status = *self.download_status.lock().await;
        return status == DownloaderStatus::Complete
            || status == DownloaderStatus::Failed
            || status == DownloaderStatus::Stop;
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

    pub fn start(&mut self) {
        *self.download_status.blocking_lock() = DownloaderStatus::Pendding;
    }

    pub fn stop(&mut self) {
        self.options.blocking_lock().cancel = true;
        *self.download_status.blocking_lock() = DownloaderStatus::Stop;
    }
}

async fn async_start_download(
    config: Arc<Mutex<DownloadConfiguration>>,
    options: Arc<Mutex<DownloadOptions>>,
    status: Arc<Mutex<DownloaderStatus>>,
    download_handle: Arc<Mutex<DownloadHandle>>) {
    if options.lock().await.cancel {
        return;
    }

    {
        *status.lock().await = DownloaderStatus::Head;
    }

    let mut remote_file = RemoteFile::new(config.lock().await.url.as_ref().unwrap().clone());
    let mut remote_file_info: Option<RemoteFileInfo> = None;
    match remote_file.head().await {
        Ok(value) => {
            remote_file_info = Some(value);
        }
        Err(e) => {
            *status.lock().await = DownloaderStatus::Failed;
            return;
        }
    }

    if options.lock().await.cancel {
        return;
    }

    {
        *status.lock().await = DownloaderStatus::Download;
    }

    if let Some(remote_file_info) = remote_file_info {
        let mut config = config.lock().await;
        config.remote_version = remote_file_info.last_modified_time;
        config.support_range_download = remote_file_info.support_range_download;
        config.total_length = remote_file_info.total_length;
    }

    if options.lock().await.cancel {
        return;
    }

    let mut chunk_hub = ChunkHub::new(config.clone());
    chunk_hub.validate(download_handle.clone()).await;

    let handles = chunk_hub.start_download(options.clone());
    for handle in handles {
        match handle.await {
            Ok(result) => {
                if let Err(e) = result {
                    println!("error: {}", e);
                    *status.lock().await = DownloaderStatus::Failed;
                    return;
                }
            }
            Err(e) => {
                *status.lock().await = DownloaderStatus::Failed;
                return;
            }
        }
    }

    if options.lock().await.cancel {
        return;
    }

    {
        *status.lock().await = DownloaderStatus::Complete;
    }
}