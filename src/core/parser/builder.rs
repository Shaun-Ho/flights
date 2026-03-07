use super::errors;
use crate::core::parser::errors::APRSParseContext;
use crate::core::parser::types::OGNBeaconID;
use crate::core::types::Aircraft;

use nom::{
    Parser,
    bytes::complete::{tag, take, take_until},
};

fn convert_latlon_minutes_to_decimals(degrees: f64, minutes: f64) -> f64 {
    degrees + minutes / 60.0
}

fn parse_callsign(input: &str) -> nom::IResult<&str, &str, errors::AircraftParseError> {
    nom::sequence::terminated(take_until(">"), tag(">"))
        .parse(input)
        .map_err(|e| {
            e.map(|_e: nom::error::Error<&str>| {
                errors::AircraftParseError::InvalidCallsign(errors::APRSParseContext {
                    input: input.to_string(),
                    message: "invalid callsign".to_string(),
                })
            })
        })
}

fn parse_timestamp(
    input: &str,
) -> nom::IResult<&str, chrono::DateTime<chrono::Utc>, errors::AircraftParseError> {
    use nom::Parser;
    let parse_to_datetime = |s: &str| -> Result<chrono::DateTime<chrono::Utc>, String> {
        let now = chrono::Utc::now();
        let h = s[0..2].parse::<u32>().map_err(|_| "invalid hour digits")?;
        let m = s[2..4]
            .parse::<u32>()
            .map_err(|_| "invalid minute digits")?;
        let s = s[4..6]
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
        .map_err(|err| err.map(errors::AircraftParseError::InvalidTimestamp))
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

#[allow(clippy::needless_pass_by_value)]
fn parse_coordinate(
    input: &str,
    coord: Coordinate,
) -> nom::IResult<&str, f64, errors::AircraftParseError> {
    let create_error = |msg: &str| {
        let context = errors::APRSParseContext {
            input: input.to_string(),
            message: msg.to_string(),
        };
        match coord {
            Coordinate::Latitude => errors::AircraftParseError::InvalidLatitude(context),
            Coordinate::Longitude => errors::AircraftParseError::InvalidLongitude(context),
        }
    };
    let suffix_negative = match coord {
        Coordinate::Latitude => "S",
        Coordinate::Longitude => "W",
    };

    let size = match coord {
        Coordinate::Latitude => 2usize,
        Coordinate::Longitude => 3usize,
    };
    let (remainder, degrees_str) = take(size).parse(input).map_err(|e| {
        e.map(|_e: nom::error::Error<&str>| create_error("invalid number of digits for degrees"))
    })?;

    let (remainder, minutes_str) = take(5usize).parse(remainder).map_err(|e| {
        e.map(|_e: nom::error::Error<&str>| create_error("invalid number of digits for minutes"))
    })?;

    let (remainder, matched_suffix) = take(1usize).parse(remainder).map_err(|e| {
        e.map(|_e: nom::error::Error<&str>| create_error("no suffix for coordinate found"))
    })?;

    let degrees_f64 = degrees_str
        .parse::<f64>()
        .map_err(|_| nom::Err::Failure(create_error("could not parse degrees")))?;

    let minutes_f64 = minutes_str
        .parse::<f64>()
        .map_err(|_| nom::Err::Failure(create_error("could not parse minutes")))?;

    let value = convert_latlon_minutes_to_decimals(degrees_f64, minutes_f64);

    if matched_suffix == suffix_negative {
        Ok((remainder, -value))
    } else {
        Ok((remainder, value))
    }
}

pub enum APRSSignalType {
    OGADSB,
}

impl std::str::FromStr for APRSSignalType {
    type Err = errors::AircraftParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OGADSB" => Ok(APRSSignalType::OGADSB),
            _ => Err(errors::AircraftParseError::InvalidAPRSSignalType(
                APRSParseContext {
                    input: s.to_owned(),
                    message: "Invalid APRS Signal Type".to_owned(),
                },
            )),
        }
    }
}
fn parse_aprs_signal_type(
    input: &str,
) -> nom::IResult<&str, APRSSignalType, errors::AircraftParseError> {
    use nom::Parser;
    let parse_to_aprs_signal_type =
        |s: &str| -> Result<APRSSignalType, errors::AircraftParseError> {
            s.parse::<APRSSignalType>()
        };
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(","), tag(",")),
        parse_to_aprs_signal_type,
    )
    .parse(input)
}

fn parse_ground_track(input: &str) -> nom::IResult<&str, f64, errors::AircraftParseError> {
    nom::combinator::map_res(take(3usize), |s: &str| s.parse::<f64>())
        .parse(input)
        .map_err(|e| {
            e.map(|_e: nom::error::Error<&str>| {
                errors::AircraftParseError::InvalidGroundTrack(errors::APRSParseContext {
                    input: input.to_string(),
                    message: "invalid ground track".to_string(),
                })
            })
        })
}
fn parse_ground_speed(input: &str) -> nom::IResult<&str, f64, errors::AircraftParseError> {
    nom::combinator::map_res(take(3usize), |s: &str| s.parse::<f64>())
        .parse(input)
        .map_err(|e| {
            e.map(|_e: nom::error::Error<&str>| {
                errors::AircraftParseError::InvalidGroundSpeed(errors::APRSParseContext {
                    input: input.to_string(),
                    message: "invalid ground speed".to_string(),
                })
            })
        })
}
fn parse_gps_altitude(input: &str) -> nom::IResult<&str, f64, errors::AircraftParseError> {
    nom::combinator::map_res(take(6usize), |s: &str| s.parse::<f64>())
        .parse(input)
        .map_err(|e| {
            e.map(|_e: nom::error::Error<&str>| {
                errors::AircraftParseError::InvalidGPSAltitude(errors::APRSParseContext {
                    input: input.to_string(),
                    message: "invalid gps altitude".to_string(),
                })
            })
        })
}

fn parse_ogn_beacon_id(input: &str) -> nom::IResult<&str, OGNBeaconID, errors::AircraftParseError> {
    // string is of format `idXXYYYYYY`
    let (remainder, id_str) = nom::sequence::preceded(tag("id"), take(8usize))
        .parse(input)
        .map_err(|e| {
            e.map(|_nom_err: nom::error::Error<&str>| {
                errors::AircraftParseError::InvalidOGNBeaconId(errors::APRSParseContext {
                    input: input.to_string(),
                    message: "invalid ogn beacon id format".to_string(),
                })
            })
        })?;

    match id_str.parse::<OGNBeaconID>() {
        Ok(beacon_id) => Ok((remainder, beacon_id)),
        Err(err) => Err(nom::Err::Failure(
            errors::AircraftParseError::InvalidOGNBeaconId(errors::APRSParseContext {
                input: id_str.to_string(),
                message: format!("invalid ogn beacon id: {err}"),
            }),
        )),
    }
}

pub fn build_aircraft_from_string(input: &str) -> Result<Aircraft, errors::AircraftParseError> {
    use nom::{Finish, Parser};

    let (input, callsign) = parse_callsign(input).finish()?;

    let (input, _aprs_packet_type) = parse_aprs_signal_type(input).finish()?;

    let (input, _) = (take_until(":/"), tag(":/"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (input, datetime) = parse_timestamp(input).finish()?;

    let (input, _) = (take_until("h"), tag("h"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let parse_specific_coordinate = |input, coord| parse_coordinate(input, coord);

    let (input, latitude) = parse_specific_coordinate(input, Coordinate::Latitude).finish()?;

    let (input, _) = (take_until("/"), tag("/"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (input, longitude) = parse_specific_coordinate(input, Coordinate::Longitude).finish()?;

    let (input, _) = (take_until("^"), tag("^"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (input, ground_track) = parse_ground_track(input).finish()?;

    let (input, _) = (take_until("/"), tag("/"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (input, ground_speed) = parse_ground_speed(input).finish()?;

    let (input, _) = (take_until("/A="), tag("/A="))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (input, gps_altitude) = parse_gps_altitude(input).finish()?;

    let (input, _) = (take_until(" id"), tag(" "))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

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
#[cfg(test)]
mod test {
    use crate::core::parser::builder::build_aircraft_from_string;
    use crate::core::parser::builder::errors::AircraftParseError;
    use crate::core::parser::builder::{
        Coordinate, parse_aprs_signal_type, parse_callsign, parse_coordinate, parse_gps_altitude,
        parse_ground_speed, parse_ground_track, parse_ogn_beacon_id, parse_timestamp,
    };
    use crate::core::parser::types::{OGNAddressType, OGNAircraftType};
    use crate::core::parser::types::{OGNBeaconID, OGNIDPrefix};
    use crate::core::types::{Aircraft, ICAOAddress};
    use approx::relative_eq;
    use nom::Finish;
    use rstest::rstest;

    const _VALID_APRS_MESSAGE: &str = r"ICA4B37A8>OGADSB,qAS,LELL:/190600h4121.18N\00219.21E^065/430/A=040111 !W29! id214B37A8 -64fpm FL400.00 A1:LUC2M";

    #[test]
    fn when_packet_contains_valid_callsign_identifier_is_correct_then_parsed_callsign_is_correct() {
        let input = "ICA4B37A8>";
        let expected_callsign = "ICA4B37A8";
        match parse_callsign(input).finish() {
            Ok((_, callsign)) => assert_eq!(callsign, expected_callsign),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_packet_contains_invalid_callsign_identifier_then_correct_error_is_returned() {
        let input = "HEADER:/2a0600h";

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
        let input = "OGADSB,";
        match parse_aprs_signal_type(input).finish() {
            Ok(_) => {}
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }

    mod timestamps {
        use super::*;

        #[test]
        fn when_valid_timestamp_digits_parsed_then_correct_datetime_is_returned() {
            let input = "190600h";
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
            let input = "2a0600h";

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
            let input = "260600h";

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
            let input = "4121.18N";
            let expected_latitude = 41.353;
            match parse_coordinate(input, Coordinate::Latitude).finish() {
                Ok((_, latitude)) => assert!(relative_eq!(latitude, expected_latitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }
        #[test]
        fn when_valid_latitude_south_coordinates_then_correct_latitude_is_returned() {
            let input = "4121.18S";
            let expected_latitude = -41.353;
            match parse_coordinate(input, Coordinate::Latitude).finish() {
                Ok((_, latitude)) => assert!(relative_eq!(latitude, expected_latitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_invalid_latitude_coordinates_then_correct_error_returned() {
            let input = "4121.18";

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
            let input = "12219.21E";
            let expected_longitude = 122.320_166_666_666_67;
            match parse_coordinate(input, Coordinate::Longitude).finish() {
                Ok((_, longitude)) => assert!(relative_eq!(longitude, expected_longitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_valid_longitude_west_coordinates_then_correct_longitude_is_returned() {
            let input = "12219.21W";
            let expected_longitude = -122.320_166_666_666_67;
            match parse_coordinate(input, Coordinate::Longitude).finish() {
                Ok((_, longitude)) => assert!(relative_eq!(longitude, expected_longitude)),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_invalid_longitude_coordinates_then_correct_error_returned() {
            let input = r"00219.21";

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
        let input = "123";
        let expected_ground_track = 123.0;
        match parse_ground_track(input).finish() {
            Ok((_, ground_track)) => assert!(relative_eq!(ground_track, expected_ground_track)),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_invalid_ground_track_parsed_then_correct_error_is_returned() {
        let input = "12a";

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
        let input = "123";
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
        let input = "12a";

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
        let input = "002341";
        let expected_gps_altitude = 2341.0;
        match parse_gps_altitude(input).finish() {
            Ok((_, gps_altitude)) => assert!(relative_eq!(gps_altitude, expected_gps_altitude)),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }
    #[test]
    fn when_invalid_gps_alitude_parsed_then_correct_error_is_returned() {
        let input = "12a";

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
        let input = "id253007EE";
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
        let input = "id123";

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
        let input = "id253007EG";

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
    #[rstest]
    #[case(
    r"ICA4400DC>OGADSB,qAS,HLST:/190606h5158.29N/01013.06E^066/488/A=034218 !W10! id254400DC -832fpm FL353.00 A3:EJU47ML",
    "ICA4400DC",
    (19, 6, 6),
    51.9715,
    10.217_666_666_666_666,
    66.0,
    488.0,
    34218.0,
    4_456_668
)]
    #[case(
    r"ICA4B027D>OGADSB,qAS,AVX1224:/190606h4651.87N/00118.95W^356/328/A=012618 !W37! id254B027D -1792fpm FL131.75 A3:EZS14TJ",
    "ICA4B027D",
    (19, 6, 6),
    46.8645,
    -1.315_833_333_333_333_4,
    356.0,
    328.0,
    12618.0,
    4_915_837
)]
    fn test_aircraft_construction(
        #[case] raw_input: &str,
        #[case] expected_callsign: &str,
        #[case] time_tuple: (u32, u32, u32),
        #[case] expected_lat: f64,
        #[case] expected_lon: f64,
        #[case] expected_track: f64,
        #[case] expected_gs: f64,
        #[case] expected_alt: f64,
        #[case] expected_icao_int: u32,
    ) {
        let (h, m, s) = time_tuple;
        let expected_datetime = chrono::Utc::now()
            .date_naive()
            .and_time(chrono::NaiveTime::from_hms_opt(h, m, s).unwrap())
            .and_utc();

        let expected_aircraft = Aircraft {
            callsign: String::from(expected_callsign),
            datetime: expected_datetime,
            latitude: expected_lat,
            longitude: expected_lon,
            ground_track: expected_track,
            ground_speed: expected_gs,
            gps_altitude: expected_alt,
            icao_address: ICAOAddress::new(expected_icao_int).unwrap(),
        };

        let aircraft = build_aircraft_from_string(raw_input).unwrap();
        assert_eq!(aircraft, expected_aircraft);
    }
}
