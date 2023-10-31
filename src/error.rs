use std::fmt::{Display, Formatter, write};

pub enum DownloadError {
    Seek,
    Write,
    FlushToDisk,
    RequestFail,
    ResponseFail,
    FailToReadChunk,
    OpenOrCreateFile,
    CreateMetaFile(String),
}

pub type Result<T> = core::result::Result<T, DownloadError>;

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Seek => { write!(f, "") }
            DownloadError::Write => { write!(f, "") }
            DownloadError::FlushToDisk => { write!(f, "") }
            DownloadError::RequestFail => { write!(f, "") }
            DownloadError::ResponseFail => { write!(f, "") }
            DownloadError::FailToReadChunk => { write!(f, "") }
            DownloadError::OpenOrCreateFile => { write!(f, "") }
            DownloadError::CreateMetaFile(message) => {
                write!(f, "{}", message)
            }
        }
    }
}