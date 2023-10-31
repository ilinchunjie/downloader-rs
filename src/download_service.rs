use std::collections::{VecDeque};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use tokio::runtime;
use tokio::sync::Mutex;
use crate::download_configuration::DownloadConfiguration;
use crate::download_operation::DownloadOperation;
use crate::downloader::Downloader;

type DownloaderQueue = VecDeque<Arc<Mutex<Downloader>>>;

pub struct DownloadService {
    cancel: Arc<Mutex<bool>>,
    download_queue: Arc<Mutex<DownloaderQueue>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl DownloadService {
    pub fn new() -> Self {
        Self {
            download_queue: Arc::new(Mutex::new(DownloaderQueue::new())),
            thread_handle: None,
            cancel: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start_service(&mut self) {
        let cancel = self.cancel.clone();
        let queue = self.download_queue.clone();
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("创建失败");

            rt.block_on(async {
                while !*cancel.lock().await {
                    if let Some(downloader) = queue.lock().await.pop_front() {
                        if downloader.lock().await.is_stop_async().await {
                            continue;
                        }
                        downloader.lock().await.start_download();
                    }
                }
            })
        });

        self.thread_handle = Some(handle);
    }

    pub fn add_downloader(&mut self, config: DownloadConfiguration) -> DownloadOperation {
        let downloader = Downloader::new(config);
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
    use crate::download_service::{DownloadService};
    use crate::downloader::{Downloader};

    #[test]
    fn test_download_service() {
        let mut service = DownloadService::new();

        service.start_service();

        let url = "https://lan.sausage.xd.com/servers.txt".to_string();
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path("temp/servers.txt".to_string())
            .build();
        let operation = service.add_downloader(config);

        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
        }

        println!("{}", operation.text());

        service.stop();
    }
}