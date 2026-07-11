use std::convert::Infallible;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use ogn_aprs_parser::{AircraftBeacon, ICAOAddress};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::central_disk_logger::{IntoLogMessage, McapSchemaDescriptor};
use crate::parser::errors::AircraftConversionError;
use crate::pb::parser::PbAircraft;

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
impl McapSchemaDescriptor for AircraftJson {
    fn schema_name() -> String {
        "AircraftJson".to_string()
    }
    fn encoding() -> &'static str {
        "jsonschema"
    }

    fn schema_bytes() -> Vec<u8> {
        let schema = schemars::schema_for!(AircraftJson);

        let schema_json_string =
            serde_json::to_string(&schema).expect("Failed to serialize JSON schema");

        schema_json_string.into_bytes()
    }
    fn topic() -> String {
        "aircraft_json".to_string()
    }
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
