pub mod config;
pub mod detail;
pub mod error;
pub mod protobuf;

pub use config::IngestorConfig;
pub use detail::{Ingestor, write_pb_aprs_packet_to_disk};
pub use protobuf::PbAprsPacket;
