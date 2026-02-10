use std::path::{Path, PathBuf};
use crate::verify::file_verify::FileVerify;
use crate::error::DownloadError;

/// Configuration for a single download operation.
///
/// Use the builder pattern via [`DownloadConfiguration::new()`] to construct.
pub struct DownloadConfiguration {
    pub url: Option<String>,
    pub temp_path: Option<PathBuf>,
    pub path: Option<PathBuf>,
    pub chunk_size: u64,
    pub total_length: u64,
    pub remote_version: i64,
    pub retry_times_on_failure: u8,
    pub receive_bytes_per_second: u64,
    pub timeout: u64,
    pub range_download: bool,
    pub chunk_download: bool,
    pub download_in_memory: bool,
    pub file_verify: FileVerify,
}

/// Builder for [`DownloadConfiguration`].
pub struct DownloadConfigurationBuilder {
    config: DownloadConfiguration,
}

impl DownloadConfigurationBuilder {
    fn new(config: DownloadConfiguration) -> Self {
        Self {
            config
        }
    }

    /// Set the download URL.
    pub fn set_url(mut self, url: &str) -> DownloadConfigurationBuilder {
        self.config.url = Some(url.to_string());
        self
    }

    /// Set the local file path for the download.
    /// A `.temp` suffix path will be automatically generated.
    pub fn set_file_path(mut self, path: impl AsRef<Path>) -> DownloadConfigurationBuilder {
        let p = path.as_ref().to_path_buf();
        let temp = PathBuf::from(format!("{}.temp", p.display()));
        self.config.path = Some(p);
        self.config.temp_path = Some(temp);
        self
    }

    /// Set the remote version for cache validation.
    pub fn set_remote_version(mut self, version: i64) -> DownloadConfigurationBuilder {
        self.config.remote_version = version;
        self
    }

    /// Enable or disable range-based (partial) downloads.
    pub fn set_range_download(mut self, range_download: bool) -> DownloadConfigurationBuilder {
        self.config.range_download = range_download;
        self
    }

    /// Enable or disable chunked downloads (splitting files into multiple pieces).
    pub fn set_chunk_download(mut self, chunk_download: bool) -> DownloadConfigurationBuilder {
        self.config.chunk_download = chunk_download;
        self
    }

    /// Set the size of each download chunk in bytes.
    pub fn set_chunk_size(mut self, chunk_size: u64) -> DownloadConfigurationBuilder {
        self.config.chunk_size = chunk_size;
        self
    }

    /// Set the maximum number of retry attempts on failure.
    pub fn set_retry_times_on_failure(mut self, retry_times: u8) -> DownloadConfigurationBuilder {
        self.config.retry_times_on_failure = retry_times;
        self
    }

    /// Set the request timeout in seconds. 0 means no timeout.
    pub fn set_timeout(mut self, timeout: u64) -> DownloadConfigurationBuilder {
        self.config.timeout = timeout;
        self
    }

    /// Set the global download speed limit in bytes per second. 0 means unlimited.
    pub fn set_download_speed_limit(mut self, receive_bytes_per_second: u64) -> DownloadConfigurationBuilder {
        self.config.receive_bytes_per_second = receive_bytes_per_second;
        self
    }

    /// Enable or disable in-memory downloads (result accessible via `DownloadOperation::bytes()`).
    pub fn set_download_in_memory(mut self, download_in_memory: bool) -> DownloadConfigurationBuilder {
        self.config.download_in_memory = download_in_memory;
        self
    }

    /// Set the file verification method (e.g., hash check).
    pub fn set_file_verify(mut self, file_verify: FileVerify) -> DownloadConfigurationBuilder {
        self.config.file_verify = file_verify;
        self
    }

    /// Build the final [`DownloadConfiguration`], validating all required fields.
    pub fn build(self) -> crate::error::Result<DownloadConfiguration> {
        self.validate()
    }

    fn validate(self) -> crate::error::Result<DownloadConfiguration> {
        if self.config.url.is_none() {
            return Err(DownloadError::Config("Download address not configured.".to_string()));
        }

        if !self.config.download_in_memory {
            if self.config.path.is_none() {
                return Err(DownloadError::Config("No download path specified.".to_string()));
            }
        }

        Ok(self.config)
    }
}

impl DownloadConfiguration {
    /// Create a new [`DownloadConfigurationBuilder`].
    pub fn new() -> DownloadConfigurationBuilder {
        let config = DownloadConfiguration {
            url: None,
            path: None,
            temp_path: None,
            file_verify: FileVerify::None,
            range_download: true,
            chunk_download: false,
            chunk_size: 1024 * 1024 * 5,
            total_length: 0,
            remote_version: 0,
            retry_times_on_failure: 0,
            receive_bytes_per_second: 0,
            download_in_memory: false,
            timeout: 0,
        };
        DownloadConfigurationBuilder::new(config)
    }

    /// Get the destination file path.
    pub fn get_file_path(&self) -> &Path {
        return self.path.as_ref().unwrap().as_path();
    }

    /// Get the temporary file path used during download.
    pub fn get_file_temp_path(&self) -> &Path {
        return self.temp_path.as_ref().unwrap().as_path();
    }

    /// Get the download URL.
    pub fn url(&self) -> &str { return self.url.as_ref().unwrap().as_str(); }
}