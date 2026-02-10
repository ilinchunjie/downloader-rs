//! Basic file download example.
//!
//! Downloads a file to disk with chunked download, speed limiting, and progress tracking.
//!
//! Usage: cargo run --example basic_download

use std::time::Duration;
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_service::DownloadService;
use downloader_rs::download_status::DownloadStatus;

#[tokio::main]
async fn main() {
    let mut service = DownloadService::new();
    service.set_parallel_count(4);

    let config = DownloadConfiguration::new()
        .set_url("https://httpbin.org/bytes/102400")
        .set_file_path("./downloads/test_file.bin")
        .set_range_download(true)
        .set_chunk_download(true)
        .set_chunk_size(1024 * 1024 * 2) // 2 MB chunks
        .set_download_speed_limit(1024 * 1024 * 5) // 5 MB/s limit
        .set_retry_times_on_failure(3)
        .set_timeout(30)
        .build()
        .expect("Invalid download configuration");

    let operation = service.add_downloader(config);

    // Run the download service in the background
    let service_handle = tokio::spawn(async move {
        service.run().await;
    });

    // Monitor download progress
    loop {
        let status = operation.status();
        match status {
            DownloadStatus::Download => {
                let progress = operation.progress() * 100.0;
                let downloaded = operation.downloaded_size();
                let total = operation.total_size();
                println!(
                    "Downloading: {:.1}% ({} / {} bytes)",
                    progress, downloaded, total
                );
            }
            DownloadStatus::DownloadPost => {
                println!("Merging chunks...");
            }
            DownloadStatus::FileVerify => {
                println!("Verifying file...");
            }
            DownloadStatus::Complete => {
                println!("Download complete!");
                break;
            }
            DownloadStatus::Failed => {
                let error = operation.error();
                eprintln!("Download failed: {}", error);
                break;
            }
            _ => {}
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Stop the service
    service_handle.abort();
}
