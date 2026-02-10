use std::fmt::{Display, Formatter};

#[derive(PartialEq, Clone, Copy)]
pub enum DownloadStatus {
    None,
    Pending,
    Head,
    Download,
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
            DownloadStatus::Download => write!(f, "Download"),
            DownloadStatus::FileVerify => write!(f, "FileVerify"),
            DownloadStatus::DownloadPost => write!(f, "DownloadPost"),
            DownloadStatus::Complete => write!(f, "Complete"),
            DownloadStatus::Failed => write!(f, "Failed"),
            DownloadStatus::Stop => write!(f, "Stop"),
        }
    }
}

impl From<DownloadStatus> for u8 {
    fn from(status: DownloadStatus) -> u8 {
        match status {
            DownloadStatus::None => 0,
            DownloadStatus::Pending => 1,
            DownloadStatus::Head => 2,
            DownloadStatus::Download => 3,
            DownloadStatus::DownloadPost => 4,
            DownloadStatus::FileVerify => 5,
            DownloadStatus::Complete => 6,
            DownloadStatus::Failed => 7,
            DownloadStatus::Stop => 8
        }
    }
}

impl From<u8> for DownloadStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => DownloadStatus::None,
            1 => DownloadStatus::Pending,
            2 => DownloadStatus::Head,
            3 => DownloadStatus::Download,
            4 => DownloadStatus::DownloadPost,
            5 => DownloadStatus::FileVerify,
            6 => DownloadStatus::Complete,
            7 => DownloadStatus::Failed,
            8 => DownloadStatus::Stop,
            _ => DownloadStatus::None,
        }
    }
}