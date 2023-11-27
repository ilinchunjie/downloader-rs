use std::collections::{VecDeque};
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use reqwest::Client;
use parking_lot::RwLock;
use tokio::runtime;
use tokio::sync::{Mutex};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use crate::download_configuration::DownloadConfiguration;
use crate::download_operation::DownloadOperation;
use crate::download_tracker;
use crate::downloader::{Downloader};


type DownloaderQueue = VecDeque<Arc<Downloader>>;

pub struct DownloadService {
    multi_thread: bool,
    worker_thread_count: usize,
    cancel_token: CancellationToken,
    parallel_count: Arc<RwLock<usize>>,
    download_queue: Arc<RwLock<DownloaderQueue>>,
    thread_handle: Option<JoinHandle<()>>,
    client: Arc<Client>,
}

impl DownloadService {
    pub fn new() -> Self {
        Self {
            multi_thread: false,
            worker_thread_count: 4,
            download_queue: Arc::new(RwLock::new(DownloaderQueue::new())),
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
        let multi_thread = self.multi_thread;
        let handle = thread::spawn(move || {
            let rt = match multi_thread {
                true => {
                    runtime::Builder::new_multi_thread()
                        .worker_threads(worker_thread_count)
                        .enable_all()
                        .build()
                        .expect("runtime build failed")
                }
                false => {
                    runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime build failed")
                }
            };

            rt.block_on(async {
                let mut downloading_count = 0;
                let mut downloadings = Vec::new();
                while !cancel_token.is_cancelled() {
                    while downloading_count < *parallel_count.read() && queue.read().len() > 0 {
                        if let Some(downloader) = queue.write().pop_front() {
                            let downloader_clone = downloader.clone();
                            if !downloader.is_pending_async().await {
                                continue;
                            }
                            let _ = &mut downloadings.push(downloader_clone);
                            downloading_count += 1;
                            downloader.start_download();
                        }
                    }
                    for i in (0..downloadings.len()).rev() {
                        let downloader = downloadings.get(i).unwrap();
                        if downloader.is_done() {
                            downloadings.remove(i);
                            downloading_count -= 1;
                        }
                    }
                    if downloadings.len() > *parallel_count.read() {
                        let mut remove_count = downloadings.len() - *parallel_count.read();
                        while remove_count > 0 {
                            let index = downloadings.len() - 1;
                            let downloader = downloadings.get(index).unwrap();
                            downloader.stop_async().await;
                            downloader.pending_async().await;
                            queue.write().push_back(downloader.clone());
                            downloadings.remove(downloadings.len() - 1);
                            remove_count -= 1;
                            downloading_count -= 1;
                        }
                    }
                    sleep(Duration::from_millis(300)).await;
                }
            })
        });

        self.thread_handle = Some(handle);
    }

    pub fn set_multi_thread(mut self, multi_thread: bool) -> DownloadService {
        self.multi_thread = multi_thread;
        self
    }

    pub fn set_worker_thread_count(mut self, worker_thread_count: usize) -> DownloadService {
        self.worker_thread_count = worker_thread_count;
        self
    }

    pub fn set_parallel_count(&mut self, parallel_count: usize) {
        *self.parallel_count.write() = parallel_count;
    }

    pub fn add_downloader(&mut self, config: DownloadConfiguration) -> DownloadOperation {
        let (tx, rx) = download_tracker::new();
        let mut downloader = Downloader::new(config, self.client.clone(), Arc::new(tx));
        downloader.pending();
        let downloader = Arc::new(downloader);
        self.download_queue.write().push_back(downloader.clone());
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
            .set_chunk_size(1024 * 1024 * 30)
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