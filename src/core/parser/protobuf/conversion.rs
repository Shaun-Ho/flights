use ogn_aprs_parser::aprs_types::ICAOAddress;
use ogn_aprs_parser::errors::ICAOAddressError;

use crate::core::parser::Aircraft;
use crate::core::parser::protobuf::PbAircraft;

#[derive(Debug, thiserror::Error)]
pub enum PacketConversionError {
    #[error("Missing timestamp field")]
    MissingTimestamp,
    #[error("Failed to parse valid Chrono DateTime from Timestamp")]
    InvalidTimestamp,
    #[error("Invalid IcaoAddress")]
    InvalidIcaoAddress(#[from] ICAOAddressError),
}

impl TryFrom<PbAircraft> for Aircraft {
    type Error = PacketConversionError;

    fn try_from(packet: PbAircraft) -> Result<Self, Self::Error> {
        let pb_ts = packet
            .timestamp
            .ok_or(PacketConversionError::MissingTimestamp)?;

        let timestamp = chrono::DateTime::from_timestamp(pb_ts.seconds, pb_ts.nanos as u32)
            .ok_or(PacketConversionError::InvalidTimestamp)?;

        let pb_rec_ts = packet
            .recorded_datetime
            .ok_or(PacketConversionError::MissingTimestamp)?;

        let recorded_datetime =
            chrono::DateTime::from_timestamp(pb_rec_ts.seconds, pb_rec_ts.nanos as u32)
                .ok_or(PacketConversionError::InvalidTimestamp)?;

        let icao_address = ICAOAddress::new(packet.icao_address)?;
        Ok(Self {
            timestamp: timestamp.into(),
            recorded_datetime,
            icao_address,
            callsign: packet.callsign,
            latitude: packet.latitude,
            longitude: packet.longitude,
            ground_track: packet.ground_track,
            ground_speed: packet.ground_speed,
            gps_altitude: packet.gps_altitude,
        })
    }
}

impl From<Aircraft> for PbAircraft {
    fn from(packet: Aircraft) -> Self {
        let pb_timestamp = packet.timestamp.into();

        let pb_recorded_datetime = prost_types::Timestamp {
            seconds: packet.recorded_datetime.timestamp(),
            nanos: packet.recorded_datetime.timestamp_subsec_nanos() as i32,
        };

        let icao_address = packet.icao_address.value();

        Self {
            timestamp: Some(pb_timestamp),
            recorded_datetime: Some(pb_recorded_datetime),
            icao_address,
            callsign: packet.callsign,
            latitude: packet.latitude,
            longitude: packet.longitude,
            ground_track: packet.ground_track,
            ground_speed: packet.ground_speed,
            gps_altitude: packet.gps_altitude,
        }
    }
}
