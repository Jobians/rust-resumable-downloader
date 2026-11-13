use std::fmt;
use std::io;

use reqwest::StatusCode;

#[derive(Debug)]
pub enum DownloadError {
    Cancelled,
    Http(StatusCode),
    Request(reqwest::Error),
    Io(io::Error),
    InvalidResponse(String),
    Incomplete { downloaded: u64, total: u64 },
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "download cancelled"),
            Self::Http(status) => write!(f, "server returned HTTP {status}"),
            Self::Request(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "{err}"),
            Self::InvalidResponse(message) => write!(f, "invalid server response: {message}"),
            Self::Incomplete { downloaded, total } => {
                write!(f, "incomplete download: got {downloaded} of {total} bytes")
            }
        }
    }
}

impl std::error::Error for DownloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(err) => Some(err),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::Request(value)
    }
}

impl From<io::Error> for DownloadError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
