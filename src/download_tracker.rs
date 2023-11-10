use tokio::sync::watch::{channel};
use crate::download_receiver::DownloadReceiver;
use crate::download_sender::DownloadSender;
use crate::error::DownloadError;

pub fn new() -> (DownloadSender, DownloadReceiver) {
    let (downloaded_size_sender, downloaded_size_receiver) = channel(0u64);
    let (download_total_size_sender, download_total_size_receiver) = channel(0u64);
    let (status_sender, status_receiver) = channel(0u8);
    let (error_sender, error_receiver) = channel(DownloadError::None);
    let sender = DownloadSender {
        downloaded_size_sender,
        download_total_size_sender,
        status_sender,
        error_sender,
    };
    let receiver = DownloadReceiver {
        downloaded_size_receiver,
        download_total_size_receiver,
        status_receiver,
        error_receiver
    };
    return (sender, receiver);
}