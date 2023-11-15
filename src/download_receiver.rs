use tokio::sync::watch::{Receiver};
use crate::error::DownloadError;

pub struct DownloadReceiver {
    pub downloaded_size_receiver: Receiver<u64>,
    pub download_total_size_receiver: Receiver<u64>,
    pub error_receiver: Receiver<DownloadError>,
}