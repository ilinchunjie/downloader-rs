use std::ops::Deref;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::sync::watch::{Receiver, Sender};
use crate::download_receiver::DownloadReceiver;
use crate::downloader::{Downloader, DownloaderStatus};

pub struct DownloadOperation {
    downloader: Arc<Mutex<Downloader>>,
    download_receiver: DownloadReceiver,
}

impl DownloadOperation {
    pub fn new(
        downloader: Arc<Mutex<Downloader>>,
        download_receiver: DownloadReceiver) -> DownloadOperation {
        DownloadOperation {
            downloader,
            download_receiver,
        }
    }

    pub fn status(&self) -> u8 {
        return *self.download_receiver.status_receiver.borrow();
    }

    pub fn downloaded_size(&self) -> u64 {
        return *self.download_receiver.downloaded_size_receiver.borrow();
    }

    pub fn total_size(&self) -> u64 {
        return *self.download_receiver.download_total_size_receiver.borrow();
    }

    pub fn progress(&self) -> f64 {
        if self.total_size() == 0 {
            return 0f64;
        }
        let total_length = self.total_size() as f64;
        let downloaded_size = self.downloaded_size() as f64;
        return (downloaded_size / total_length).clamp(0f64, 1f64);
    }

    pub fn is_done(&self) -> bool {
        return *self.download_receiver.is_done_receiver.borrow();
    }

    pub fn stop(&self) {
        self.downloader.blocking_lock().stop();
    }
}