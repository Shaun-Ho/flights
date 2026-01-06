use super::types::ICAOAddress;
#[derive(Debug, PartialEq)]
pub struct Aircraft {
    pub callsign: String,
    pub icao_address: ICAOAddress,
    pub time: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}
