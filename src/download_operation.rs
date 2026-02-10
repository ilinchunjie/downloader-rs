use std::sync::Arc;
use crate::download_status::DownloadStatus;
use crate::download_receiver::DownloadReceiver;
use crate::downloader::Downloader;
use crate::error::DownloadError;

/// Handle to an active or completed download.
///
/// Provides methods to query download progress, status, and retrieve results.
pub struct DownloadOperation {
    downloader: Arc<Downloader>,
    download_receiver: DownloadReceiver,
}

impl DownloadOperation {
    pub fn new(
        downloader: Arc<Downloader>,
        download_receiver: DownloadReceiver) -> DownloadOperation {
        DownloadOperation {
            downloader,
            download_receiver,
        }
    }

    /// Get the current download status.
    pub fn status(&self) -> DownloadStatus {
        let status = self.downloader.status();
        return status;
    }

    /// Get the number of bytes downloaded so far.
    pub fn downloaded_size(&self) -> u64 {
        self.download_receiver.downloaded_size()
    }

    /// Get the total file size in bytes.
    pub fn total_size(&self) -> u64 {
        return *self.download_receiver.download_total_size_receiver.borrow();
    }

    /// Get the download progress as a value between 0.0 and 1.0.
    pub fn progress(&self) -> f64 {
        let total_size = self.total_size();
        if total_size == 0 {
            return 0f64;
        }
        let total_length = total_size as f64;
        let downloaded_size = self.downloaded_size() as f64;
        return (downloaded_size / total_length).clamp(0f64, 1f64);
    }

    /// Get the downloaded data (only available for in-memory downloads).
    pub fn bytes(&self) -> Vec<u8> {
        let bytes = self.download_receiver.memory_receiver.as_ref().unwrap().borrow();
        bytes.to_vec()
    }

    /// Returns `true` if the download has completed (success or failure).
    pub fn is_done(&self) -> bool {
        return self.downloader.is_done();
    }

    /// Returns `true` if the download failed.
    pub fn is_error(&self) -> bool {
        return self.status() == DownloadStatus::Failed;
    }

    /// Get the error that caused the download to fail.
    pub fn error(&self) -> DownloadError {
        return (*self.download_receiver.error_receiver.borrow()).clone();
    }

    /// Stop the download.
    pub fn stop(&self) {
        self.downloader.stop();
    }
}