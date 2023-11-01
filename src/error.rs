use std::fmt::{Display, Formatter, write};

pub enum DownloadError {
    FileSetLength(String),
    FileSeek,
    FileWrite,
    FileFlush,
    MemorySeek,
    MemoryWrite,
    MemoryFlush,
    Request,
    Response,
    ResponseChunk,
    OpenOrCreateFile,
    CreateMetaFile(String),
}

pub type Result<T> = core::result::Result<T, DownloadError>;

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::FileSetLength(message) => {
                write!(f, "{}", message)
            }
            DownloadError::FileSeek => { write!(f, "") }
            DownloadError::FileWrite => { write!(f, "") }
            DownloadError::FileFlush => { write!(f, "") }
            DownloadError::MemorySeek => { write!(f, "") }
            DownloadError::MemoryWrite => { write!(f, "") }
            DownloadError::MemoryFlush => { write!(f, "") }
            DownloadError::Request => { write!(f, "") }
            DownloadError::Response => { write!(f, "") }
            DownloadError::ResponseChunk => { write!(f, "") }
            DownloadError::OpenOrCreateFile => { write!(f, "") }
            DownloadError::CreateMetaFile(message) => {
                write!(f, "{}", message)
            }
        }
    }
}