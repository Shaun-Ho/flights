use std::time::SystemTime;

use crate::core::central_disk_logger::interface::IntoLogMessage;
use crate::ingestor::errors::APRSPacketConversionError;
use crate::ingestor::task::AprsPacket;
use crate::pb::ingestor::PbAprsPacket;

impl IntoLogMessage<PbAprsPacket> for AprsPacket {
    type Error = std::convert::Infallible;
    fn message_timestamp(&self) -> chrono::prelude::DateTime<chrono::prelude::Utc> {
        self.timestamp
    }
    fn into_message(self) -> Result<PbAprsPacket, Self::Error> {
        let sys_time: SystemTime = self.timestamp.into();

        Ok(PbAprsPacket {
            timestamp: Some(sys_time.into()),
            message: self.message,
        })
    }
}

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
