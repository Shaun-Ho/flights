use std::time::SystemTime;

use flights_core::logging::{IntoLogMessage, ProtoFormat};
use flights_core::reader::ReplayableMessage;
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta};
    use flights_core::logging::{LogSender, LoggerRegistry};
    use flights_core::reader::DecodeMessage;
    use flights_core::thread_manager::{SteppableTask, TaskState};

    use super::*;

    fn packets() -> Vec<AprsPacket> {
        let start = DateTime::from_timestamp(1_760_000_000, 0).unwrap();
        (0..3)
            .map(|i| AprsPacket {
                timestamp: start + TimeDelta::milliseconds(i * 10),
                message: format!("packet {i}\n").into(),
            })
            .collect()
    }

    #[test]
    fn given_packets_logged_by_registry_when_decoded_and_converted_then_identical_packets_in_order()
    {
        let path = testdir::testdir!().join("ingestor.mcap");
        let packets = packets();
        let mut registry = LoggerRegistry::new();
        let handle = registry
            .register_proto::<PbAprsPacket>(path.clone(), "ingestor".to_owned())
            .unwrap();
        let mut logger = registry.build();
        for packet in &packets {
            handle.send(packet.clone()).unwrap();
            assert!(matches!(logger.step(), TaskState::Running));
        }
        drop(logger);

        let bytes = std::fs::read(&path).unwrap();
        let replayed: Vec<AprsPacket> = mcap::MessageStream::new(&bytes)
            .unwrap()
            .map(|message| {
                let stored: PbAprsPacket = ProtoFormat::decode(&message.unwrap().data).unwrap();
                AprsPacket::from_stored(stored).unwrap()
            })
            .collect();

        assert_eq!(replayed, packets);
    }

    #[test]
    fn given_pb_packet_without_timestamp_when_converted_then_conversion_error() {
        let stored = PbAprsPacket {
            timestamp: None,
            message: "no timestamp\n".into(),
        };

        assert!(matches!(
            AprsPacket::from_stored(stored),
            Err(APRSPacketConversionError::MissingTimestamp)
        ));
    }
}
