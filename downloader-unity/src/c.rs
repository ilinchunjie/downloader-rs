use std::ffi::{c_char, CStr};
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_operation::DownloadOperation;
use downloader_rs::download_service::DownloadService;
use downloader_rs::downloader::Downloader;

#[repr(C)]
pub struct DownloadConfig {
    url: *const c_char,
    path: *const c_char,
    chunk_download: bool,
    chunk_siez: u64,
}

#[no_mangle]
pub extern "C" fn start_download_service() -> *mut DownloadService {
    let mut download_service = DownloadService::new();
    download_service.start_service();
    Box::into_raw(Box::new(download_service))
}

#[no_mangle]
pub extern "C" fn stop_download_service(ptr: *mut DownloadService) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let mut download_service = Box::from_raw(ptr);
        download_service.stop();
    }
}

#[no_mangle]
pub extern "C" fn add_downloader(ptr: *mut DownloadService, config: DownloadConfig) -> *mut DownloadOperation {
    let download_service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    let url = unsafe { CStr::from_ptr(config.url).to_string_lossy().to_string() };
    let path = unsafe { CStr::from_ptr(config.path).to_string_lossy().to_string() };
    let config = DownloadConfiguration::new()
        .set_url(url)
        .set_file_path(path)
        .set_chunk_download(config.chunk_download)
        .set_chunk_size(config.chunk_siez)
        .build();
    let operation = download_service.add_downloader(config);
    Box::into_raw(Box::new(operation))
}

#[no_mangle]
pub extern "C" fn get_download_status(ptr: *mut DownloadOperation) -> i32 {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.status()
}

#[no_mangle]
pub extern "C" fn get_downloaded_size(ptr: *mut DownloadOperation) -> u64 {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.downloaded_size()
}

#[no_mangle]
pub extern "C" fn stop_downloader(ptr: *mut DownloadOperation) {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.stop();
}
