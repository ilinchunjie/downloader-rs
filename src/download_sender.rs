use tokio::sync::watch::Sender;
use crate::error::DownloadError;

pub struct DownloadSender {
    pub downloaded_size_sender: Sender<u64>,
    pub download_total_size_sender: Sender<u64>,
    pub status_sender: Sender<u8>,
    pub error_sender: Sender<DownloadError>,
}