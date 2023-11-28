use std::fmt::{Display, Formatter};

#[derive(PartialEq, Clone, Copy)]
pub enum DownloadFile {
    File,
    Patch,
}

impl Display for DownloadFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadFile::File => write!(f, "File"),
            DownloadFile::Patch => write!(f, "Patch"),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum DownloadStatus {
    None,
    Pending,
    Head,
    Download(DownloadFile),
    DownloadPost,
    FileVerify,
    Complete,
    Failed,
    Stop,
}

impl Display for DownloadStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadStatus::None => write!(f, "None"),
            DownloadStatus::Pending => write!(f, "Pending"),
            DownloadStatus::Head => write!(f, "Head"),
            DownloadStatus::Download(file) => write!(f, "Download {}", file),
            DownloadStatus::FileVerify => write!(f, "FileVerify"),
            DownloadStatus::DownloadPost => write!(f, "DownloadPost"),
            DownloadStatus::Complete => write!(f, "Complete"),
            DownloadStatus::Failed => write!(f, "Failed"),
            DownloadStatus::Stop => write!(f, "Stop"),
        }
    }
}

impl Into<u8> for DownloadStatus {
    fn into(self) -> u8 {
        match self {
            DownloadStatus::None => 0,
            DownloadStatus::Pending => 1,
            DownloadStatus::Head => 2,
            DownloadStatus::Download(file) => {
                match file {
                    DownloadFile::File => 3,
                    DownloadFile::Patch => 4
                }
            },
            DownloadStatus::DownloadPost => 5,
            DownloadStatus::FileVerify => 6,
            DownloadStatus::Complete => 7,
            DownloadStatus::Failed => 8,
            DownloadStatus::Stop => 9
        }
    }
}

impl From<u8> for DownloadStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => DownloadStatus::None,
            1 => DownloadStatus::Pending,
            2 => DownloadStatus::Head,
            3 => DownloadStatus::Download(DownloadFile::File),
            4 => DownloadStatus::Download(DownloadFile::Patch),
            5 => DownloadStatus::DownloadPost,
            6 => DownloadStatus::FileVerify,
            7 => DownloadStatus::Complete,
            8 => DownloadStatus::Failed,
            9 => DownloadStatus::Stop,
            _ => DownloadStatus::None,
        }
    }
}