#[derive(Clone)]
pub struct DownloadConfiguration {
    pub url: String,
    pub range_download: bool,
    pub range_start: u64,
    pub range_end: u64,
}

