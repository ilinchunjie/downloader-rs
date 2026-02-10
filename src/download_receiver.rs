use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch::Receiver;
use crate::error::DownloadError;

pub struct DownloadReceiver {
    pub download_total_size_receiver: Receiver<u64>,
    pub error_receiver: Receiver<DownloadError>,
    pub memory_receiver: Option<Receiver<Vec<u8>>>,
    /// Shared counter for total downloaded bytes — same Arc as in DownloadSender.
    pub downloaded_size: Arc<AtomicU64>,
}

impl DownloadReceiver {
    /// Returns the total downloaded size.
    pub fn downloaded_size(&self) -> u64 {
        self.downloaded_size.load(Ordering::Relaxed)
    }
}