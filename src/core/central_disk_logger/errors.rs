use std::io;
use std::path::PathBuf;

use crate::core::central_disk_logger::task::LoggerTaskID;

#[derive(Debug, thiserror::Error)]
pub enum DiskloggerRegistryError {
    #[error("path already registered: {0}")]
    PathAlreadyRegisteredError(PathBuf),

    #[error("Unable to create logger file at path: {path}")]
    LogFileCreationError {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CentralDiskLoggerError {
    #[error("TaskID not registered: {0}")]
    TaskNotRegistered(LoggerTaskID),

    #[error("Unable to write data to log file: {path}")]
    WriteError {
        path: PathBuf,
        payload: Vec<u8>,
        #[source]
        source: io::Error,
    },
}
