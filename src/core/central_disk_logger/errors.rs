use std::io;
use std::path::PathBuf;

use crate::core::central_disk_logger::task::LoggerTaskID;

#[derive(Debug, thiserror::Error)]
pub enum CentralisedLoggerError {
    #[error("TaskID not registered: {0}")]
    TaskNotRegistered(LoggerTaskID),
    #[error("Unable to create logger file at path: {path}")]
    LogFileCreationError {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Unable to write data to log file: {path}")]
    WriteError {
        path: PathBuf,
        payload: Vec<u8>,
        #[source]
        source: io::Error,
    },
}
