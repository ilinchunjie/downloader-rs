use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct DownloadConfiguration {
    pub url: Option<Arc<String>>,
    pub path: Option<Arc<String>>,
    pub chunk_size: u64,
    pub total_length: u64,
    pub remote_version: i64,
    pub remote_file_hash: u64,
    pub retry_times_on_failure: u8,
    pub download_in_memory: bool,
    pub support_range_download: bool,
    pub set_file_length: bool,
    pub chunk_download: bool,
    pub create_temp_file: bool,
}

pub struct DownloadConfigurationBuilder {
    config: DownloadConfiguration,
}

impl DownloadConfigurationBuilder {
    fn new(config: DownloadConfiguration) -> Self {
        Self {
            config
        }
    }

    pub fn set_url(mut self, url: String) -> DownloadConfigurationBuilder {
        self.config.url = Some(Arc::new(url));
        self
    }

    pub fn set_file_path(mut self, path: String) -> DownloadConfigurationBuilder {
        self.config.path = Some(Arc::new(path));
        self
    }

    pub fn set_download_in_memory(mut self) -> DownloadConfigurationBuilder {
        self.config.download_in_memory = true;
        self
    }

    pub fn set_chunk_download(mut self, chunk_download: bool) -> DownloadConfigurationBuilder {
        self.config.chunk_download = chunk_download;
        self
    }

    pub fn set_chunk_size(mut self, chunk_size: u64) -> DownloadConfigurationBuilder {
        self.config.chunk_size = chunk_size;
        self
    }

    pub fn set_file_length(mut self)  -> DownloadConfigurationBuilder {
        self.config.set_file_length = true;
        self
    }

    pub fn set_retry_times_on_failure(mut self, retry_times: u8) -> DownloadConfigurationBuilder {
        self.config.retry_times_on_failure = retry_times;
        self
    }

    pub fn create_temp_file(mut self, create_temp_file: bool) -> DownloadConfigurationBuilder {
        self.config.create_temp_file = create_temp_file;
        self
    }

    pub fn create_dir(mut self) -> DownloadConfigurationBuilder {
        let path = Path::new(self.config.path.as_ref().unwrap().deref());
        if let Some(directory) = path.parent() {
            if !directory.exists() {
                let result = fs::create_dir_all(directory);
                if let Err(e) = result {
                    panic!("{}", e);
                }
            }
        }
        self
    }

    pub fn build(mut self) -> DownloadConfiguration {
        self.validate()
    }

    fn validate(mut self) -> DownloadConfiguration {
        if self.config.url == None {
            panic!("Download address not configured.");
        }

        if !self.config.download_in_memory && self.config.path == None {
            panic!("No download path specified.");
        }

        if self.config.chunk_download {
            self.config.set_file_length = true
        }

        self.config
    }
}

impl DownloadConfiguration {
    pub fn new() -> DownloadConfigurationBuilder {
        let config = DownloadConfiguration {
            url: None,
            path: None,
            support_range_download: false,
            chunk_download: false,
            chunk_size: 1024 * 1024 * 5,
            total_length: 0,
            remote_version: 0,
            remote_file_hash: 0,
            download_in_memory: false,
            set_file_length: false,
            retry_times_on_failure: 0,
            create_temp_file: true,
        };
        DownloadConfigurationBuilder::new(config)
    }
}