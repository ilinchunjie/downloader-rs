use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc};
use bytes::Buf;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::chunk_hub::ChunkHub;
use crate::download_configuration::DownloadConfiguration;
use crate::remote_file::RemoteFile;

#[derive(PartialEq, Clone)]
pub enum DownloaderStatus {
    None = 0,
    Head = 1,
    Download = 2,
    Archive = 3,
    Complete = 4,
}

impl Display for DownloaderStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloaderStatus::None => write!(f, "None"),
            DownloaderStatus::Head => write!(f, "Head"),
            DownloaderStatus::Download => write!(f, "Download"),
            DownloaderStatus::Archive => write!(f, "Archive"),
            DownloaderStatus::Complete => write!(f, "Complete"),
        }
    }
}

pub struct DownloadOptions {
    pub cancel: bool,
    pub downloaded_size: u64,
}

pub struct Downloader {
    config: Arc<Mutex<DownloadConfiguration>>,
    remote_file: Arc<Mutex<RemoteFile>>,
    remote_file_request_handle: Option<JoinHandle<()>>,
    chunk_hub: Option<ChunkHub>,
    pub download_status: DownloaderStatus,
    options: Arc<Mutex<DownloadOptions>>,
}

impl Downloader {
    pub fn new(config: DownloadConfiguration) -> Downloader {
        let url = config.url.clone();
        let config = Arc::new(Mutex::new(config));
        let mut downloader = Downloader {
            config,
            remote_file: Arc::new(Mutex::new(RemoteFile::new(url))),
            remote_file_request_handle: None,
            chunk_hub: None,
            download_status: DownloaderStatus::None,
            options: Arc::new(Mutex::new(DownloadOptions {
                downloaded_size: 0,
                cancel: false,
            })),
        };
        downloader
    }

    pub async fn start(&mut self) {
        if self.download_status != DownloaderStatus::None {
            return;
        }
        self.set_downloader_status(DownloaderStatus::Head).await;
    }

    pub async fn update(&mut self) {
        match self.download_status {
            DownloaderStatus::Head => {
                if let Some(handle) = &self.remote_file_request_handle {
                    if handle.is_finished() {
                        self.set_downloader_status(DownloaderStatus::Download).await;
                    }
                }
            }
            DownloaderStatus::Download => {
                if let Some(chunk_hub) = &mut self.chunk_hub {
                    if chunk_hub.is_download_done() {
                        self.set_downloader_status(DownloaderStatus::Archive).await;
                    }
                }
            }
            DownloaderStatus::Archive => {
                if let Some(chunk_hub) = &mut self.chunk_hub {
                    if chunk_hub.is_archive_done() {
                        self.set_downloader_status(DownloaderStatus::Complete).await;
                    }
                }
            }
            _ => {}
        }
    }

    pub async fn stop(&mut self) {
        self.options.lock().await.cancel = true;
        self.set_downloader_status(DownloaderStatus::Complete).await;
    }

    fn start_head(&mut self) {
        let handle = tokio::spawn(async_remote_file(self.remote_file.clone()));
        self.remote_file_request_handle = Some(handle);
    }

    async fn start_download(&mut self) {
        let remote_file = self.remote_file.lock().await;
        if let Some(remote_file_info) = &remote_file.remote_file_info {
            let mut config = self.config.lock().await;
            config.remote_version = remote_file_info.last_modified_time;
            config.support_range_download = remote_file_info.support_range_download;
            config.total_length = remote_file_info.total_length;
            drop(config);
        }

        let mut chunk_hub = ChunkHub::new(self.config.clone());
        chunk_hub.set_file_chunks().await;
        chunk_hub.start_download(&self.options.clone());

        self.chunk_hub = Some(chunk_hub);
    }

    async fn start_archive(&mut self) {
        if let Some(chunk_hub) = &mut self.chunk_hub {
            chunk_hub.start_archive().await;
        }
    }

    async fn set_downloader_status(&mut self, status: DownloaderStatus) {
        match self.download_status {
            _ => {}
        }
        self.download_status = status;
        match self.download_status {
            DownloaderStatus::Head => {
                self.start_head();
            }
            DownloaderStatus::Download => {
                self.start_download().await;
            }
            DownloaderStatus::Archive => {
                self.start_archive().await;
            }
            _ => {}
        }
    }
}

async fn async_remote_file(remote_file: Arc<Mutex<RemoteFile>>) {
    let mut remote_file = remote_file.lock().await;
    remote_file.head().await;
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::time::Duration;
    use tokio::runtime;
    use tokio::time::{Instant, sleep};
    use crate::download_configuration::DownloadConfiguration;
    use crate::downloader::{Downloader, DownloaderStatus};

    #[tokio::test]
    async fn test_downloader() {
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("创建失败");

            rt.block_on(async {
                let time = Instant::now();

                let url = "https://n17x06.xdcdn.net/media/SS6_CG_Weather_Kingdom.mp4".to_string();
                let mut downloader = Downloader::new(DownloadConfiguration::from_url_path(url, "SS6_CG_Weather_Kingdom.mp4".to_string()));
                downloader.start().await;
                while downloader.download_status != DownloaderStatus::Complete {
                    downloader.update().await;
                    if downloader.download_status == DownloaderStatus::Download {
                        //sleep(Duration::from_secs(4)).await;
                        //downloader.stop().await;
                    }
                }

                println!("took {}s", Instant::now().duration_since(time).as_secs());
            })
        });

        handle.join().expect("");
    }
}