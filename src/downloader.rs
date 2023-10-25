use std::sync::{Arc, Condvar, mpsc};
use std::time::Duration;
use bytes::Buf;
use tokio::sync::Mutex;
use crate::download_status::DownloadStatus;
use crate::remote_file::RemoteFile;

pub struct Downloader {
    remote_url: Arc<String>,
    remote_file: Arc<Mutex<RemoteFile>>,
    download_status: Arc<Mutex<DownloadStatus>>,
    download_status_condvar: Arc<Condvar>,
}

impl Downloader {
    pub fn new(remote_url: String, file_path: String) -> Downloader {
        let remote_url = Arc::new(remote_url);
        let remote_url_clone = remote_url.clone();
        let mut downloader = Downloader {
            remote_url,
            remote_file: Arc::new(Mutex::new(RemoteFile::new(remote_url_clone))),
            download_status: Arc::new(Mutex::new(DownloadStatus::None)),
            download_status_condvar: Arc::new(Condvar::new()),
        };
        downloader
    }

    pub fn start(&self) {
        let download_status_condvar_clone = self.download_status_condvar.clone();
        let remote_file_clone = self.remote_file.clone();
        let download_status_clone = self.download_status.clone();
        let handle = tokio::spawn(start_download(
            remote_file_clone,
            download_status_clone,
            download_status_condvar_clone));
    }

    pub fn update(&self) {
        let mut download_status = self.download_status.lock();
        if let Ok(result) = self.download_status_condvar.wait_timeout(download_status, Duration::from_millis(1)) {
            download_status = result.0;
            match *download_status {
                DownloadStatus::None => {

                }
                _ => {

                }
            }
        }
    }

    pub fn stop() {

    }
}

async fn start_download(
    remote_file: Arc<Mutex<RemoteFile>>,
    download_status: Arc<Mutex<DownloadStatus>>,
    download_status_condvar: Arc<Condvar>) {
    let mut remote_file = remote_file.lock().await;
    remote_file.head().await;
    drop(remote_file);
    download_status_condvar.notify_one();
}