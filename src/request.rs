use reqwest::header::RANGE;
use reqwest::RequestBuilder;
use crate::download_configuration::DownloadConfiguration;

pub fn get_download_request(config: &DownloadConfiguration) -> RequestBuilder {
    let request = reqwest::Client::new().get(&config.url);
    let mut range_str = String::new();
    if config.range_download {
        if config.range_start < config.range_end {
            range_str = format!("bytes={}-{}", config.range_start, config.range_end);
        } else {
            range_str = format!("bytes={}-", config.range_start);
        }
    }
    request.header(RANGE, range_str)
}