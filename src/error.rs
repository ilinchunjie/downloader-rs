use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum DownloadError {
    None,
    FileOpen,
    FileSeek,
    FileWrite,
    FileFlush,
    FileRename(String),
    DeleteFile,
    MemorySeek,
    MemoryWrite,
    MemoryFlush,
    Head,
    Request,
    Response(String, u16),
    ResponseChunk,
    OpenOrCreateFile,
    FileVerify,
    DownloadTask,
    Patch,
    ChunkDownloadHandle,
}

pub type Result<T> = core::result::Result<T, DownloadError>;

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::None => { write!(f, "None") }
            DownloadError::FileOpen => { write!(f, "FileOpen") }
            DownloadError::FileSeek => { write!(f, "FileSeek") }
            DownloadError::FileWrite => { write!(f, "FileWrite") }
            DownloadError::FileFlush => { write!(f, "FileFlush") }
            DownloadError::FileRename(message) => {
                write!(f, "file rename fail {}", message)
            }
            DownloadError::DeleteFile => { write!(f, "DeleteFile") }
            DownloadError::MemorySeek => { write!(f, "MemorySeek") }
            DownloadError::MemoryWrite => { write!(f, "MemoryWrite") }
            DownloadError::MemoryFlush => { write!(f, "MemoryFlush") }
            DownloadError::Head => { write!(f, "Head") }
            DownloadError::Request => { write!(f, "Request") }
            DownloadError::Response(url, status_code) => {
                write!(f, "{} response fail {}", url, status_code)
            }
            DownloadError::ResponseChunk => { write!(f, "ResponseChunk") }
            DownloadError::OpenOrCreateFile => { write!(f, "OpenOrCreateFile") }
            DownloadError::DownloadTask => { write!(f, "DownloadTask") }
            DownloadError::FileVerify => {
                write!(f, "FileVerify")
            }
            DownloadError::Patch => { write!(f, "Patch") }
            DownloadError::ChunkDownloadHandle => { write!(f, "ChunkDownloadHandle") }
        }
    }
}