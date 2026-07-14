use std::path::PathBuf;
use std::{convert::Infallible, io};

use crate::core::central_disk_logger::interface::{DiskLoggerMessage, LoggerID};

#[derive(Debug, thiserror::Error)]
pub enum ProtoLoggingError<T> {
    #[error("Packet Conversion Error: {0}")]
    Conversion(T),
    #[error("Failed to send: {0}")]
    SendError(#[from] crossbeam_channel::SendError<DiskLoggerMessage>),
}
impl<T> From<Infallible> for ProtoLoggingError<T> {
    fn from(err: Infallible) -> Self {
        match err {}
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JsonLoggingError<T> {
    #[error("Packet Conversion Error: {0}")]
    Conversion(T),
    #[error("Failed to send: {0}")]
    SendError(#[from] crossbeam_channel::SendError<DiskLoggerMessage>),
    #[error("Failed to send: {0}")]
    Serialization(#[from] serde_json::Error),
}
impl<T> From<std::convert::Infallible> for JsonLoggingError<T> {
    fn from(err: std::convert::Infallible) -> Self {
        match err {}
    }
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
