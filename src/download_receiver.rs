use tokio::sync::watch::{Receiver};

pub struct DownloadReceiver {
    pub downloaded_size_receiver: Receiver<u64>,
    pub download_total_size_receiver: Receiver<u64>,
    pub status_receiver: Receiver<u8>,
    pub is_done_receiver: Receiver<bool>,
}