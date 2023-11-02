use crate::download_handle_file::DownloadHandleFile;
use crate::download_handle_memory::DownloadHandleMemory;

pub enum DownloadHandle {
    File(DownloadHandleFile),
    Memory(DownloadHandleMemory),
}

#[async_trait::async_trait]
pub trait DownloadHandleTrait {
    async fn setup(&mut self) -> crate::error::Result<()>;
    async fn received_bytes_async(&mut self, position: u64, buffer: &Vec<u8>) -> crate::error::Result<()>;
    fn get_downloaded_size(&self) -> u64;
    fn update_downloaded_size(&mut self, length: u64);
}