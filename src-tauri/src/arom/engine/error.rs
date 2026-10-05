use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug)]
pub enum EngineError {
    Message(String),
    Io {
        op: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Image {
        op: &'static str,
        path: PathBuf,
        source: image::ImageError,
    },
    Task {
        task: String,
        reason: String,
    },
    Tier {
        tier: &'static str,
        failures: Vec<String>,
    },
    PathNotFound {
        source: u32,
        target: u32,
    },
    LockPoisoned(&'static str),
    ChannelClosed(&'static str),
}

impl EngineError {
    pub fn io(op: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io {
            op,
            path: path.to_path_buf(),
            source,
        }
    }

    pub fn image(op: &'static str, path: &Path, source: image::ImageError) -> Self {
        Self::Image {
            op,
            path: path.to_path_buf(),
            source,
        }
    }
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{msg}"),
            Self::Io { op, path, source } => {
                write!(f, "I/O error during {op} at {}: {source}", path.display())
            }
            Self::Image { op, path, source } => {
                write!(f, "Image error during {op} at {}: {source}", path.display())
            }
            Self::Task { task, reason } => write!(f, "Task `{task}` failed: {reason}"),
            Self::Tier { tier, failures } => {
                write!(f, "Tier `{tier}` failed ({} errors): {}", failures.len(), failures.join(" | "))
            }
            Self::PathNotFound { source, target } => {
                write!(f, "Unable to find conversion path from {source} to {target}")
            }
            Self::LockPoisoned(name) => write!(f, "Lock poisoned: {name}"),
            Self::ChannelClosed(name) => write!(f, "Channel closed: {name}"),
        }
    }
}

impl Error for EngineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Image { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<String> for EngineError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for EngineError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}