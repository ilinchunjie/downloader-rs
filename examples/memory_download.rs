//! In-memory download example.
//!
//! Downloads content directly into memory without writing to disk.
//!
//! Usage: cargo run --example memory_download

use std::time::Duration;
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_service::DownloadService;
use downloader_rs::download_status::DownloadStatus;

#[tokio::main]
async fn main() {
    let mut service = DownloadService::new();

    let config = DownloadConfiguration::new()
        .set_url("https://httpbin.org/bytes/1024")
        .set_download_in_memory(true)
        .set_timeout(10)
        .build()
        .expect("Invalid download configuration");

    let operation = service.add_downloader(config);

    // Run the download service in the background
    let service_handle = tokio::spawn(async move {
        service.run().await;
    });

    // Wait for download to complete
    loop {
        match operation.status() {
            DownloadStatus::Complete => {
                let data = operation.bytes();
                println!("Downloaded {} bytes into memory", data.len());
                println!("First 16 bytes: {:?}", &data[..data.len().min(16)]);
                break;
            }
            DownloadStatus::Failed => {
                eprintln!("Download failed: {}", operation.error());
                break;
            }
            _ => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    service_handle.abort();
}
