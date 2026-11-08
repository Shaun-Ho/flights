pub mod config;
pub mod conversion;
pub mod errors;
pub mod task;

pub use flights_proto::ingestor::PbAprsPacket;
pub use task::{AprsPacket, Ingestor};
