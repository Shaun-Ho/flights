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

pub fn write_pb_aprs_packet_to_disk(
    writer: &mut std::io::BufWriter<std::fs::File>,
    aprs_packet: &PbAprsPacket,
) -> Result<(), std::io::Error> {
    let mut buf = Vec::new();
    let () = aprs_packet.encode_length_delimited(&mut buf)?;
    writer.write_all(&buf)
}
