<div>
  <!-- Crates version -->
  <a href="https://crates.io/crates/downloader-rs">
    <img src="https://shields.io/crates/v/downloader-rs" alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/downloader-rs">
    <img src="https://shields.io/crates/d/downloader-rs" alt="Download" />
  </a>
  <!-- License -->
  <a href="https://github.com/ilinchunjie/downloader-rs/blob/main/LICENSE">
    <img src="https://shields.io/crates/l/downloader-rs" alt="LICENSE" />
  </a>
</div>

# downloader-rs

A high-performance, async file downloader library for Rust.

## Features

- ✅ Chunked / resumable / multi-task download
- ✅ Configurable parallel download limit
- ✅ Global token-bucket rate limiter
- ✅ In-memory download mode
- ✅ xxHash file verification
- ✅ Configurable retry on failure
- ✅ Structured logging via `tracing`
- ✅ Runs in caller's Tokio runtime (no self-built runtime)

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
downloader-rs = "0.6"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Basic Usage

```rust
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_service::DownloadService;

#[tokio::main]
async fn main() {
    let mut service = DownloadService::new();
    service.set_parallel_count(4);

    let config = DownloadConfiguration::new()
        .set_url("https://example.com/file.zip")
        .set_file_path("/tmp/file.zip")
        .set_range_download(true)
        .set_chunk_download(true)
        .set_download_speed_limit(1024 * 1024) // 1 MB/s
        .set_retry_times_on_failure(3)
        .build()
        .unwrap();

    let operation = service.add_downloader(config);

    // Run the service (non-blocking scheduling loop)
    tokio::spawn(async move { service.run().await });

    // Monitor progress
    loop {
        if operation.is_done() {
            println!("Download complete!");
            break;
        }
        println!("Progress: {:.1}%", operation.progress() * 100.0);
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}
```

### In-Memory Download

```rust
let config = DownloadConfiguration::new()
    .set_url("https://example.com/data.json")
    .set_download_in_memory(true)
    .build()
    .unwrap();

let operation = service.add_downloader(config);
// ... after download completes:
let data = operation.bytes();
```

### File Verification

```rust
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::verify::file_verify::FileVerify;

let config = DownloadConfiguration::new()
    .set_url("https://example.com/file.bin")
    .set_file_path("/tmp/file.bin")
    .set_file_verify(FileVerify::xxHash(0x123456789ABCDEF0))
    .build()
    .unwrap();
```

## Architecture

```
DownloadService          — Scheduling loop with configurable parallelism
  └─ Downloader          — Single download lifecycle (HEAD → download → verify → rename)
       └─ Chunk(s)       — Concurrent chunk tasks with shared AtomicU64 progress
            └─ RateLimiter — Global token-bucket bandwidth control
```

## API Overview

| Type                    | Description                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------- |
| `DownloadService`       | Manages concurrent downloads with configurable parallelism                            |
| `DownloadConfiguration` | Builder for download settings (URL, path, chunks, speed, etc.)                        |
| `DownloadOperation`     | Handle to monitor progress, status, errors, and retrieve results                      |
| `DownloadStatus`        | Enum: None, Pending, Head, Download, DownloadPost, FileVerify, Complete, Failed, Stop |
| `DownloadError`         | Error type with descriptive messages via `thiserror`                                  |
| `RateLimiter`           | Global token-bucket rate limiter shared across all chunks                             |

## License

MIT

## Examples

![][download_gif]

## Welcome to submit pull requests.

[download_gif]: https://github.com/ilinchunjie/downloader-rs/blob/main/res/download.gif
