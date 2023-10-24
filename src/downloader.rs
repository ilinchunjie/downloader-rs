use std::rc::Rc;
use crate::remote_file::RemoteFile;

pub struct Downloader {
    remote_url: Rc<String>,
    remote_file: RemoteFile,
}

impl Downloader {
    pub fn new(remote_url: String, file_path: String) -> Downloader {
        let remote_url = Rc::new(remote_url);
        let remote_url_clone = remote_url.clone();
        let mut downloader = Downloader {
            remote_url,
            remote_file: RemoteFile::new(remote_url_clone),
        };
        downloader
    }

    pub fn start() {}

    pub fn stop() {}
}