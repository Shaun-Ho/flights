use super::constants::{FLIGHT_LEVEL_REGEX, GPS_DATA_REGEX, ICAO_ADDRESS};

#[derive(Debug)]
pub enum AircraftBuildError {
    InvalidFormat(String),
    InvalidTimeFormat(String),
    MissingHeaderOrBodyError(String),
    UnkownField(String),
    MissingCapture(String),
    MissingCallsign,
    MissingICAOAddress,
    MissingTime,
    MissingLatitude,
    MissingLongitude,
    MissingGroundTrack,
    MissingGPSAltitude,
    MissingStandardPressureAltitude,
    MissingClimbRate,
}

#[derive(Debug, PartialEq)]
enum AircraftData {
    GPSData {
        time: chrono::DateTime<chrono::Utc>,
        latitude: f64,
        longitude: f64,
        ground_track: f64,
        ground_speed: f64,
        gps_altitude: f64,
    },
    FlightLevel(f64),
    ICAOAddress(u32),
}
#[allow(dead_code)]
fn extract_data_from_string(string: &str) -> Result<AircraftData, AircraftBuildError> {
    if let Some(captures) = GPS_DATA_REGEX.captures(string) {
        let time: String = parse_captures(&captures, "time")?;
        let latitude_degrees: f64 = parse_captures(&captures, "latitude_degrees")?;
        let latitude_minutes: f64 = parse_captures(&captures, "latitude_minutes")?;
        let longitude_degrees: f64 = parse_captures(&captures, "longitude_degrees")?;
        let longitude_minutes: f64 = parse_captures(&captures, "longitude_minutes")?;
        let ground_track: f64 = parse_captures(&captures, "ground_track")?;
        let ground_speed: f64 = parse_captures(&captures, "ground_speed")?;
        let gps_altitude: f64 = parse_captures(&captures, "gps_altitude")?;

        Ok(AircraftData::GPSData {
            time: convert_to_current_datetime(&time)?,
            latitude: convert_latlon_minutes_to_decimals(latitude_degrees, latitude_minutes),
            longitude: convert_latlon_minutes_to_decimals(longitude_degrees, longitude_minutes),
            ground_track,
            ground_speed,
            gps_altitude,
        })
    } else if let Some(captures) = FLIGHT_LEVEL_REGEX.captures(string) {
        let flight_level: f64 = parse_captures(&captures, "flight_level")?;
        Ok(AircraftData::FlightLevel(flight_level))
    } else if let Some(captures) = ICAO_ADDRESS.captures(string) {
        let icao_address: u32 = parse_captures(&captures, "icao_address")?;
        Ok(AircraftData::ICAOAddress(icao_address))
    } else {
        Err(AircraftBuildError::UnkownField(String::from("a")))
    }
}
pub fn parse_captures<T>(
    captures: &regex::Captures,
    string_name: &str,
) -> Result<T, AircraftBuildError>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    captures
        .name(string_name)
        .ok_or_else(|| AircraftBuildError::MissingCapture(string_name.to_string()))?
        .as_str()
        .parse::<T>()
        .map_err(|e| {
            AircraftBuildError::InvalidFormat(format!("{string_name} component has error: {e}.",))
        })
}

fn convert_latlon_minutes_to_decimals(degrees: f64, minutes: f64) -> f64 {
    degrees + minutes / 60.0
}
fn convert_to_current_datetime(
    string: &str,
) -> Result<chrono::DateTime<chrono::Utc>, AircraftBuildError> {
    let today_utc = chrono::Utc::now().date_naive();
    let naive_time = chrono::NaiveTime::parse_from_str(string, "%H%M%S")
        .map_err(|e| AircraftBuildError::InvalidTimeFormat(e.to_string()))?;
    Ok(today_utc.and_time(naive_time).and_utc())
}

#[cfg(test)]
mod test {
    use super::{AircraftData, extract_data_from_string};

    #[test]
    fn when_unpacking_valid_string_for_gps_data_then_correct_data_is_extracted() {
        let string = String::from("102100h4938.77N/00848.62E^129/435/A=035443");
        let aircraft_data = extract_data_from_string(&string).expect("Test should pass");
        let expected_time = chrono::NaiveTime::from_hms_opt(10, 21, 00).unwrap();
        let expected_date = chrono::Local::now().date_naive();
        let expected_datetime = expected_date.and_time(expected_time).and_utc();
        let expected_aircraft_data = AircraftData::GPSData {
            time: expected_datetime,
            latitude: 49.646166666666666,
            longitude: 8.810333333333332,
            ground_track: 129.0,
            ground_speed: 435.0,
            gps_altitude: 35443.0,
        };
        assert_eq!(aircraft_data, expected_aircraft_data);
    }

    #[test]
    fn when_unpacking_valid_string_for_flight_level_then_correct_data_is_extracted() {
        let string = String::from("FL349");
        let aircraft_data = extract_data_from_string(&string).expect("Test should pass");
        let expected_flight_level = 349.0;

        let expected_aircraft_info = AircraftData::FlightLevel(expected_flight_level);
        assert_eq!(aircraft_data, expected_aircraft_info);
    }

    #[test]
    fn when_unpacking_valid_string_for_icao_address_then_correct_data_is_extracted() {
        let string = String::from("id12345678");
        let aircraft_data = extract_data_from_string(&string).expect("Test should pass");
        let expected_icao_address = 12345678;

        let expected_aircraft_info = AircraftData::ICAOAddress(expected_icao_address);
        assert_eq!(aircraft_data, expected_aircraft_info);
    }
}
