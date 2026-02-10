use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::watch::channel;
use crate::download_receiver::DownloadReceiver;
use crate::download_sender::DownloadSender;
use crate::error::DownloadError;

pub fn new(download_in_memory: bool) -> (DownloadSender, DownloadReceiver) {
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

    let downloaded_size = Arc::new(AtomicU64::new(0));

    let sender = DownloadSender {
        download_total_size_sender,
        error_sender,
        memory_sender,
        downloaded_size: downloaded_size.clone(),
    };
    let receiver = DownloadReceiver {
        download_total_size_receiver,
        error_receiver,
        memory_receiver,
        downloaded_size,
    };
    return (sender, receiver);
}