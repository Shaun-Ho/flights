use ogn_aprs_parser::ICAOAddress;

use crate::parser::Aircraft;

#[must_use]
pub fn create_dummy_aircraft_at_time(
    timestamp: chrono::DateTime<chrono::Utc>,
    icao_address: ICAOAddress,
) -> Aircraft {
    Aircraft {
        callsign: String::from("dummy"),
        icao_address,
        broadcasted_timestamp: timestamp,
        latitude: 0.0,
        longitude: 0.0,
        ground_track: 0.0,
        ground_speed: 0.0,
        gps_altitude: 0.0,
    }
}
