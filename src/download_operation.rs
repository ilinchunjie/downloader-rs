use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::download_status::DownloadStatus;
use crate::download_receiver::DownloadReceiver;
use crate::downloader::{Downloader};
use crate::error::DownloadError;

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

    pub fn status(&self) -> DownloadStatus {
        let status = *self.download_receiver.status_receiver.borrow();
        return DownloadStatus::from(status);
    }

    pub fn downloaded_size(&self) -> u64 {
        return *self.download_receiver.downloaded_size_receiver.borrow();
    }

    pub fn total_size(&self) -> u64 {
        return *self.download_receiver.download_total_size_receiver.borrow();
    }

    pub fn progress(&self) -> f64 {
        let total_size = self.total_size();
        if total_size == 0 {
            return 0f64;
        }
        let total_length = total_size as f64;
        let downloaded_size = self.downloaded_size() as f64;
        return (downloaded_size / total_length).clamp(0f64, 1f64);
    }

    pub fn is_done(&self) -> bool {
        return self.downloader.blocking_lock().is_done();
    }

    pub fn is_error(&self) -> bool {
        return self.status() == DownloadStatus::Failed;
    }

    pub fn error(&self) -> DownloadError {
        return (*self.download_receiver.error_receiver.borrow()).clone();
    }

    pub fn stop(&self) {
        self.downloader.blocking_lock().stop();
    }
}