use tokio::sync::watch::{channel};
use crate::download_receiver::DownloadReceiver;
use crate::download_sender::DownloadSender;
use crate::error::DownloadError;

pub fn new(download_in_memory: bool) -> (DownloadSender, DownloadReceiver) {
    let (downloaded_size_sender, downloaded_size_receiver) = channel(0u64);
    let (download_total_size_sender, download_total_size_receiver) = channel(0u64);
    let (error_sender, error_receiver) = channel(DownloadError::None);
    let (memory_sender, memory_receiver) = match download_in_memory {
        true => {
            let (memory_sender, memory_receiver) = channel(vec![]);
            (Some(memory_sender), Some(memory_receiver))
        }
        false => {
            (None, None)
        }
    };
    let sender = DownloadSender {
        downloaded_size_sender,
        download_total_size_sender,
        error_sender,
        memory_sender,
    };
    let receiver = DownloadReceiver {
        downloaded_size_receiver,
        download_total_size_receiver,
        error_receiver,
        memory_receiver,
    };
    return (sender, receiver);
}