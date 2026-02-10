use thiserror::Error;

/// Errors that can occur during download operations.
#[derive(Debug, Clone, Error)]
pub enum DownloadError {
    #[error("no error")]
    None,
    #[error("failed to open file")]
    FileOpen,
    #[error("failed to seek in file")]
    FileSeek,
    #[error("failed to write to file")]
    FileWrite,
    #[error("failed to flush file")]
    FileFlush,
    #[error("failed to rename file: {0}")]
    FileRename(String),
    #[error("failed to delete file")]
    DeleteFile,
    #[error("failed to seek in memory buffer")]
    MemorySeek,
    #[error("failed to write to memory buffer")]
    MemoryWrite,
    #[error("failed to flush memory buffer")]
    MemoryFlush,
    #[error("HEAD request failed")]
    Head,
    #[error("HTTP request failed")]
    Request,
    #[error("HTTP response error from {0}: status {1}")]
    Response(String, u16),
    #[error("failed to read response chunk")]
    ResponseChunk,
    #[error("failed to open or create file")]
    OpenOrCreateFile,
    #[error("file verification failed")]
    FileVerify,
    #[error("download task failed")]
    DownloadTask,
    #[error("patch operation failed")]
    Patch,
    #[error("chunk download handle error")]
    ChunkDownloadHandle,
    #[error("configuration error: {0}")]
    Config(String),
}

pub type Result<T> = core::result::Result<T, DownloadError>;