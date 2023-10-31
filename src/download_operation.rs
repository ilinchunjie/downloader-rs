use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::downloader::Downloader;

pub struct DownloadOperation {
    downloader: Arc<Mutex<Downloader>>,
}

impl DownloadOperation {
    pub fn new(downloader: Arc<Mutex<Downloader>>) -> DownloadOperation {
        DownloadOperation {
            downloader
        }
    }

    pub fn text(&self) -> String {
        return self.downloader.blocking_lock().text();
    }

    pub fn status(&self) -> i32 {
        return self.downloader.blocking_lock().get_download_status();
    }

    pub fn downloaded_size(&self) -> u64 {
        return self.downloader.blocking_lock().get_downloaded_size();
    }

    pub fn is_done(&self) -> bool {
        return self.downloader.blocking_lock().is_done();
    }

    pub fn stop(&self) {
        self.downloader.blocking_lock().stop();
    }
}