//! Multiple concurrent downloads example.
//!
//! Demonstrates using DownloadService to manage multiple downloads with parallel limits.
//!
//! Usage: cargo run --example multi_download

use std::time::Duration;
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_service::DownloadService;

#[tokio::main]
async fn main() {
    let mut service = DownloadService::new();
    service.set_parallel_count(2); // max 2 concurrent downloads

    let urls = vec![
        ("https://www.rust-lang.org/static/images/rust-logo-blk.svg", "./downloads/rust-logo.svg"),
        ("https://www.rust-lang.org/static/images/favicon-32x32.png", "./downloads/favicon.png"),
        ("https://www.rust-lang.org/static/images/rust-social-wide.jpg", "./downloads/rust-social.jpg"),
    ];

    let mut operations = Vec::new();
    for (url, path) in &urls {
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path(*path)
            .set_timeout(10)
            .set_retry_times_on_failure(3)
            .build()
            .expect("Invalid download configuration");

        let op = service.add_downloader(config);
        operations.push((path, op));
    }

    // Run the download service in the background
    let service_handle = tokio::spawn(async move {
        service.run().await;
    });

    // Monitor all downloads
    loop {
        let mut all_done = true;

        for (path, op) in &operations {
            if !op.is_done() {
                all_done = false;
                let total = op.total_size();
                if total > 0 {
                    println!("{}: {:.1}% ({}/{})", path, op.progress() * 100.0, op.downloaded_size(), total);
                } else {
                    println!("{}: waiting...", path);
                }
            }
        }

        if all_done {
            println!("\nAll downloads complete!");
            for (path, op) in &operations {
                if op.is_error() {
                    eprintln!("  {} — FAILED: {}", path, op.error());
                } else {
                    println!("  {} — OK ({} bytes)", path, op.total_size());
                }
            }
            break;
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    service_handle.abort();
}
