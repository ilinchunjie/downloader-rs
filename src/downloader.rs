use std::ops::Deref;
use std::sync::{Arc};
use bytes::Buf;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::download_task::{DownloadTaskConfiguration, DownloadTask};
use crate::remote_file::RemoteFile;

#[derive(PartialEq)]
enum DownloaderStatus {
    None = 0,
    Head = 1,
    Download = 2,
    Complete = 3,
}

pub struct Downloader {
    file_path: String,
    remote_url: Arc<String>,
    remote_file: Arc<Mutex<RemoteFile>>,
    remote_file_request_handle: Option<JoinHandle<()>>,
    download_tasks: Option<Vec<Arc<Mutex<DownloadTask>>>>,
    download_task_handles: Option<Vec<JoinHandle<()>>>,
    download_status: DownloaderStatus,
}

impl Downloader {
    pub fn new(remote_url: String, file_path: String) -> Downloader {
        let remote_url= Arc::new(remote_url);
        let remote_url_clone = remote_url.clone();
        let mut downloader = Downloader {
            file_path,
            remote_url,
            remote_file: Arc::new(Mutex::new(RemoteFile::new(remote_url_clone))),
            remote_file_request_handle: None,
            download_tasks: None,
            download_task_handles: None,
            download_status: DownloaderStatus::None,
        };
        downloader
    }

    pub async fn start(&mut self) {
        if self.download_status != DownloaderStatus::None {
            return;
        }
        self.set_downloader_status(DownloaderStatus::Head).await;
    }

    pub async fn update(&mut self) {
        match self.download_status {
            DownloaderStatus::Head => {
                if let Some(handle) = &self.remote_file_request_handle {
                    if handle.is_finished() {
                        self.set_downloader_status(DownloaderStatus::Download).await;
                    }
                }
            }
            DownloaderStatus::Download => {
                if let Some(handles) = &self.download_task_handles {
                    let mut complete = true;
                    for handle in handles {
                        if !handle.is_finished() {
                            complete = false;
                        }
                    }
                    if complete {
                        self.set_downloader_status(DownloaderStatus::Complete).await;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn stop() {}

    fn start_head(&mut self) {
        let handle = tokio::spawn(async_remote_file(self.remote_file.clone()));
        self.remote_file_request_handle = Some(handle);
    }

    async fn start_download(&mut self) {
        let remote_file = self.remote_file.lock().await;
        let mut chunks: Option<Vec<(u64, u64)>> = None;
        if let Some(remote_file_info) = &remote_file.remote_file_info {
            let range_download = remote_file_info.support_range_download;
            let total_length = remote_file_info.total_length;
            chunks = get_download_chunks(total_length, range_download);
        }

        self.download_tasks = Some(vec![]);
        if let Some(chunks) = chunks {
            for (i, chunk) in chunks.iter().enumerate() {
                let config = DownloadTaskConfiguration {
                    url: self.remote_url.clone(),
                    range_start: chunk.0,
                    range_end: chunk.1,
                    range_download: true,
                };
                let path = format!("{}.chunk{}", self.file_path, i);
                println!("{}", path);
                let task = DownloadTask::new(path, config);
                let task = Arc::new(Mutex::new(task));
                self.download_tasks.as_mut().unwrap().push(task);
            }
        } else {
            let config = DownloadTaskConfiguration {
                url: self.remote_url.clone(),
                range_start: 0,
                range_end: 0,
                range_download: false,
            };
            let task = DownloadTask::new(self.file_path.clone(), config);
            let task = Arc::new(Mutex::new(task));
            self.download_tasks.as_mut().unwrap().push(task);
        }

        if let Some(tasks) = &self.download_tasks {
            self.download_task_handles = Some(vec![]);
            for task in tasks {
                let handle = spawn(async_download_task(task.clone()));
                self.download_task_handles.as_mut().unwrap().push(handle);
            }
        }
    }

    async fn set_downloader_status(&mut self, status: DownloaderStatus) {
        match self.download_status {
            _ => {}
        }
        self.download_status = status;
        match self.download_status {
            DownloaderStatus::Head => {
                self.start_head();
            }
            DownloaderStatus::Download => {
                self.start_download().await;
            }
            _ => {}
        }
    }
}

fn get_download_chunks(total_length: u64, range_download: bool) -> Option<Vec<(u64, u64)>> {
    println!("{}", total_length);
    if !range_download || total_length < 1024 * 50 {
        return None;
    }
    let mut chunks: Vec<(u64, u64)> = vec![];
    let chunk_count = 3;
    let chunk_size = total_length / chunk_count;
    for i in 0..chunk_count {
        let start_position = i * chunk_size;
        let mut end_position = start_position + chunk_size - 1;
        if i == chunk_count - 1 {
            end_position = end_position + total_length % chunk_count;
        }
        chunks.push((start_position, end_position));
    }
    Some(chunks)
}

async fn async_remote_file(remote_file: Arc<Mutex<RemoteFile>>) {
    let mut remote_file = remote_file.lock().await;
    remote_file.head().await;
}

async fn async_download_task(task: Arc<Mutex<DownloadTask>>) {
    let mut task = task.lock().await;
    task.start_download().await;
}

#[cfg(test)]
mod test {
    use std::thread;
    use tokio::runtime;
    use crate::downloader::{Downloader, DownloaderStatus};

    #[tokio::test]
    async fn test_downloader() {
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("创建失败");

            rt.block_on(async {
                let url = "https://lan.sausage.xd.com/servers.txt".to_string();
                let mut downloader = Downloader::new(url, "servers.txt".to_string());
                downloader.start().await;
                while downloader.download_status != DownloaderStatus::Complete {
                    downloader.update().await;
                }

                println!("complete")
            })
        });

        handle.join().expect("");
    }
}