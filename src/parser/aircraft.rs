use super::types::ICAOAddress;
#[derive(Debug)]
pub struct Aircraft {
    pub callsign: String,
    pub icao_address: ICAOAddress,
    pub time: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub gps_altitude: f64,
    pub flight_level: f64,
    pub standard_pressure_altitude: f64,
    pub climb_rate: f64,
}
