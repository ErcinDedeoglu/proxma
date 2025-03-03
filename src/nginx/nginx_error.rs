use std::fmt;

#[derive(Debug)]
pub enum NginxError {
    Io(std::io::Error),
    ReloadFailed(String),
    InvalidRule(String),
}

impl fmt::Display for NginxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NginxError::Io(e) => write!(f, "IO error: {}", e),
            NginxError::ReloadFailed(s) => write!(f, "Nginx reload failed: {}", s),
            NginxError::InvalidRule(s) => write!(f, "Invalid proxy rule: {}", s),
        }
    }
}

impl std::error::Error for NginxError {}

impl From<std::io::Error> for NginxError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}