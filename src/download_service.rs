use std::collections::{HashMap, VecDeque};
use std::ops::DerefMut;
use std::sync::{Arc};
use std::thread;
use std::thread::JoinHandle;
use tokio::runtime;
use tokio::sync::Mutex;
use crate::downloader::Downloader;

type Downloaders = HashMap<u64, Arc<Mutex<Downloader>>>;

pub struct DownloadService {
    instance_id: u64,
    cancel: Arc<Mutex<bool>>,
    download_queue: Arc<Mutex<VecDeque<u64>>>,
    downloaders: Arc<Mutex<Downloaders>>,
    thread_handle: Option<JoinHandle<()>>,
}

impl DownloadService {
    pub fn new() -> Self {
        Self {
            instance_id: 0u64,
            download_queue: Arc::new(Mutex::new(VecDeque::new())),
            downloaders: Arc::new(Mutex::new(Downloaders::new())),
            thread_handle: None,
            cancel: Arc::new(Mutex::new(false)),
        }
    }

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
                    if let Some(id) = queue.lock().await.pop_front() {
                        if let Some(downloader) = &mut downloaders.lock().await.get(&id) {
                            downloader.lock().await.start_download();
                        }
                    }
                }
            })
        });

        self.thread_handle = Some(handle);
    }

    pub fn add_downloader(&mut self, downloader: Downloader) -> u64 {
        self.instance_id = self.instance_id + 1;
        let downloader = Arc::new(Mutex::new(downloader));
        self.downloaders.blocking_lock().insert(self.instance_id, downloader);
        self.download_queue.blocking_lock().push_front(self.instance_id);
        return self.instance_id;
    }

    pub fn remove_downloader(&mut self, id: u64) {
        if let Some(downloader) = self.downloaders.blocking_lock().get(&id) {
            downloader.blocking_lock().stop();
            self.downloaders.blocking_lock().remove(&id);
        }
    }

    pub fn get_downloaded_size(&mut self, id: u64) -> u64 {
        if let Some(downloader) = self.downloaders.blocking_lock().get(&id) {
            return downloader.blocking_lock().get_downloaded_size();
        }
        return 0;
    }

    pub fn get_download_status(&mut self, id: u64) -> i32 {
        if let Some(downloader) = self.downloaders.blocking_lock().get(&id) {
            return downloader.blocking_lock().get_download_status();
        }
        return 0;
    }

    pub fn get_download_is_done(&mut self, id: u64) -> bool {
        if let Some(downloader) = self.downloaders.blocking_lock().get(&id) {
            return downloader.blocking_lock().is_done();
        }
        return true;
    }

    pub fn stop(&mut self) {
        *self.cancel.blocking_lock() = true;
    }
}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use std::process::id;
    use std::sync::Arc;
    use std::thread::sleep;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use crate::download_configuration::DownloadConfiguration;
    use crate::download_service::{Downloaders, DownloadService};
    use crate::downloader::{Downloader};

    #[test]
    fn test_download_service() {
        let mut service = DownloadService {
            instance_id: 0u64,
            download_queue: Arc::new(Mutex::new(VecDeque::new())),
            downloaders: Arc::new(Mutex::new(Downloaders::new())),
            thread_handle: None,
            cancel: Arc::new(Mutex::new(false)),
        };

        service.start_service();

        let url = "https://n17x06.xdcdn.net/media/SS6_CG_Weather_Kingdom.mp4".to_string();
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path("temp/SS6_CG_Weather_Kingdom.mp4".to_string())
            .create_dir(true)
            .build();
        let mut downloader = Downloader::new(config);
        let id0 = service.add_downloader(downloader);

        while !service.get_download_is_done(id0) {
            println!("{}", service.get_downloaded_size(id0));
        }

        service.stop();
    }
}