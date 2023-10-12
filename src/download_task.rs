use std::fmt::{Debug};
use std::ops::Deref;
use std::sync::{Arc};
use tokio::spawn;
use crate::download_configuration::DownloadConfiguration;
use crate::stream::Stream;
use futures::StreamExt;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::download_handle::DownloadHandle;
use crate::download_status::DownloadStatus;
use crate::request;

pub struct DownloadTask {
    config: DownloadConfiguration,
    handle: Arc<Mutex<DownloadHandle>>,
    status: Arc<Mutex<DownloadStatus>>,
    cancel: Arc<Mutex<bool>>,
    join_handle: Option<JoinHandle<()>>,
}

impl DownloadTask {
    pub fn new(config: DownloadConfiguration, stream: Stream) -> DownloadTask {
        DownloadTask {
            config,
            cancel: Arc::new(Mutex::new(false)),
            status: Arc::new(Mutex::new(DownloadStatus::None)),
            handle: Arc::new(Mutex::new(DownloadHandle::new(stream))),
            join_handle: None,
        }
    }

    pub fn start(&mut self) {
        let handle = spawn(start_download(
            self.config.clone(),
            self.handle.clone(),
            self.status.clone(),
            self.cancel.clone()));
        self.join_handle = Some(handle);
    }

    pub fn is_done(&self) -> bool {
        match &self.join_handle {
            Some(handle) => {
                return handle.is_finished();
            }
            None => false
        }
    }

    pub async fn stop(&self) {
        *self.cancel.lock().await = true;
    }
}

async fn start_download(config: DownloadConfiguration,
                        handle: Arc<Mutex<DownloadHandle>>,
                        status: Arc<Mutex<DownloadStatus>>,
                        cancel: Arc<Mutex<bool>>) {
    let request = request::get_download_request(&config);
    let mut status = status.lock().await;
    let mut response = request.send().await.unwrap();
    if *cancel.lock().await {
        return;
    }
    if response.status().is_success() {
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            if *cancel.lock().await {
                return;
            }
            match chunk {
                Ok(bytes) => {
                    let buffer = bytes.to_vec() as Vec<u8>;
                    println!("{}", buffer.len());
                    handle.lock().await.on_received_bytes_async(&buffer).await;
                }
                Err(e) => {
                    *status = DownloadStatus::Failed;
                }
            }
        }
        handle.lock().await.on_complete_async().await;
        *status = DownloadStatus::Complete;
    } else {
        *status = DownloadStatus::Failed;
    }
}

#[cfg(test)]
mod test {
    use std::{fs, thread};
    use std::sync::{Arc};
    use std::time::Duration;
    use tokio::runtime;
    use tokio::sync::Mutex;
    use tokio::time::{Instant, sleep};
    use crate::download_configuration::DownloadConfiguration;
    use crate::download_task::DownloadTask;
    use crate::stream::Stream;

    #[tokio::test]
    async fn test_download() {
        let handle = thread::spawn(move || {
            let rt = runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .expect("创建失败");

            rt.block_on(async {
                let save_path = "servers.txt";
                let temp_path = "servers.txt.temp";
                let mut success = false;
                {
                    let address = "https://www.7-zip.org/a/7z2301-x64.exe";
                    let mut config = DownloadConfiguration::new(address.to_string(), save_path.to_string());
                    config.range_download = false;
                    let stream = Stream::new(&config.temp_path).await;
                    let mut downloader = DownloadTask::new(config, stream);
                    let time = Instant::now();
                    downloader.start();

                    while !downloader.is_done() {}

                    println!("took {}s", Instant::now().duration_since(time).as_secs());
                    success = true;
                }

                if success {
                    fs::rename(temp_path, save_path).expect("文件移动失败");
                }
            })
        });

        handle.join().expect("");
    }
}