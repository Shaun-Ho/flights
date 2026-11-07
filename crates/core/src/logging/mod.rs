mod centralized_logger;
pub mod errors;
mod interface;
mod traits;

#[cfg(test)]
mod testing;

pub use centralized_logger::CentralizedLogger;
pub use interface::{
    DiskLoggerMessage, DiskLoggerRegistry, JsonlLoggerHandle, LoggerID, ProtoLoggerHandle,
};
pub use traits::{IntoLogMessage, LogSender, ProtoFileDescriptor};
