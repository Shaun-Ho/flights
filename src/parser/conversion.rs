use std::time::SystemTime;

use chrono::{DateTime, Utc};
use ogn_aprs_parser::{AircraftBeacon, ICAOAddress};
use serde::{Deserialize, Serialize};

use crate::parser::errors::AircraftConversionError;
use crate::pb::parser::PbAircraft;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Aircraft {
    pub callsign: String,
    #[serde(with = "icao_serde")]
    pub icao_address: ICAOAddress,
    pub datetime: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}

pub fn convert_ogn_aprs_beacon_to_aircraft(
    aircraft_beacon: AircraftBeacon,
    timestamp: std::time::SystemTime,
) -> Aircraft {
    let now: chrono::DateTime<chrono::Utc> = timestamp.into();

    let datetime = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        now.date_naive().and_time(aircraft_beacon.time),
        chrono::Utc,
    );

    Aircraft {
        callsign: aircraft_beacon.callsign,
        icao_address: aircraft_beacon.ogn_beacon_id.icao_address,
        datetime,
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
    fn try_from(packet: PbAircraft) -> Result<Self, Self::Error> {
        let icao_address = ICAOAddress::new(packet.icao_address)?;
        let ts = packet
            .datetime
            .ok_or(AircraftConversionError::MissingDatetime)?;
        let sys_time = SystemTime::try_from(ts)?;
        let datetime = DateTime::<Utc>::from(sys_time);

        Ok(Self {
            callsign: packet.callsign,
            icao_address,
            datetime,
            latitude: packet.latitude,
            longitude: packet.longitude,
            ground_track: packet.ground_track,
            ground_speed: packet.ground_speed,
            gps_altitude: packet.gps_altitude,
        })
    }
}

impl From<Aircraft> for PbAircraft {
    fn from(aircraft: Aircraft) -> Self {
        let system_time = SystemTime::from(aircraft.datetime);
        let timestamp = prost_types::Timestamp::from(system_time);
        Self {
            callsign: aircraft.callsign,
            icao_address: aircraft.icao_address.value(),
            datetime: Some(timestamp),
            latitude: aircraft.latitude,
            longitude: aircraft.longitude,
            ground_track: aircraft.ground_track,
            ground_speed: aircraft.ground_speed,
            gps_altitude: aircraft.gps_altitude,
        }
    }
}
