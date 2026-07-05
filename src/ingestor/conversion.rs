use std::time::SystemTime;

use crate::ingestor::errors::APRSPacketConversionError;
use crate::ingestor::task::AprsPacket;
use crate::pb::ingestor::PbAprsPacket;

impl TryFrom<PbAprsPacket> for AprsPacket {
    type Error = APRSPacketConversionError;
    fn try_from(pb_packet: PbAprsPacket) -> Result<Self, Self::Error> {
        let pb_timestamp = pb_packet
            .timestamp
            .ok_or(APRSPacketConversionError::MissingTimestamp)?;

        let sys_time: SystemTime = pb_timestamp.try_into()?;
        let timestamp = sys_time.into();

        Ok(Self {
            timestamp,
            message: pb_packet.message,
        })
    }
}

impl From<AprsPacket> for PbAprsPacket {
    fn from(packet: AprsPacket) -> Self {
        let sys_time: SystemTime = packet.timestamp.into();

        Self {
            timestamp: Some(sys_time.into()),
            message: packet.message,
        }
    }
}
