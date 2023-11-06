use std::collections::{VecDeque};
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use reqwest::Client;
use tokio::runtime;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_operation::DownloadOperation;
use crate::downloader::{Downloader};


type DownloaderQueue = VecDeque<Arc<Mutex<Downloader>>>;

pub struct DownloadService {
    worker_thread_count: u8,
    cancel: Arc<Mutex<bool>>,
    parallel_count: Arc<Mutex<u16>>,
    download_queue: Arc<Mutex<DownloaderQueue>>,
    thread_handle: Option<JoinHandle<()>>,
    client: Arc<Mutex<Client>>,
}

impl DownloadService {
    pub fn new() -> Self {
        Self {
            worker_thread_count: 4,
            download_queue: Arc::new(Mutex::new(DownloaderQueue::new())),
            parallel_count: Arc::new(Mutex::new(32)),
            thread_handle: None,
            cancel: Arc::new(Mutex::new(false)),
            client: Arc::new(Mutex::new(Client::new())),
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
                    for i in (0 as usize..downloadings.len()).rev() {
                        let downloader = downloadings.get(i).unwrap();
                        if downloader.lock().await.is_done_async().await {
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
        let mut downloader = Downloader::new(config, self.client.clone());
        downloader.pending();
        let downloader = Arc::new(Mutex::new(downloader));
        self.download_queue.blocking_lock().push_front(downloader.clone());
        let operation = DownloadOperation::new(downloader.clone());
        return operation;
    }

    pub fn stop(&mut self) {
        *self.cancel.blocking_lock() = true;
    }
}

#[cfg(test)]
mod test {
    use crate::download_configuration::DownloadConfiguration;
    use crate::download_service::DownloadService;

    #[test]
    pub fn test_download_service() {
        let mut service = DownloadService::new();
        service.start_service();
        let url = "https://gh.con.sh/https://github.com/AaronFeng753/Waifu2x-Extension-GUI/releases/download/v2.21.12/Waifu2x-Extension-GUI-v2.21.12-Portable.7z".to_string();
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path("temp/temp.7z".to_string())
            .set_chunk_download(true)
            .set_chunk_size(1024 * 1024 * 20)
            .create_dir(true)
            .build();
        let operation = service.add_downloader(config);

        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
        }

        service.stop();

    }
}