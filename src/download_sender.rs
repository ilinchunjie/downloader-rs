use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::watch::Sender;
use crate::error::DownloadError;

pub struct DownloadSender {
    pub download_total_size_sender: Sender<u64>,
    pub error_sender: Sender<DownloadError>,
    pub memory_sender: Option<Sender<Vec<u8>>>,
    /// Shared counter for total downloaded bytes across all chunks.
    pub downloaded_size: Arc<AtomicU64>,
}