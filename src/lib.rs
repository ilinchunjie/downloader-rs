//! # downloader-rs
//!
//! A high-performance, async file downloader library for Rust.
//!
//! Features:
//! - Chunked & range-based downloads
//! - Global rate limiting (token-bucket)
//! - In-memory download mode
//! - File verification (xxHash)
//! - Parallel download service with configurable concurrency

mod download_task;
mod stream;
mod remote_file;
mod chunk;
mod chunk_metadata;
mod chunk_hub;
mod chunk_range;
mod download_tracker;
mod download_sender;
mod download_receiver;
pub mod verify;
pub mod error;
pub mod rate_limiter;
pub mod download_status;
pub mod download_configuration;
pub mod download_service;
pub mod downloader;
pub mod download_operation;