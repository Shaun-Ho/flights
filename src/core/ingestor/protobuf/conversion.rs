use std::io::Write;

use prost::Message;

use crate::core::ingestor::errors::PacketConversionError;
use crate::core::ingestor::protobuf::PbAprsPacket;
use crate::core::ingestor::task::AprsPacket;

impl TryFrom<PbAprsPacket> for AprsPacket {
    type Error = PacketConversionError;
    fn try_from(packet: PbAprsPacket) -> Result<Self, Self::Error> {
        let timestamp = packet
            .timestamp
            .ok_or(PacketConversionError::MissingTimestamp)?
            .try_into()?;

        Ok(Self {
            timestamp,
            message: packet.message,
        })
    }
}

impl From<AprsPacket> for PbAprsPacket {
    fn from(packet: AprsPacket) -> Self {
        let pb_timestamp = packet.timestamp.into();

        Self {
            timestamp: Some(pb_timestamp),
            message: packet.message,
        }
    }
}
