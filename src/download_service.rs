use std::collections::VecDeque;
use std::ops::DerefMut;
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use tokio::runtime;
use tokio::sync::Mutex;
use crate::downloader::Downloader;

pub struct DownloadService {
    cancel: Arc<Mutex<bool>>,
    download_queue: Arc<Mutex<VecDeque<usize>>>,
    downloaders: Arc<Mutex<Vec<Arc<Mutex<Downloader>>>>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl DownloadService {
    pub fn start_service(&mut self) {
        let cancel = self.cancel.clone();
        let downloaders = self.downloaders.clone();
        let queue = self.download_queue.clone();
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("创建失败");

            rt.block_on(async {
                while !*cancel.lock().await {
                    if let Some(index) = queue.lock().await.pop_front() {
                        if let Some(downloader) = &mut downloaders.lock().await.get(index) {
                            downloader.lock().await.start_download();
                        }
                    }
                }
            })
        });

        self.thread_handle = Some(handle);
    }

    pub fn add_downloader(&mut self, downloader: Downloader) -> usize {
        let len = self.downloaders.blocking_lock().len();
        self.downloaders.blocking_lock().push(Arc::new(Mutex::new(downloader)));
        self.download_queue.blocking_lock().push_front(len);
        return len;
    }

    pub fn get_downloaded_size(&mut self, index: usize) -> u64 {
        if let Some(downloader) = self.downloaders.blocking_lock().get(index) {
            return downloader.blocking_lock().get_downloaded_size();
        }
        return 0;
    }

    pub fn get_download_status(&mut self, index: usize) -> i32 {
        if let Some(downloader) = self.downloaders.blocking_lock().get(index) {
            return downloader.blocking_lock().get_download_status();
        }
        return 0;
    }

    pub fn stop(&mut self) {
        *self.cancel.blocking_lock() = true;
    }
}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::thread::sleep;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use crate::download_configuration::DownloadConfiguration;
    use crate::download_service::DownloadService;
    use crate::downloader::{Downloader};

    #[test]
    fn test_download_service() {
        let mut service = DownloadService {
            download_queue: Arc::new(Mutex::new(VecDeque::new())),
            downloaders: Arc::new(Mutex::new(Vec::new())),
            thread_handle: None,
            cancel: Arc::new(Mutex::new(false))
        };

        service.start_service();

        let url = "https://n17x06.xdcdn.net/media/SS6_CG_Weather_Kingdom.mp4".to_string();
        let mut downloader = Downloader::new(DownloadConfiguration::from_url_path(url, "SS6_CG_Weather_Kingdom.mp4".to_string()));
        let index0 = service.add_downloader(downloader);

        let url = "https://lan.sausage.xd.com/servers.txt".to_string();
        let mut downloader = Downloader::new(DownloadConfiguration::from_url_path(url, "servers.txt".to_string()));
        let index1 = service.add_downloader(downloader);

        while service.get_download_status(index0) != 4 || service.get_download_status(index1) != 4 {
            println!("file1->{}", service.get_downloaded_size(index0));
            println!("file2->{}", service.get_downloaded_size(index1));
        }

        service.stop();
    }
}