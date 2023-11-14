use std::collections::{VecDeque};
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use reqwest::Client;
use tokio::runtime;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use crate::download_configuration::DownloadConfiguration;
use crate::download_operation::DownloadOperation;
use crate::download_tracker;
use crate::downloader::{Downloader};


type DownloaderQueue = VecDeque<Arc<Mutex<Downloader>>>;

pub struct DownloadService {
    worker_thread_count: usize,
    cancel_token: CancellationToken,
    parallel_count: Arc<RwLock<usize>>,
    download_queue: Arc<Mutex<DownloaderQueue>>,
    thread_handle: Option<JoinHandle<()>>,
    client: Arc<Client>,
}

impl DownloadService {
    pub fn new() -> Self {
        Self {
            worker_thread_count: 4,
            download_queue: Arc::new(Mutex::new(DownloaderQueue::new())),
            parallel_count: Arc::new(RwLock::new(32)),
            thread_handle: None,
            cancel_token: CancellationToken::new(),
            client: Arc::new(Client::new()),
        }
    }

    pub fn start_service(&mut self) {
        let cancel_token = self.cancel_token.clone();
        let queue = self.download_queue.clone();
        let parallel_count = self.parallel_count.clone();
        let worker_thread_count = self.worker_thread_count;
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(worker_thread_count)
                .enable_all()
                .build()
                .expect("runtime build failed");

            rt.block_on(async {
                while !cancel_token.is_cancelled() {
                    let mut futures = Vec::with_capacity(*parallel_count.read().await);
                    while futures.len() < *parallel_count.read().await && queue.lock().await.len() > 0 {
                        if let Some(downloader) = queue.lock().await.pop_front() {
                            if !downloader.lock().await.is_pending_async().await {
                                continue;
                            }
                            let future = downloader.lock().await.start_download();
                            futures.push(future);
                        }
                    }
                    for future in futures {
                        future.await;
                    }
                }
            })
        });

        self.thread_handle = Some(handle);
    }

    pub fn set_parallel_count(&mut self, parallel_count: usize) {
        *self.parallel_count.blocking_write() = parallel_count;
    }

    pub fn set_worker_thread_count(&mut self, worker_thread_count: usize) {
        self.worker_thread_count = worker_thread_count;
    }

    pub fn add_downloader(&mut self, config: DownloadConfiguration) -> DownloadOperation {
        let (tx, rx) = download_tracker::new();
        let mut downloader = Downloader::new(config, self.client.clone(), Arc::new(tx));
        downloader.pending();
        let downloader = Arc::new(Mutex::new(downloader));
        self.download_queue.blocking_lock().push_back(downloader.clone());
        let operation = DownloadOperation::new(downloader.clone(), rx);
        return operation;
    }

    pub fn is_finished(&self) -> bool {
        if let Some(handle) = &self.thread_handle {
            return handle.is_finished();
        }
        return false;
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::thread::sleep;
    use std::time::Duration;
    use tokio::runtime;
    use tokio::time::Instant;
    use crate::download_configuration::DownloadConfiguration;
    use crate::download_service::DownloadService;

    #[test]
    pub fn test_download_service() {
        let mut service = DownloadService::new();
        service.start_service();
        let url = "https://gh.con.sh/https://github.com/AaronFeng753/Waifu2x-Extension-GUI/releases/download/v2.21.12/Waifu2x-Extension-GUI-v2.21.12-Portable.7z".to_string();
        let config = DownloadConfiguration::new()
            .set_url(&url)
            .set_file_path("temp/temp.7z")
            .set_chunk_download(true)
            .set_chunk_size(1024 * 1024 * 10)
            .set_retry_times_on_failure(2)
            .build();
        let operation = service.add_downloader(config);


        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
        }

        if operation.is_error() {
            println!("{}", operation.error());
        }

        service.stop();
    }
}