use std::path::PathBuf;
use std::{convert::Infallible, io};

use crate::logging::interface::{DiskLoggerMessage, LoggerID};

#[derive(Debug, thiserror::Error)]
pub enum LoggingError<T> {
    #[error("Packet Conversion Error: {0}")]
    Conversion(T),
    #[error("Failed to encode: {0}")]
    Encode(#[from] EncodeError),
    #[error("Failed to send: {0}")]
    SendError(#[from] crossbeam_channel::SendError<DiskLoggerMessage>),
}
impl<T> From<Infallible> for LoggingError<T> {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DiskloggerRegistryError {
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Path already registered: {0}")]
    PathAlreadyRegisteredError(PathBuf),

    #[error("Unable to create logger file at path: {path}")]
    LogFileCreationError {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Unable to create Mcap writer: {0}")]
    WriterCreationError(#[from] mcap::McapError),
}

#[derive(Debug, thiserror::Error)]
pub enum CentralDiskLoggerError {
    #[error("TaskID not registered: {0}")]
    TaskNotRegistered(LoggerID),

    #[error("Unable to write data to log file: {0}")]
    WriteError(#[from] mcap::McapError),
}
