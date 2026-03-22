use ogn_aprs_parser::{AircraftBeacon, ICAOAddress};

#[derive(Debug, PartialEq, Clone)]
pub struct Aircraft {
    pub callsign: String,
    pub icao_address: ICAOAddress,
    pub datetime: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}

pub fn convert_ogn_aprs_beacon_to_aircraft(aircraft_beacon: AircraftBeacon) -> Aircraft {
    let now = chrono::Utc::now();

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
