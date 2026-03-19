pub mod config;
pub mod detail;
pub mod error;

pub use config::IngestorConfig;
pub use detail::pb::PbAprsPacket;
pub use detail::{Ingestor, write_pb_aprs_packet_to_disk};
