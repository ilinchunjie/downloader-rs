#[derive(Clone)]
pub struct DownloadConfiguration {
    pub path : String,
    pub temp_path : String,
    pub url : String,
    pub range_download : bool,
    pub range_start : i64,
    pub range_end : i64
}

impl DownloadConfiguration {
    pub fn new(url : String, path : String) -> DownloadConfiguration {
        let temp_path = path.to_string() + ".temp";
        DownloadConfiguration {
            path,
            temp_path,
            url,
            range_download : false,
            range_start : 0,
            range_end : 0
        }
    }
}