use tokio::sync::watch::Sender;

pub struct DownloadSender {
    pub downloaded_size_sender: Sender<u64>,
    pub download_total_size_sender: Sender<u64>,
    pub status_sender: Sender<u8>,
    pub is_done_sender: Sender<bool>,
}