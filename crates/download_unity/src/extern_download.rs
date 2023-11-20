use std::ffi::{c_char, CStr, CString};
use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_operation::DownloadOperation;
use downloader_rs::download_service::DownloadService;

#[repr(C)]
pub struct DownloadConfig {
    url: *const c_char,
    path: *const c_char,
    retry_times: u8,
    chunk_download: bool,
    version: i64,
    chunk_size: u64,
}

#[no_mangle]
pub extern "C" fn start_download_service(multi_thread: bool, thread_count: u16, parallel_count: u16) -> *mut DownloadService {
    let mut download_service = DownloadService::new()
        .set_worker_thread_count(thread_count as usize)
        .set_multi_thread(multi_thread);
    download_service.set_parallel_count(parallel_count as usize);
    download_service.start_service();
    Box::into_raw(Box::new(download_service))
}

#[no_mangle]
pub extern "C" fn set_parallel_count(ptr: *mut DownloadService, count: u16) {
    let service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    service.set_parallel_count(count as usize)
}

#[no_mangle]
pub extern "C" fn stop_download_service(ptr: *mut DownloadService) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let download_service = Box::from_raw(ptr);
        download_service.stop();
    }
}

#[no_mangle]
pub extern "C" fn add_downloader(ptr: *mut DownloadService, config: DownloadConfig) -> *mut DownloadOperation {
    let download_service = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    let url = unsafe { CStr::from_ptr(config.url).to_string_lossy().to_string() };
    let path = unsafe { CStr::from_ptr(config.path).to_string_lossy().to_string() };
    let config = DownloadConfiguration::new()
        .set_url(&url)
        .set_file_path(&path)
        .set_chunk_download(config.chunk_download)
        .set_chunk_size(config.chunk_size)
        .set_remote_version(config.version)
        .set_retry_times_on_failure(config.retry_times)
        .build();
    let operation = download_service.add_downloader(config);
    Box::into_raw(Box::new(operation))
}

#[no_mangle]
pub extern "C" fn get_download_status(ptr: *mut DownloadOperation) -> u8 {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.status().into()
}

#[no_mangle]
pub extern "C" fn get_download_is_done(ptr: *mut DownloadOperation) -> bool {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.is_done()
}

#[no_mangle]
pub extern "C" fn get_download_is_error(ptr: *mut DownloadOperation) -> bool {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.is_error()
}

#[no_mangle]
pub extern "C" fn get_download_error(ptr: *mut DownloadOperation) -> *mut c_char {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    let ptr = CString::new(operation.error().to_string()).unwrap().into_raw();
    ptr
}

#[no_mangle]
pub extern "C" fn free_download_error(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[no_mangle]
pub extern "C" fn get_downloaded_size(ptr: *mut DownloadOperation) -> u64 {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.downloaded_size()
}

#[no_mangle]
pub extern "C" fn get_download_progress(ptr: *mut DownloadOperation) -> f64 {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.progress()
}

#[no_mangle]
pub extern "C" fn stop_downloader(ptr: *mut DownloadOperation) {
    let operation = unsafe { ptr.as_mut().expect("invalid ptr: ") };
    operation.stop()
}

#[no_mangle]
pub extern "C" fn downloader_dispose(ptr: *mut DownloadOperation) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr);
    }
}
