use tokio::time::Instant;
use crate::stream::Stream;

pub struct DownloadHandle {
    downloaded_size : i64,
    download_size : i64,
    download_speed : f64,
    last_update_time : Option<Instant>,
    stream: Stream,
}

impl DownloadHandle {
    pub fn new(stream : Stream) -> Self {
        Self {
            stream,
            downloaded_size : 0,
            download_size : 0,
            download_speed : 0.,
            last_update_time : None,
        }
    }

    pub async fn on_received_bytes_async(&mut self, buffer : &Vec<u8>) {
        self.downloaded_size += buffer.len() as i64;
        self.download_size = buffer.len() as i64;
        self.stream.write_async(buffer).await;
        self.update_download_speed();
        self.last_update_time = Some(Instant::now());
    }

    pub async fn on_complete_async(&mut self) {
        self.stream.flush_async().await;
    }

    pub fn get_download_speed(&self) -> f64 {
        self.download_speed
    }

    fn update_download_speed(&mut self) {
        if self.last_update_time == None {
            self.download_speed = 0.;
        } else {
            let delta_time = Instant::now().duration_since(self.last_update_time.unwrap());
            self.download_speed = self.download_size as f64 / delta_time.as_secs() as f64;
        }
    }
}