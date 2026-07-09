use std::fmt;

#[derive(Debug)]
pub enum IngestError {
    KbStore(String),
    Config(String),
    Cron(String),
    Pipeline(String),
    Shutdown,
}

impl fmt::Display for IngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestError::KbStore(msg) => write!(f, "KB store error: {msg}"),
            IngestError::Config(msg) => write!(f, "configuration error: {msg}"),
            IngestError::Cron(msg) => write!(f, "cron parse error: {msg}"),
            IngestError::Pipeline(msg) => write!(f, "pipeline error: {msg}"),
            IngestError::Shutdown => write!(f, "shutdown requested"),
        }
    }
}

impl std::error::Error for IngestError {}
