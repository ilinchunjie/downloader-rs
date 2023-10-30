use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct DownloadConfiguration {
    pub url: Option<Arc<String>>,
    pub path: Option<Arc<String>>,
    pub support_range_download: bool,
    pub chunk_download: bool,
    pub chunk_size: u64,
    pub total_length: u64,
    pub remote_version: i64,
    pub remote_file_hash: u64,
    pub download_in_memory: bool,
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

    pub fn create_dir(mut self, create: bool) -> DownloadConfigurationBuilder {
        let path = Path::new(self.config.path.as_ref().unwrap().deref());
        if let Some(directory) = path.parent() {
            if !directory.exists() {
                let result = fs::create_dir(directory);
                if let Err(e) = result {
                    panic!("{}", e);
                }
            }
        }
        self
    }

    pub fn build(self) -> DownloadConfiguration {
        self.validate()
    }

    fn validate(self) -> DownloadConfiguration {
        if self.config.url == None {
            panic!("未设置下载地址");
        }

        if !self.config.download_in_memory && self.config.path == None {
            panic!("未设置下载路径");
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
        };
        DownloadConfigurationBuilder::new(config)
    }
}