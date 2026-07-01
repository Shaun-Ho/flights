use std::time::SystemTime;

use ogn_aprs_parser::{AircraftBeacon, ICAOAddress};

#[derive(Debug, PartialEq, Clone)]
pub struct Aircraft {
    pub timestamp: SystemTime,
    pub callsign: String,
    pub icao_address: ICAOAddress,
    pub recorded_datetime: chrono::DateTime<chrono::Utc>,
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
        timestamp,
        callsign: aircraft_beacon.callsign,
        icao_address: aircraft_beacon.ogn_beacon_id.icao_address,
        recorded_datetime: datetime,
        latitude: aircraft_beacon.latitude,
        longitude: aircraft_beacon.longitude,
        ground_track: aircraft_beacon.ground_track,
        ground_speed: aircraft_beacon.ground_speed,
        gps_altitude: aircraft_beacon.gps_altitude,
    }
}
