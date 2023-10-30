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

#[derive(PartialEq, Clone)]
pub enum DownloaderStatus {
    None = 0,
    Head = 1,
    Download = 2,
    Archive = 3,
    Complete = 4,
    Failed = 5,
}

impl Display for DownloaderStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloaderStatus::None => write!(f, "None"),
            DownloaderStatus::Head => write!(f, "Head"),
            DownloaderStatus::Download => write!(f, "Download"),
            DownloaderStatus::Archive => write!(f, "Archive"),
            DownloaderStatus::Complete => write!(f, "Complete"),
            DownloaderStatus::Failed => write!(f, "Failed"),
        }
    }
}

pub struct DownloadOptions {
    pub cancel: bool,
}

pub struct Downloader {
    config: Arc<Mutex<DownloadConfiguration>>,
    download_status: Arc<Mutex<DownloaderStatus>>,
    download_handle: Arc<Mutex<DownloadHandle>>,
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

    pub fn get_download_status(&self) -> i32 {
        match *self.download_status.blocking_lock() {
            DownloaderStatus::None => 0,
            DownloaderStatus::Head => 1,
            DownloaderStatus::Download => 2,
            DownloaderStatus::Archive => 3,
            DownloaderStatus::Complete => 4,
            DownloaderStatus::Failed => 5,
        }
    }

    pub fn get_downloaded_size(&self) -> u64 {
        match self.download_handle.blocking_lock().deref_mut() {
            DownloadHandle::File(download_handle) => {
                return download_handle.get_downloaded_size();
            }
            DownloadHandle::Memory(download_handle) => {
                return download_handle.get_downloaded_size();
            }
        }
    }

    pub fn get_total_size(&self) -> u64 {
        self.config.blocking_lock().total_length
    }

    pub fn is_done(&self) -> bool {
        return *self.download_status.blocking_lock() == DownloaderStatus::Complete || *self.download_status.blocking_lock() == DownloaderStatus::Failed;
    }

    pub fn is_error(&self) -> bool {
        return *self.download_status.blocking_lock() == DownloaderStatus::Failed;
    }

    pub fn stop(&mut self) {
        self.options.blocking_lock().cancel = true;
        *self.download_status.blocking_lock() = DownloaderStatus::None;
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
        println!("{}", *status.lock().await);
    }

    let mut remote_file = RemoteFile::new(config.lock().await.url.as_ref().unwrap().clone());
    let mut remote_file_info: Option<RemoteFileInfo> = None;
    match remote_file.head().await {
        Ok(value) => {
            remote_file_info = Some(value);
        }
        Err(e) => {
            println!("{}", e.to_string());
            *status.lock().await = DownloaderStatus::Failed;
            return;
        }
    }

    if options.lock().await.cancel {
        return;
    }

    {
        *status.lock().await = DownloaderStatus::Download;
        println!("{}", *status.lock().await);
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

    println!("start chunk");

    let mut chunk_hub = ChunkHub::new(config.clone());
    chunk_hub.validate(download_handle.clone()).await;

    println!("chunk_hub validate");

    let handles = chunk_hub.start_download(options.clone());
    for handle in handles {
        match handle.await {
            Ok(result) => {
                if let Err(e) = result {
                    println!("{}", e.to_string());
                    *status.lock().await = DownloaderStatus::Failed;
                    return;
                }
            }
            Err(e) => {
                println!("错误：{}", e);
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

#[cfg(test)]
mod test {
    use std::sync::{Arc};
    use std::thread;
    use std::thread::sleep;
    use std::time::Duration;
    use tokio::runtime;
    use tokio::sync::Mutex;
    use tokio::time::{Instant};
    use crate::download_configuration::DownloadConfiguration;
    use crate::downloader::{Downloader, DownloaderStatus};

    #[test]
    fn test_downloader() {
        let url = "https://lan.sausage.xd.com/servers.txt".to_string();
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path("servers.txt".to_string())
            .build();
        let mut downloader = Downloader::new(config);
        let mut downloader = Arc::new(Mutex::new(downloader));
        let downloader_clone = downloader.clone();
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("创建失败");

            rt.block_on(async {
                let time = Instant::now();
                downloader_clone.lock().await.start_download();
                loop {}
            })
        });

        while !downloader.blocking_lock().is_done() {}

        if downloader.blocking_lock().is_error() {
            println!("error");
        }
    }
}