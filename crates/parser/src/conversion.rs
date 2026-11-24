use std::convert::Infallible;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use ogn_aprs_parser::{AircraftBeacon, ICAOAddress};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use flights_core::logging::{IntoLogMessage, JsonFormat, ProtoFormat};
use flights_core::reader::ReplayableMessage;
use flights_proto::parser::PbAircraft;

use crate::errors::AircraftConversionError;

#[derive(Debug, PartialEq, Clone)]
pub struct Aircraft {
    pub callsign: String,
    pub icao_address: ICAOAddress,
    pub broadcasted_timestamp: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}
impl IntoLogMessage<AircraftJson> for Aircraft {
    type Error = Infallible;
    fn into_message(self) -> Result<AircraftJson, Self::Error> {
        Ok(AircraftJson {
            callsign: self.callsign,
            icao_address: self.icao_address,
            broadcasted_timestamp: self.broadcasted_timestamp,
            latitude: self.latitude,
            longitude: self.longitude,
            gps_altitude: self.gps_altitude,
            ground_speed: self.ground_speed,
            ground_track: self.ground_track,
        })
    }
    fn message_timestamp(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct AircraftJson {
    pub callsign: String,
    #[serde(with = "icao_serde")]
    #[schemars(with = "u32")]
    pub icao_address: ICAOAddress,
    pub broadcasted_timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}

pub fn convert_ogn_aprs_beacon_to_aircraft(
    aircraft_beacon: AircraftBeacon,
    reference_packet_timestamp: DateTime<Utc>,
) -> Aircraft {
    let broadcasted_timestamp = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        reference_packet_timestamp
            .date_naive()
            .and_time(aircraft_beacon.time),
        chrono::Utc,
    );

    Aircraft {
        callsign: aircraft_beacon.callsign,
        icao_address: aircraft_beacon.ogn_beacon_id.icao_address,
        broadcasted_timestamp,
        latitude: aircraft_beacon.latitude,
        longitude: aircraft_beacon.longitude,
        ground_track: aircraft_beacon.ground_track,
        ground_speed: aircraft_beacon.ground_speed,
        gps_altitude: aircraft_beacon.gps_altitude,
    }
}

mod icao_serde {
    use super::ICAOAddress;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)] // signature required by `#[serde(with)]`
    pub fn serialize<S>(icao: &ICAOAddress, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw_value: u32 = icao.value();

        serializer.serialize_u32(raw_value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ICAOAddress, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value = u32::deserialize(deserializer)?;

        ICAOAddress::new(raw_value).map_err(D::Error::custom)
    }
}

impl TryFrom<PbAircraft> for Aircraft {
    type Error = AircraftConversionError;
    fn try_from(pb_aircraft: PbAircraft) -> Result<Self, Self::Error> {
        let icao_address = ICAOAddress::new(pb_aircraft.icao_address)?;
        let ts = pb_aircraft
            .broadcasted_timestamp
            .ok_or(AircraftConversionError::MissingDatetime)?;
        let sys_time = SystemTime::try_from(ts)?;
        let broadcasted_timestamp = DateTime::<Utc>::from(sys_time);

        Ok(Self {
            callsign: pb_aircraft.callsign,
            icao_address,
            broadcasted_timestamp,
            latitude: pb_aircraft.latitude,
            longitude: pb_aircraft.longitude,
            ground_track: pb_aircraft.ground_track,
            ground_speed: pb_aircraft.ground_speed,
            gps_altitude: pb_aircraft.gps_altitude,
        })
    }
}

impl IntoLogMessage<Aircraft> for Aircraft {
    type Error = Infallible;
    fn message_timestamp(&self) -> DateTime<Utc> {
        Utc::now()
    }
    fn into_message(self) -> Result<Aircraft, Self::Error> {
        Ok(self)
    }
}

impl From<Aircraft> for PbAircraft {
    fn from(aircraft: Aircraft) -> Self {
        let system_time = SystemTime::from(aircraft.broadcasted_timestamp);
        let timestamp = prost_types::Timestamp::from(system_time);
        Self {
            callsign: aircraft.callsign,
            icao_address: aircraft.icao_address.value(),
            broadcasted_timestamp: Some(timestamp),
            latitude: aircraft.latitude,
            longitude: aircraft.longitude,
            ground_track: aircraft.ground_track,
            ground_speed: aircraft.ground_speed,
            gps_altitude: aircraft.gps_altitude,
        }
    }
}

impl ReplayableMessage<ProtoFormat> for Aircraft {
    type Message = PbAircraft;
    type Error = AircraftConversionError;

    fn from_stored(stored: PbAircraft) -> Result<Self, Self::Error> {
        stored.try_into()
    }
}

impl From<AircraftJson> for Aircraft {
    fn from(stored: AircraftJson) -> Self {
        Self {
            callsign: stored.callsign,
            icao_address: stored.icao_address,
            broadcasted_timestamp: stored.broadcasted_timestamp,
            latitude: stored.latitude,
            longitude: stored.longitude,
            ground_track: stored.ground_track,
            ground_speed: stored.ground_speed,
            gps_altitude: stored.gps_altitude,
        }
    }
}

impl ReplayableMessage<JsonFormat> for Aircraft {
    type Message = AircraftJson;
    type Error = Infallible;

    fn from_stored(stored: AircraftJson) -> Result<Self, Self::Error> {
        Ok(stored.into())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use flights_core::logging::{LogSender, LoggerRegistry};
    use flights_core::reader::DecodeMessage;
    use flights_core::thread_manager::{SteppableTask, TaskState};

    use super::*;

    fn aircraft_at(second: i64) -> Aircraft {
        Aircraft {
            callsign: "TEST".to_owned(),
            icao_address: ICAOAddress::new(0x00AB_CDEF).unwrap(),
            broadcasted_timestamp: DateTime::from_timestamp(1_760_000_000, 0).unwrap()
                + TimeDelta::seconds(second),
            latitude: 47.5,
            longitude: 8.5,
            ground_track: 90.0,
            ground_speed: 30.0,
            gps_altitude: 1200.0,
        }
    }

    #[test]
    fn given_aircraft_logged_by_registry_when_decoded_and_converted_then_identical_aircraft_in_order()
     {
        let path = testdir::testdir!().join("parser.mcap");
        let aircraft: Vec<Aircraft> = (0..3).map(aircraft_at).collect();
        let mut registry = LoggerRegistry::new();
        let handle = registry
            .register_jsonl::<AircraftJson>(path.clone(), "parser".to_owned())
            .unwrap();
        let mut logger = registry.build();
        for one in &aircraft {
            handle.send(one.clone()).unwrap();
            assert!(matches!(logger.step(), TaskState::Running));
        }
        drop(logger);

        let bytes = std::fs::read(&path).unwrap();
        let replayed: Vec<Aircraft> = mcap::MessageStream::new(&bytes)
            .unwrap()
            .map(|message| {
                let stored: AircraftJson = JsonFormat::decode(&message.unwrap().data).unwrap();
                <Aircraft as ReplayableMessage<JsonFormat>>::from_stored(stored).unwrap()
            })
            .collect();

        assert_eq!(replayed, aircraft);
    }

    #[test]
    fn given_aircraft_as_protobuf_when_converted_back_then_equals_original() {
        let original = aircraft_at(0);
        let stored = PbAircraft::from(original.clone());

        let replayed = <Aircraft as ReplayableMessage<ProtoFormat>>::from_stored(stored).unwrap();

        assert_eq!(replayed, original);
    }

    #[test]
    fn given_json_with_out_of_range_icao_address_when_decoded_then_decode_error() {
        // ICAO addresses are 24-bit: 0x0100_0000 is one past the largest.
        let json = br#"{"callsign":"TEST","icao_address":16777216,"broadcasted_timestamp":"2025-10-01T19:05:58Z","latitude":47.5,"longitude":8.5,"ground_track":90.0,"ground_speed":30.0,"gps_altitude":1200.0}"#;

        let decoded: Result<AircraftJson, _> = JsonFormat::decode(json);

        assert!(decoded.is_err());
    }

    #[test]
    fn given_pb_aircraft_with_out_of_range_icao_address_when_converted_then_conversion_error() {
        let mut stored = PbAircraft::from(aircraft_at(0));
        stored.icao_address = 0x0100_0000;

        assert!(<Aircraft as ReplayableMessage<ProtoFormat>>::from_stored(stored).is_err());
    }
}
