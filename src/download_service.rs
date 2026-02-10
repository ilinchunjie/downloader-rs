use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use reqwest::{Client, ClientBuilder};
use parking_lot::RwLock;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use crate::download_configuration::DownloadConfiguration;
use crate::download_operation::DownloadOperation;
use crate::download_tracker;
use crate::downloader::Downloader;
use tracing;

type DownloaderQueue = VecDeque<Arc<Downloader>>;

/// Service that manages concurrent downloads with configurable parallelism.
///
/// Call [`run()`](DownloadService::run) within your Tokio runtime to start the scheduling loop.
pub struct DownloadService {
    cancel_token: CancellationToken,
    parallel_count: Arc<RwLock<usize>>,
    download_queue: Arc<RwLock<DownloaderQueue>>,
    client: Arc<Client>,
}

impl DownloadService {
    /// Create a new download service with default settings.
    pub fn new() -> Self {
        let client = ClientBuilder::new()
            .use_rustls_tls()
            .build()
            .unwrap();
        Self {
            download_queue: Arc::new(RwLock::new(DownloaderQueue::new())),
            parallel_count: Arc::new(RwLock::new(32)),
            cancel_token: CancellationToken::new(),
            client: Arc::new(client),
        }
    }

    /// Run the download scheduling loop.
    /// This runs within the caller's existing Tokio runtime — no self-built Runtime.
    ///
    /// NOTE: All RwLock guard accesses are scoped to ensure guards are dropped
    /// before any `.await` point, making this future `Send`.
    pub async fn run(&self) {
        tracing::info!("download service started");
        let mut downloadings: Vec<Arc<Downloader>> = Vec::new();

        loop {
            if self.cancel_token.is_cancelled() {
                break;
            }

            // Read parallel limit and queue length with guards dropped immediately
            let parallel_limit = { *self.parallel_count.read() };
            let mut queue_has_items = { self.download_queue.read().len() > 0 };

            // Start new downloads up to parallel limit
            while downloadings.len() < parallel_limit && queue_has_items {
                let next = { self.download_queue.write().pop_front() };
                match next {
                    Some(downloader) => {
                        if !downloader.is_pending_async().await {
                            queue_has_items = self.download_queue.read().len() > 0;
                            continue;
                        }
                        downloadings.push(downloader.clone());
                        tracing::debug!(active = downloadings.len(), "starting download task");
                        downloader.start_download();
                    }
                    None => break,
                }
                queue_has_items = self.download_queue.read().len() > 0;
            }

            // Remove completed downloads
            downloadings.retain(|d| !d.is_done());

            // Handle parallel count reduction
            let current_parallel = { *self.parallel_count.read() };
            if downloadings.len() > current_parallel {
                let mut remove_count = downloadings.len() - current_parallel;
                while remove_count > 0 && !downloadings.is_empty() {
                    let index = downloadings.len() - 1;
                    let downloader = downloadings[index].clone();
                    downloader.stop_async().await;
                    downloader.pending_async().await;
                    { self.download_queue.write().push_back(downloader); }
                    downloadings.remove(index);
                    remove_count -= 1;
                }
            }

            tokio::select! {
                _ = sleep(Duration::from_millis(100)) => {}
                _ = self.cancel_token.cancelled() => break,
            }
        }
    }

    /// Set the maximum number of concurrent downloads.
    pub fn set_parallel_count(&mut self, parallel_count: usize) {
        *self.parallel_count.write() = parallel_count;
    }

    /// Add a download to the queue and return a handle to monitor it.
    pub fn add_downloader(&mut self, config: DownloadConfiguration) -> DownloadOperation {
        let (tx, rx) = download_tracker::new(config.download_in_memory);
        let mut downloader = Downloader::new(config, self.client.clone(), Arc::new(tx));
        downloader.pending();
        let downloader = Arc::new(downloader);
        self.download_queue.write().push_back(downloader.clone());
        let operation = DownloadOperation::new(downloader.clone(), rx);
        return operation;
    }

    pub fn stop(&self) {
        tracing::info!("stopping download service");
        self.cancel_token.cancel();
    }
}

#[cfg(test)]
mod test {
    use crate::download_configuration::DownloadConfiguration;
    use crate::download_service::DownloadService;

    #[tokio::test]
    pub async fn test_download_service() {
        let mut service = DownloadService::new();
        let url = "https://lan.sausage.xd.com/servers.txt".to_string();
        let config = DownloadConfiguration::new()
            .set_url(&url)
            .set_download_in_memory(true)
            .set_retry_times_on_failure(2)
            .set_timeout(5)
            .build()
            .unwrap();
        let operation = service.add_downloader(config);

        // Spawn the service in the background
        let service_handle = tokio::spawn(async move {
            service.run().await;
        });

        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        if operation.is_error() {
            println!("{}", operation.error());
        }

        let bytes = operation.bytes();
        println!("{}", bytes.len());

        // The service handle will be dropped and cancelled
        service_handle.abort();
    }
}