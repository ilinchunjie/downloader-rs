use std::sync::Arc;
use crate::file_verify::FileVerify;

pub struct DownloadConfiguration {
    pub url: Option<Arc<String>>,
    pub path: Option<Arc<String>>,
    pub chunk_size: u64,
    pub total_length: u64,
    pub remote_version: i64,
    pub retry_times_on_failure: u8,
    pub range_download: bool,
    pub chunk_download: bool,
    pub file_verify: FileVerify,
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

    pub fn set_remote_version(mut self, version: i64) -> DownloadConfigurationBuilder {
        self.config.remote_version = version;
        self
    }

    pub fn set_range_download(mut self, range_download: bool) -> DownloadConfigurationBuilder {
        self.config.range_download = range_download;
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

    pub fn set_retry_times_on_failure(mut self, retry_times: u8) -> DownloadConfigurationBuilder {
        self.config.retry_times_on_failure = retry_times;
        self
    }

    pub fn set_file_verify(mut self, file_verify: FileVerify) -> DownloadConfigurationBuilder {
        self.config.file_verify = file_verify;
        self
    }

    pub fn build(self) -> DownloadConfiguration {
        self.validate()
    }

    fn validate(self) -> DownloadConfiguration {
        if self.config.url == None {
            panic!("Download address not configured.");
        }

        if self.config.path == None {
            panic!("No download path specified.");
        }

        self.config
    }
}

impl DownloadConfiguration {
    pub fn new() -> DownloadConfigurationBuilder {
        let config = DownloadConfiguration {
            url: None,
            path: None,
            file_verify: FileVerify::None,
            range_download: true,
            chunk_download: false,
            chunk_size: 1024 * 1024 * 5,
            total_length: 0,
            remote_version: 0,
            retry_times_on_failure: 0,
        };
        DownloadConfigurationBuilder::new(config)
    }

    pub fn get_file_path(&self) -> &str {
        return self.path.as_ref().unwrap().as_str();
    }

    pub fn url(&self) -> &str { return self.url.as_ref().unwrap().as_str(); }
}