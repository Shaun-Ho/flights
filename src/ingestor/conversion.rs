use crate::ingestor::errors::APRSPacketConversionError;
use crate::ingestor::task::AprsPacket;
use crate::pb::ingestor::PbAprsPacket;

impl TryFrom<PbAprsPacket> for AprsPacket {
    type Error = APRSPacketConversionError;
    fn try_from(packet: PbAprsPacket) -> Result<Self, Self::Error> {
        let timestamp = packet
            .timestamp
            .ok_or(APRSPacketConversionError::MissingTimestamp)?
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
