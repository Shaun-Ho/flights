use crate::parser::constants::{CALLSIGN_DELITMETER, HEADER_BODY_DELIMITER};
use crate::parser::constants::{
    GPS_ALTITUDE, GROUND_SPEED, GROUND_TRACK, LATITUDE_DEGREES, LATITUDE_MINUTES,
    LONGITUDE_DEGREES, LONGITUDE_MINUTES, OGN_BEACON_ID,
};
use crate::parser::constants::{GPS_DATA_REGEX, OGN_BEACON_ID_REGEX};
use crate::parser::types::OGNBeaconID;
use crate::types::{Aircraft, ICAOAddress};

pub fn build_aircraft_from_string(string: &str) -> Result<Aircraft, AircraftBuildError> {
    string
        .find(CALLSIGN_DELITMETER)
        .ok_or(AircraftBuildError::InvalidMessage(
            InvalidMessageError::InvalidFormat(String::from("No valid callsign")),
        ))?;

    string
        .find(HEADER_BODY_DELIMITER)
        .ok_or(AircraftBuildError::InvalidMessage(
            InvalidMessageError::InvalidFormat(String::from("Cannot establish header")),
        ))?;

    let callsign_pos = string
        .find(CALLSIGN_DELITMETER)
        .ok_or(AircraftBuildError::MissingCallsign)?;

    let (header, body) =
        string
            .split_once(HEADER_BODY_DELIMITER)
            .ok_or(AircraftBuildError::InvalidMessage(
                InvalidMessageError::MissingHeaderOrBodyError(String::from(
                    "Cannot establish header",
                )),
            ))?;

    let callsign = &header[..callsign_pos];

    let split = body.split_whitespace();

    let mut builder = AircraftBuilder::new();
    builder.callsign = Some(callsign.to_string());

    for chunk in split {
        let info_opt = extract_data_from_string(chunk)?;
        if let Some(info) = info_opt {
            match info {
                AircraftBuilderData::GPSData {
                    time,
                    latitude,
                    longitude,
                    ground_track,
                    ground_speed,
                    gps_altitude,
                } => {
                    builder.time = Some(time);
                    builder.latitude = Some(latitude);
                    builder.longitude = Some(longitude);
                    builder.ground_track = Some(ground_track);
                    builder.ground_speed = Some(ground_speed);
                    builder.gps_altitude = Some(gps_altitude);
                }

                AircraftBuilderData::OGNBeaconIDData(ogn_beacon_id) => {
                    builder.icao_address = Some(ogn_beacon_id.icao_address);
                }
            }
        }
    }

    builder.build()
}

#[derive(Debug)]
pub enum InvalidMessageError {
    InvalidFormat(String),
    InvalidTimeFormat(String),
    MissingHeaderOrBodyError(String),
    MissingCapture(String),
}
#[derive(Debug)]
pub enum AircraftBuildError {
    MissingCallsign,
    MissingICAOAddress,
    MissingTime,
    MissingLatitude,
    MissingLongitude,
    MissingGroundTrack,
    MissingGroundSpeed,
    MissingGPSAltitude,
    InvalidMessage(InvalidMessageError),
}

#[derive(Debug)]
struct AircraftBuilder {
    pub callsign: Option<String>,
    pub icao_address: Option<ICAOAddress>,
    pub time: Option<chrono::DateTime<chrono::Utc>>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub ground_track: Option<f64>,
    pub ground_speed: Option<f64>,
    pub gps_altitude: Option<f64>,
}

impl AircraftBuilder {
    #[must_use]
    pub fn new() -> AircraftBuilder {
        AircraftBuilder {
            callsign: None,
            icao_address: None,
            time: None,
            latitude: None,
            longitude: None,
            ground_track: None,
            ground_speed: None,
            gps_altitude: None,
        }
    }
    pub fn build(&self) -> Result<Aircraft, AircraftBuildError> {
        let callsign = self
            .callsign
            .as_ref()
            .ok_or(AircraftBuildError::MissingCallsign)?
            .to_string();
        let icao_address = self
            .icao_address
            .ok_or(AircraftBuildError::MissingICAOAddress)?;
        let time = self.time.ok_or(AircraftBuildError::MissingTime)?;
        let latitude = self.latitude.ok_or(AircraftBuildError::MissingLatitude)?;
        let longitude = self.longitude.ok_or(AircraftBuildError::MissingLongitude)?;
        let ground_track = self
            .ground_track
            .ok_or(AircraftBuildError::MissingGroundTrack)?;
        let ground_speed = self
            .ground_speed
            .ok_or(AircraftBuildError::MissingGroundSpeed)?;
        let gps_altitude = self
            .gps_altitude
            .ok_or(AircraftBuildError::MissingGPSAltitude)?;

        Ok(Aircraft {
            callsign,
            icao_address,
            datetime: time,
            latitude,
            longitude,
            ground_track,
            ground_speed,
            gps_altitude,
        })
    }
}

impl Default for AircraftBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq)]
enum AircraftBuilderData {
    GPSData {
        time: chrono::DateTime<chrono::Utc>,
        latitude: f64,
        longitude: f64,
        ground_track: f64,
        ground_speed: f64,
        gps_altitude: f64,
    },
    OGNBeaconIDData(OGNBeaconID),
}

fn extract_data_from_string(
    string: &str,
) -> Result<Option<AircraftBuilderData>, AircraftBuildError> {
    if let Some(captures) = GPS_DATA_REGEX.captures(string) {
        let time: String =
            parse_captures(&captures, "time").map_err(AircraftBuildError::InvalidMessage)?;
        let latitude_degrees: f64 = parse_captures(&captures, LATITUDE_DEGREES)
            .map_err(AircraftBuildError::InvalidMessage)?;
        let latitude_minutes: f64 = parse_captures(&captures, LATITUDE_MINUTES)
            .map_err(AircraftBuildError::InvalidMessage)?;
        let longitude_degrees: f64 = parse_captures(&captures, LONGITUDE_DEGREES)
            .map_err(AircraftBuildError::InvalidMessage)?;
        let longitude_minutes: f64 = parse_captures(&captures, LONGITUDE_MINUTES)
            .map_err(AircraftBuildError::InvalidMessage)?;
        let ground_track: f64 =
            parse_captures(&captures, GROUND_TRACK).map_err(AircraftBuildError::InvalidMessage)?;
        let ground_speed: f64 =
            parse_captures(&captures, GROUND_SPEED).map_err(AircraftBuildError::InvalidMessage)?;
        let gps_altitude: f64 =
            parse_captures(&captures, GPS_ALTITUDE).map_err(AircraftBuildError::InvalidMessage)?;

        Ok(Some(AircraftBuilderData::GPSData {
            time: convert_to_current_datetime(&time).map_err(AircraftBuildError::InvalidMessage)?,
            latitude: convert_latlon_minutes_to_decimals(latitude_degrees, latitude_minutes),
            longitude: convert_latlon_minutes_to_decimals(longitude_degrees, longitude_minutes),
            ground_track,
            ground_speed,
            gps_altitude,
        }))
    } else if let Some(captures) = OGN_BEACON_ID_REGEX.captures(string) {
        let ogn_beacon_id: OGNBeaconID =
            parse_captures(&captures, OGN_BEACON_ID).map_err(AircraftBuildError::InvalidMessage)?;
        Ok(Some(AircraftBuilderData::OGNBeaconIDData(ogn_beacon_id)))
    } else {
        Ok(None)
    }
}
fn parse_captures<T>(
    captures: &regex::Captures,
    string_name: &str,
) -> Result<T, InvalidMessageError>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    captures
        .name(string_name)
        .ok_or_else(|| InvalidMessageError::MissingCapture(string_name.to_string()))?
        .as_str()
        .parse::<T>()
        .map_err(|e| {
            InvalidMessageError::InvalidFormat(format!("{string_name} component has error: {e}.",))
        })
}

fn convert_latlon_minutes_to_decimals(degrees: f64, minutes: f64) -> f64 {
    degrees + minutes / 60.0
}
fn convert_to_current_datetime(
    string: &str,
) -> Result<chrono::DateTime<chrono::Utc>, InvalidMessageError> {
    let today_utc = chrono::Utc::now().date_naive();
    let naive_time = chrono::NaiveTime::parse_from_str(string, "%H%M%S")
        .map_err(|e| InvalidMessageError::InvalidTimeFormat(e.to_string()))?;
    Ok(today_utc.and_time(naive_time).and_utc())
}

#[cfg(test)]
mod test {
    use crate::parser::types::{OGNBeaconID, OGNIDPrefix};
    use crate::types::{Aircraft, ICAOAddress};

    use super::{AircraftBuilderData, build_aircraft_from_string, extract_data_from_string};

    #[test]
    fn when_unpacking_valid_string_for_gps_data_then_correct_data_is_extracted() {
        let string = String::from("102100h4938.77N/00848.62E^129/435/A=035443");
        let aircraft_data = extract_data_from_string(&string)
            .expect("This should construct no errors")
            .expect("This should return Some value");
        let expected_time = chrono::NaiveTime::from_hms_opt(10, 21, 00).unwrap();
        let expected_date = chrono::Local::now().date_naive();
        let expected_datetime = expected_date.and_time(expected_time).and_utc();
        let expected_aircraft_data = AircraftBuilderData::GPSData {
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
    fn when_unpacking_valid_string_for_ogn_beacon_id_data_then_correct_data_is_extracted() {
        let string = String::from("id00A80F3B");
        let aircraft_data = extract_data_from_string(&string)
            .expect("This should construct no errors")
            .expect("This should return Some value");
        let expected_icao_address = ICAOAddress::new(11013947).unwrap();
        let expected_prefix = OGNIDPrefix::new(0).unwrap();
        let expected_beacon_id_data = OGNBeaconID::new(expected_prefix, expected_icao_address);

        let expected_aircraft_data = AircraftBuilderData::OGNBeaconIDData(expected_beacon_id_data);
        assert_eq!(aircraft_data, expected_aircraft_data);
    }
    #[test]
    fn when_valid_ogn_line_is_parsed_then_correct_aircraft_struct_is_constructed() {
        let string = String::from(
            "ICA407F7A>OGADSB,qAS,Lengfeld:/102100h4938.77N/00848.62E^129/435/A=035443 !W29! id25407F7A +0fpm FL349.75 A3:EZY62RN Sq2731\r\n",
        );
        let expected_time = chrono::NaiveTime::from_hms_opt(10, 21, 00).unwrap();
        let expected_date = chrono::Local::now().date_naive();
        let expected_datetime = expected_date.and_time(expected_time).and_utc();

        let aircraft = build_aircraft_from_string(&string).unwrap();
        let expected_aircraft = Aircraft {
            callsign: String::from("ICA407F7A"),
            icao_address: ICAOAddress::new(0x407F7A).unwrap(),
            datetime: expected_datetime,
            latitude: 49.646166666666666,
            longitude: 8.810333333333332,
            ground_track: 129.0,
            ground_speed: 435.0,
            gps_altitude: 35443.0,
        };
        assert_eq!(aircraft, expected_aircraft);
    }
}
