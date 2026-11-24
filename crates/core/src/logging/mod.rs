mod centralized_logger;
pub mod errors;
mod logger_registry;
mod traits;

#[cfg(test)]
mod integration_tests;

pub use crate::common::{JsonFormat, ProtoFormat};
pub use crate::logging::centralized_logger::CentralizedLogger;
pub use crate::logging::logger_registry::{
    JsonlLoggerHandle, LoggerID, LoggerMessage, LoggerRegistry, ProtoLoggerHandle,
};
pub use crate::logging::traits::{
    EncodableMessage, IntoLogMessage, LogSender, ProtoFileDescriptor,
};
