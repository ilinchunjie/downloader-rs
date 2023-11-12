use std::collections::{VecDeque};
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use reqwest::Client;
use tokio::runtime;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_operation::DownloadOperation;
use crate::download_tracker;
use crate::downloader::{Downloader};


type DownloaderQueue = VecDeque<Arc<Mutex<Downloader>>>;

pub struct DownloadService {
    worker_thread_count: u8,
    cancel: Arc<Mutex<bool>>,
    parallel_count: Arc<Mutex<u16>>,
    download_queue: Arc<Mutex<DownloaderQueue>>,
    thread_handle: Option<JoinHandle<()>>,
    client: Arc<Client>,
}

impl DownloadService {
    pub fn new() -> Self {
        Self {
            worker_thread_count: 4,
            download_queue: Arc::new(Mutex::new(DownloaderQueue::new())),
            parallel_count: Arc::new(Mutex::new(32)),
            thread_handle: None,
            cancel: Arc::new(Mutex::new(false)),
            client: Arc::new(Client::new()),
        }
    }

    pub fn start_service(&mut self) {
        let cancel = self.cancel.clone();
        let queue = self.download_queue.clone();
        let parallel_count = self.parallel_count.clone();
        let worker_thread_count = self.worker_thread_count as usize;
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(worker_thread_count)
                .enable_all()
                .build()
                .expect("runtime build failed");

            rt.block_on(async {
                let mut downloading_count = 0;
                let mut downloadings = Vec::new();
                while !*cancel.lock().await {
                    if downloading_count < *parallel_count.lock().await {
                        if let Some(downloader) = queue.lock().await.pop_front() {
                            let downloader_clone = downloader.clone();
                            if !downloader.lock().await.is_pending_async().await {
                                continue;
                            }
                            let _ = &mut downloadings.push(downloader_clone);
                            downloading_count += 1;
                            downloader.lock().await.start_download();
                        }
                    }
                    for i in (0..downloadings.len()).rev() {
                        let downloader = downloadings.get(i).unwrap();
                        if downloader.lock().await.is_done() {
                            downloadings.remove(i);
                            downloading_count -= 1;
                        }
                    }
                }
            })
        });

        self.thread_handle = Some(handle);
    }

    pub fn set_parallel_count(&mut self, parallel_count: u16) {
        *self.parallel_count.blocking_lock() = parallel_count;
    }

    pub fn set_worker_thread_count(&mut self, worker_thread_count: u8) {
        self.worker_thread_count = worker_thread_count;
    }

    pub fn add_downloader(&mut self, config: DownloadConfiguration) -> DownloadOperation {
        let (tx, rx) = download_tracker::new();
        let mut downloader = Downloader::new(config, self.client.clone(), Arc::new(tx));
        downloader.pending();
        let downloader = Arc::new(Mutex::new(downloader));
        self.download_queue.blocking_lock().push_front(downloader.clone());
        let operation = DownloadOperation::new(downloader.clone(), rx);
        return operation;
    }

    pub fn is_finished(&self) -> bool {
        if let Some(handle) = &self.thread_handle {
            return handle.is_finished();
        }
        return false;
    }

    pub fn stop(&mut self) {
        *self.cancel.blocking_lock() = true;
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
            .set_chunk_download(false)
            .set_chunk_size(1024 * 1024 * 20)
            .set_retry_times_on_failure(2)
            .set_download_speed_limit(1024 * 1024)
            .build();
        let operation = service.add_downloader(config);


        let mut last_downloaded_size = 0u64;
        let mut last_time = Instant::now();
        while !operation.is_done() {
            sleep(Duration::from_secs(5));
            let downloaded_size = operation.downloaded_size();
            if downloaded_size > last_downloaded_size {
                let delta = (downloaded_size - last_downloaded_size) as f64;
                let seconds = Instant::now().duration_since(last_time).as_secs_f64();

                println!("download speed {} per seconds", (delta / seconds) / 1024f64 / 1024f64);

                last_downloaded_size = downloaded_size;
                last_time = Instant::now();
            }
        }

        if operation.is_error() {
            println!("{}", operation.error());
        }

        service.stop();
    }
}