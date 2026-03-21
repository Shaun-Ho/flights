use super::errors::{APRSParseContext, AircraftParseError};
use crate::aprs_types::{APRSSignalType, OGNBeaconID};

use nom::{
    Parser,
    bytes::complete::{tag, take, take_until},
};

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

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub struct ICAOAddress(u32);

impl ICAOAddress {
    pub const MAX_VALUE: u32 = 0x00FF_FFFF;

    pub fn new(value: u32) -> Result<Self, ICAOAddressError> {
        if value <= Self::MAX_VALUE {
            Ok(ICAOAddress(value))
        } else {
            Err(ICAOAddressError::InvalidAddress(value))
        }
    }

    #[must_use]
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ICAOAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08X}", self.0)
    }
}

#[derive(Debug)]
pub enum ICAOAddressError {
    InvalidHexFormat,
    InvalidAddress(u32),
}
impl std::fmt::Display for ICAOAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ICAOAddressError::InvalidHexFormat => write!(f, "Invalid hexadecimal format"),
            ICAOAddressError::InvalidAddress(val) => {
                write!(
                    f,
                    "Value 0x{:X} ({}) exceeds 24-bit ICAO address limit (0x{:X})",
                    val,
                    val,
                    ICAOAddress::MAX_VALUE
                )
            }
        }
    }
}

pub fn parse_aircraft(input: &[u8]) -> Result<Aircraft, AircraftParseError> {
    use nom::Finish;

    let (input, callsign) = parse_callsign(input).finish()?;

    let (input, _aprs_packet_type) = parse_aprs_signal_type(input).finish()?;

    let (input, _) = (take_until(b":/".as_slice()), tag(b":/".as_slice()))
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let (input, datetime) = parse_timestamp(input).finish()?;

    let (input, _) = (take_until(b"h".as_slice()), tag(b"h".as_slice()))
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let parse_specific_coordinate = |input, coord| parse_coordinate(input, coord);

    let (input, latitude) = parse_specific_coordinate(input, Coordinate::Latitude).finish()?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let (input, longitude) = parse_specific_coordinate(input, Coordinate::Longitude).finish()?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let (input, ground_track) = parse_ground_track(input).finish()?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let (input, ground_speed) = parse_ground_speed(input).finish()?;

    let (input, _) = (take_until(b"A=".as_slice()), tag(b"A=".as_slice()))
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let (input, gps_altitude) = parse_gps_altitude(input).finish()?;

    let (input, _) = (take_until(b" id".as_slice()), tag(b" ".as_slice()))
        .parse(input)
        .finish()
        .map_err(AircraftParseError::IncorrectSeparator)?;

    let (_input, ogn_beacon_id) = parse_ogn_beacon_id(input).finish()?;

    Ok(Aircraft {
        callsign: callsign.to_string(),
        datetime,
        latitude,
        longitude,
        ground_track,
        ground_speed,
        gps_altitude,
        icao_address: ogn_beacon_id.icao_address,
    })
}

enum Coordinate {
    Latitude,
    Longitude,
}
impl std::fmt::Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Coordinate::Latitude => write!(f, "latitude"),
            Coordinate::Longitude => write!(f, "longitude"),
        }
    }
}
fn convert_latlon_minutes_to_decimals(degrees: f64, minutes: f64) -> f64 {
    degrees + minutes / 60.0
}

fn parse_callsign(input: &[u8]) -> nom::IResult<&[u8], &str, AircraftParseError> {
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(b">".as_slice()), tag(b">".as_slice())),
        std::str::from_utf8,
    )
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            AircraftParseError::InvalidCallsign(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid callsign".to_string(),
            })
        })
    })
}

fn parse_timestamp(
    input: &[u8],
) -> nom::IResult<&[u8], chrono::DateTime<chrono::Utc>, AircraftParseError> {
    let parse_to_datetime = |s: &[u8]| -> Result<chrono::DateTime<chrono::Utc>, String> {
        let s_str = std::str::from_utf8(s).map_err(|_| "invalid utf8")?;
        let now = chrono::Utc::now();
        let h = s_str[0..2]
            .parse::<u32>()
            .map_err(|_| "invalid hour digits")?;
        let m = s_str[2..4]
            .parse::<u32>()
            .map_err(|_| "invalid minute digits")?;
        let s = s_str[4..6]
            .parse::<u32>()
            .map_err(|_| "invalid second digits")?;

        let native_time = chrono::NaiveTime::from_hms_opt(h, m, s).ok_or("invalid time")?;
        Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            now.date_naive().and_time(native_time),
            chrono::Utc,
        ))
    };

    nom::combinator::map_res(take(6usize), parse_to_datetime)
        .parse(input)
        .map_err(|err| err.map(AircraftParseError::InvalidTimestamp))
}

#[allow(clippy::needless_pass_by_value)]
fn parse_coordinate(
    input: &[u8],
    coord: Coordinate,
) -> nom::IResult<&[u8], f64, AircraftParseError> {
    let create_error = |msg: &str| {
        let context = APRSParseContext {
            input: String::from_utf8_lossy(input).to_string(),
            message: msg.to_string(),
        };
        match coord {
            Coordinate::Latitude => AircraftParseError::InvalidLatitude(context),
            Coordinate::Longitude => AircraftParseError::InvalidLongitude(context),
        }
    };
    let suffix_negative = match coord {
        Coordinate::Latitude => b"S",
        Coordinate::Longitude => b"W",
    };

    let size = match coord {
        Coordinate::Latitude => 2usize,
        Coordinate::Longitude => 3usize,
    };
    let (remainder, degrees_bytes) = take(size).parse(input).map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| create_error("invalid number of digits for degrees"))
    })?;

    let (remainder, minutes_bytes) = take(5usize).parse(remainder).map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| create_error("invalid number of digits for minutes"))
    })?;

    let (remainder, matched_suffix) = take(1usize).parse(remainder).map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| create_error("no suffix for coordinate found"))
    })?;

    let degrees_f64 = std::str::from_utf8(degrees_bytes)
        .unwrap_or("")
        .parse::<f64>()
        .map_err(|_| nom::Err::Failure(create_error("could not parse degrees")))?;

    let minutes_f64 = std::str::from_utf8(minutes_bytes)
        .unwrap_or("")
        .parse::<f64>()
        .map_err(|_| nom::Err::Failure(create_error("could not parse minutes")))?;

    let value = convert_latlon_minutes_to_decimals(degrees_f64, minutes_f64);

    if matched_suffix == suffix_negative {
        Ok((remainder, -value))
    } else {
        Ok((remainder, value))
    }
}

fn parse_aprs_signal_type(input: &[u8]) -> nom::IResult<&[u8], APRSSignalType, AircraftParseError> {
    let parse_to_aprs_signal_type = |s: &[u8]| -> Result<APRSSignalType, AircraftParseError> {
        std::str::from_utf8(s)
            .unwrap_or("")
            .parse::<APRSSignalType>()
    };
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(b",".as_slice()), tag(b",".as_slice())),
        parse_to_aprs_signal_type,
    )
    .parse(input)
}

fn parse_ground_track(input: &[u8]) -> nom::IResult<&[u8], f64, AircraftParseError> {
    nom::combinator::map_res(take(3usize), |s: &[u8]| {
        std::str::from_utf8(s).unwrap_or("").parse::<f64>()
    })
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            AircraftParseError::InvalidGroundTrack(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid ground track".to_string(),
            })
        })
    })
}

fn parse_ground_speed(input: &[u8]) -> nom::IResult<&[u8], f64, AircraftParseError> {
    nom::combinator::map_res(take(3usize), |s: &[u8]| {
        std::str::from_utf8(s).unwrap_or("").parse::<f64>()
    })
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            AircraftParseError::InvalidGroundSpeed(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid ground speed".to_string(),
            })
        })
    })
}

fn parse_gps_altitude(input: &[u8]) -> nom::IResult<&[u8], f64, AircraftParseError> {
    nom::combinator::map_res(take(6usize), |s: &[u8]| {
        std::str::from_utf8(s).unwrap_or("").parse::<f64>()
    })
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            AircraftParseError::InvalidGPSAltitude(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid gps altitude".to_string(),
            })
        })
    })
}

fn parse_ogn_beacon_id(input: &[u8]) -> nom::IResult<&[u8], OGNBeaconID, AircraftParseError> {
    // string is of format `idXXYYYYYY`
    let (remainder, id_bytes) = nom::sequence::preceded(tag(b"id".as_slice()), take(8usize))
        .parse(input)
        .map_err(|e| {
            e.map(|_nom_err: nom::error::Error<&[u8]>| {
                AircraftParseError::InvalidOGNBeaconId(APRSParseContext {
                    input: String::from_utf8_lossy(input).to_string(),
                    message: "invalid ogn beacon id format".to_string(),
                })
            })
        })?;

    let id_str = std::str::from_utf8(id_bytes).unwrap_or("");
    match id_str.parse::<OGNBeaconID>() {
        Ok(beacon_id) => Ok((remainder, beacon_id)),
        Err(err) => Err(nom::Err::Failure(AircraftParseError::InvalidOGNBeaconId(
            APRSParseContext {
                input: id_str.to_string(),
                message: format!("invalid ogn beacon id: {err}"),
            },
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::aprs_types::{OGNAddressType, OGNAircraftType, OGNBeaconID, OGNIDPrefix};
    use crate::parse::AircraftParseError;
    use crate::parse::parse_aircraft;
    use crate::parse::{Aircraft, ICAOAddress};
    use crate::parse::{
        Coordinate, parse_aprs_signal_type, parse_callsign, parse_coordinate, parse_gps_altitude,
        parse_ground_speed, parse_ground_track, parse_ogn_beacon_id, parse_timestamp,
    };
    use approx::relative_eq;
    use nom::Finish;

    #[test]
    fn when_packet_contains_valid_callsign_identifier_is_correct_then_parsed_callsign_is_correct() {
        let input = b"ICA4B37A8>".as_slice();
        let expected_callsign = "ICA4B37A8";
        match parse_callsign(input).finish() {
            Ok((_, callsign)) => assert_eq!(callsign, expected_callsign),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_packet_contains_invalid_callsign_identifier_then_correct_error_is_returned() {
        let input = b"HEADER:/2a0600h".as_slice();

        match parse_callsign(input).finish() {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(AircraftParseError::InvalidCallsign(info)) => {
                assert_eq!(info.input, "HEADER:/2a0600h");
                assert_eq!(info.message, "invalid callsign");
            }

            Err(other) => panic!("Expected InvalidCallsign, got: {other}"),
        }
    }

    #[test]
    fn when_packet_contains_ognadsb_signal_type_then_message_continue_parsing() {
        let input = b"OGADSB,".as_slice();
        match parse_aprs_signal_type(input).finish() {
            Ok(_) => {}
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }

    mod timestamps {
        use super::*;

        #[test]
        fn when_valid_timestamp_digits_parsed_then_correct_datetime_is_returned() {
            let input = b"190600h".as_slice();
            let now = chrono::Utc::now();
            let expected_datetime = now
                .date_naive()
                .and_time(chrono::NaiveTime::from_hms_opt(19, 0o6, 00).unwrap())
                .and_utc();

            match parse_timestamp(input) {
                Ok((_, datetime)) => assert_eq!(datetime, expected_datetime),
                Err(err) => panic!("Expected no errors, received:  {err}"),
            }
        }
        #[test]
        fn when_invalid_timestamp_digits_parsed_then_error_shows_correct_digit_error() {
            let input = b"2a0600h".as_slice();

            match parse_timestamp(input).finish() {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(AircraftParseError::InvalidTimestamp(info)) => {
                    assert_eq!(info.input, "2a0600h");
                    assert_eq!(info.message, "invalid hour digits");
                }

                Err(other) => panic!("Expected InvalidTimestamp, got: {other}"),
            }
        }
        #[test]
        fn when_invalid_timestamp_parsed_then_error_shows_correct_time_conversion_error() {
            let input = b"260600h".as_slice();

            match parse_timestamp(input).finish() {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(AircraftParseError::InvalidTimestamp(info)) => {
                    assert_eq!(info.input, "260600h");
                    assert_eq!(info.message, "invalid time");
                }

                Err(other) => panic!("Expected InvalidTimestamp, got: {other}"),
            }
        }
    }
    mod coordinates {
        use super::*;

        #[test]
        fn when_valid_latitude_north_coordinates_then_correct_latitude_is_returned() {
            let input = b"4121.18N".as_slice();
            let expected_latitude = 41.353;
            match parse_coordinate(input, Coordinate::Latitude).finish() {
                Ok((_, latitude)) => assert!(relative_eq!(latitude, expected_latitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }
        #[test]
        fn when_valid_latitude_south_coordinates_then_correct_latitude_is_returned() {
            let input = b"4121.18S".as_slice();
            let expected_latitude = -41.353;
            match parse_coordinate(input, Coordinate::Latitude).finish() {
                Ok((_, latitude)) => assert!(relative_eq!(latitude, expected_latitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_invalid_latitude_coordinates_then_correct_error_returned() {
            let input = b"4121.18".as_slice();

            match parse_coordinate(input, Coordinate::Latitude).finish() {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(AircraftParseError::InvalidLatitude(info)) => {
                    assert_eq!(info.input, "4121.18");
                    assert_eq!(info.message, "no suffix for coordinate found");
                }

                Err(other) => panic!("Expected InvalidLatitude, got: {other}"),
            }
        }

        #[test]
        fn when_valid_longitude_east_coordinates_then_correct_longitude_is_returned() {
            let input = b"12219.21E".as_slice();
            let expected_longitude = 122.320_166_666_666_67;
            match parse_coordinate(input, Coordinate::Longitude).finish() {
                Ok((_, longitude)) => assert!(relative_eq!(longitude, expected_longitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_valid_longitude_west_coordinates_then_correct_longitude_is_returned() {
            let input = b"12219.21W".as_slice();
            let expected_longitude = -122.320_166_666_666_67;
            match parse_coordinate(input, Coordinate::Longitude).finish() {
                Ok((_, longitude)) => assert!(relative_eq!(longitude, expected_longitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_invalid_longitude_coordinates_then_correct_error_returned() {
            let input = b"00219.21".as_slice();

            match parse_coordinate(input, Coordinate::Longitude).finish() {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(AircraftParseError::InvalidLongitude(info)) => {
                    assert_eq!(info.input, "00219.21");
                    assert_eq!(info.message, "no suffix for coordinate found");
                }

                Err(other) => panic!("Expected InvalidLongitude, got: {other}"),
            }
        }
    }

    #[test]
    fn when_correct_ground_track_is_given_then_correct_value_is_returned() {
        let input = b"123".as_slice();
        let expected_ground_track = 123.0;
        match parse_ground_track(input).finish() {
            Ok((_, ground_track)) => assert!(relative_eq!(ground_track, expected_ground_track)),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_invalid_ground_track_parsed_then_correct_error_is_returned() {
        let input = b"12a".as_slice();

        match parse_ground_track(input).finish() {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(AircraftParseError::InvalidGroundTrack(info)) => {
                assert_eq!(info.input, "12a");
                assert_eq!(info.message, "invalid ground track");
            }

            Err(other) => panic!("Expected InvalidGroundTrack, got: {other}"),
        }
    }
    #[test]
    fn when_correct_ground_speed_is_given_then_correct_value_is_returned() {
        let input = b"123".as_slice();
        let expected_ground_track = 123.0;
        match parse_ground_speed(input).finish() {
            Ok((_, ground_track)) => {
                assert!(relative_eq!(ground_track, expected_ground_track));
            }
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_invalid_ground_speed_parsed_then_correct_error_is_returned() {
        let input = b"12a".as_slice();

        match parse_ground_speed(input).finish() {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(AircraftParseError::InvalidGroundSpeed(info)) => {
                assert_eq!(info.input, "12a");
                assert_eq!(info.message, "invalid ground speed");
            }

            Err(other) => panic!("Expected InvalidGroundSpeed, got: {other}"),
        }
    }
    #[test]
    fn when_correct_gps_altitude_is_given_then_correct_value_is_returned() {
        let input = b"002341".as_slice();
        let expected_gps_altitude = 2341.0;
        match parse_gps_altitude(input).finish() {
            Ok((_, gps_altitude)) => assert!(relative_eq!(gps_altitude, expected_gps_altitude)),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_invalid_gps_alitude_parsed_then_correct_error_is_returned() {
        let input = b"12a".as_slice();

        match parse_gps_altitude(input).finish() {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(AircraftParseError::InvalidGPSAltitude(info)) => {
                assert_eq!(info.input, "12a");
                assert_eq!(info.message, "invalid gps altitude");
            }

            Err(other) => panic!("Expected InvalidGPSAltitude, got: {other}"),
        }
    }
    #[test]
    fn when_when_valid_ogn_beacon_id_is_given_then_correct_ogn_beacon_id_returned() {
        let input = b"id253007EE".as_slice();
        let expected_ogn_beacon_id = OGNBeaconID::new(
            OGNIDPrefix {
                aircraft_type: OGNAircraftType::JetTurbopropAircraft,
                no_track: false,
                address_type: OGNAddressType::ICAO,
                stealth_mode: false,
            },
            ICAOAddress::new(3_147_758).unwrap(),
        );
        match parse_ogn_beacon_id(input).finish() {
            Ok((_, ogn_beacon_id)) => assert_eq!(ogn_beacon_id, expected_ogn_beacon_id),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_when_invalid_ogn_beacon_id_from_hex_length_is_given_then_correct_error_is_returned() {
        let input = b"id123".as_slice();

        match parse_ogn_beacon_id(input).finish() {
            Ok(_) => panic!("Expected an error"),

            Err(AircraftParseError::InvalidOGNBeaconId(info)) => {
                assert_eq!(info.input, "id123");
                assert_eq!(info.message, "invalid ogn beacon id format");
            }

            Err(other) => panic!("Expected InvalidGPSAltitude, got: {other}"),
        }
    }
    #[test]
    fn when_when_invalid_ogn_beacon_id_hex_format_is_given_then_correct_error_is_returned() {
        let input = b"id253007EG".as_slice();

        match parse_ogn_beacon_id(input).finish() {
            Ok(_) => panic!("Expected an error"),

            Err(AircraftParseError::InvalidOGNBeaconId(info)) => {
                assert_eq!(info.input, "253007EG");
                assert_eq!(
                    info.message,
                    "invalid ogn beacon id: Invalid hexadecimal format"
                );
            }

            Err(other) => panic!("Expected InvalidGPSAltitude, got: {other}"),
        }
    }

    struct AircraftComparison {
        raw: &'static [u8],
        expected_callsign: &'static str,
        hms: (u32, u32, u32),
        lat_lon: (f64, f64),
        specs: (f64, f64, f64), // track, ground_speed, alt
        icao: u32,
    }

    impl AircraftComparison {
        fn to_comparison_tuple(&self) -> (&'static [u8], Aircraft) {
            let (h, m, s) = self.hms;
            let (lat, lon) = self.lat_lon;
            let (track, gs, alt) = self.specs;

            let aircraft = Aircraft {
                callsign: self.expected_callsign.to_string(),
                datetime: chrono::Utc::now()
                    .date_naive()
                    .and_time(chrono::NaiveTime::from_hms_opt(h, m, s).expect("Invalid time"))
                    .and_utc(),
                latitude: lat,
                longitude: lon,
                ground_track: track,
                ground_speed: gs,
                gps_altitude: alt,
                icao_address: ICAOAddress::new(self.icao).unwrap(),
            };
            (self.raw, aircraft)
        }
    }

    #[rstest::rstest]
    #[case::case_1(AircraftComparison {
        raw: b"ICA4400DC>OGADSB,qAS,HLST:/190606h5158.29N/01013.06E^066/488/A=034218 !W10! id254400DC -832fpm FL353.00 A3:EJU47ML".as_slice(),
        expected_callsign: "ICA4400DC",
        hms: (19, 6, 6),
        lat_lon: (51.9715, 10.217_666_666_666_666),
        specs: (66.0, 488.0, 34218.0),
        icao: 4_456_668
    })]
    #[case::case_2(AircraftComparison {
        raw: b"ICA4B027D>OGADSB,qAS,AVX1224:/190606h4651.87N/00118.95W^356/328/A=012618 !W37! id254B027D -1792fpm FL131.75 A3:EZS14TJ".as_slice(),
        expected_callsign:     "ICA4B027D",
        hms: (19, 6, 6),
        lat_lon: (46.8645, -1.315_833_333_333_333_4),
        specs: (356.0, 328.0, 12618.0),
        icao: 4_915_837
    })]
    fn test_aircraft_construction_(#[case] scenario: AircraftComparison) {
        let (raw_input, expected) = scenario.to_comparison_tuple();
        let result = parse_aircraft(raw_input).unwrap();

        assert_eq!(result, expected);
    }
}
