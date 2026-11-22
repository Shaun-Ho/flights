use std::time::SystemTime;

use flights_core::logging::{IntoLogMessage, ProtoFormat};
use flights_core::reader::traits::ReplayableMessage;
use flights_proto::ingestor::PbAprsPacket;

use crate::errors::APRSPacketConversionError;
use crate::task::AprsPacket;

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

impl ReplayableMessage<ProtoFormat> for AprsPacket {
    type Message = PbAprsPacket;
    type Error = APRSPacketConversionError;

    fn from_stored(stored: PbAprsPacket) -> Result<Self, Self::Error> {
        stored.try_into()
    }
}
