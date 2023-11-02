use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::downloader::Downloader;

#[derive(Clone)]
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

    pub fn status(&self) -> u8 {
        return self.downloader.blocking_lock().status();
    }

    pub fn downloaded_size(&self) -> u64 {
        return self.downloader.blocking_lock().get_downloaded_size();
    }

    pub fn total_size(&self) -> u64 {
        return self.downloader.blocking_lock().get_total_size();
    }

    pub fn progress(&self) -> f64 {
        if self.total_size() == 0 {
            return 0f64;
        }
        let total_length = self.total_size() as f64;
        let downloaded_size = self.downloaded_size() as f64;
        return (downloaded_size / total_length).clamp(0f64, 1f64);
    }

    pub fn chunk_count(&self) -> usize {
        return self.downloader.blocking_lock().get_chunk_count();
    }

    pub fn chunk_progress(&self, index: usize) -> f64 {
        return self.downloader.blocking_lock().get_chunk_progress(index);
    }

    pub fn is_done(&self) -> bool {
        return self.downloader.blocking_lock().is_done();
    }

    pub fn stop(&self) {
        self.downloader.blocking_lock().stop();
    }
}