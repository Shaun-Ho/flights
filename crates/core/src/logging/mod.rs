mod centralized_logger;
pub mod errors;
mod logger_registry;
mod traits;

#[cfg(test)]
mod testing;

pub use centralized_logger::CentralizedLogger;
pub use logger_registry::{
    JsonlLoggerHandle, LoggerID, LoggerMessage, LoggerRegistry, ProtoLoggerHandle,
};
pub use traits::{IntoLogMessage, LogSender, ProtoFileDescriptor};
