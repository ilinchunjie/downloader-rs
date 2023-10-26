use std::ffi::{c_char, CStr};
use crate::download_configuration::DownloadConfiguration;
use crate::download_service::DownloadService;
use crate::downloader::Downloader;

#[repr(C)]
pub struct DownloadConfig {
    url: *const c_char,
    path: *const c_char,
}

#[no_mangle]
pub extern "C" fn get_download_service() -> *mut DownloadService {
    let mut download_service = DownloadService::new();
    download_service.start_service();
    Box::into_raw(Box::new(download_service))
}

#[no_mangle]
pub extern "C" fn download_service_dispose(ptr: *mut Downloader) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let mut download_service = Box::from_raw(ptr);
        download_service.stop();
    }
}

#[no_mangle]
pub extern "C" fn add_downloader(ptr: *mut DownloadService, config: DownloadConfig) -> u64 {
    let download_service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    let url = unsafe { CStr::from_ptr(config.url).to_string_lossy().to_string() };
    let path = unsafe { CStr::from_ptr(config.path).to_string_lossy().to_string() };
    let config = DownloadConfiguration::from_url_path(url, path);
    let downloader = Downloader::new(config);
    download_service.add_downloader(downloader)
}

#[no_mangle]
pub extern "C" fn get_download_status(ptr: *mut DownloadService, id: u64) -> i32 {
    let download_service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    download_service.get_download_status(id)
}

#[no_mangle]
pub extern "C" fn get_downloaded_size(ptr: *mut DownloadService, id: u64) -> u64 {
    let download_service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    download_service.get_downloaded_size(id)
}

#[no_mangle]
pub extern "C" fn remove_downloader(ptr: *mut DownloadService, id: u64) {
    let download_service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    download_service.remove_downloader(id)
}