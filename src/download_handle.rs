use crate::download_handle_file::DownloadHandleFile;
use crate::download_handle_memory::DownloadHandleMemory;

pub enum DownloadHandle {
    DownloadHandleFile(DownloadHandleFile),
    DownloadHandleMemory(DownloadHandleMemory),
}