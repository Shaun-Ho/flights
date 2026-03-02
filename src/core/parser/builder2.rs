use super::errors;
use crate::core::{
    parser::errors::APRSParseContext,
    types::{Aircraft, ICAOAddress},
};

use nom::{
    Parser,
    bytes::complete::{tag, take, take_until},
};
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
    let create_error = || {
        let context = errors::APRSParseContext {
            input: input.to_string(),
            message: format!("invalid {coord}"),
        };
        match coord {
            Coordinate::Latitude => errors::AircraftParseError::InvalidLatitude(context),
            Coordinate::Longitude => errors::AircraftParseError::InvalidLongitude(context),
        }
    };
    let (suffix_positive, suffix_negative) = match coord {
        Coordinate::Latitude => ("N", "S"),
        Coordinate::Longitude => ("E", "W"),
    };

    let mut parser = nom::branch::alt((
        nom::sequence::pair(take_until(suffix_positive), tag(suffix_positive)),
        nom::sequence::pair(take_until(suffix_negative), tag(suffix_negative)),
    ));

    let (remainder, (number_str, matched_suffix)) = parser
        .parse(input)
        .map_err(|e| e.map(|_e: nom::error::Error<&str>| create_error()))?;

    let mut value = number_str
        .parse::<f64>()
        .map_err(|_| nom::Err::Failure(create_error()))?;

    if matched_suffix == suffix_negative {
        value = -value;
    }
    Ok((remainder, value))
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

    let (_input, ground_track) = parse_ground_track(input).finish()?;

    Ok(Aircraft {
        callsign: callsign.to_string(),
        datetime,
        latitude,
        longitude,
        ground_track,
        icao_address: ICAOAddress::new(0x407_F7A).unwrap(),
        ground_speed: 1.0,
        gps_altitude: 1.0,
    })
}
#[cfg(test)]
mod test {
    use crate::core::parser::builder2::errors::AircraftParseError;
    use crate::core::parser::builder2::{
        Coordinate, parse_aprs_signal_type, parse_callsign, parse_coordinate, parse_ground_track,
        parse_timestamp,
    };
    use nom::Finish;

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
                .and_time(chrono::NaiveTime::from_hms_opt(19, 06, 00).unwrap())
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
        fn when_valid_latitude_coordinates_then_correct_latitude_is_returned() {
            let input = "4121.18N";
            let expected_latitude = 4121.18;
            match parse_coordinate(input, Coordinate::Latitude).finish() {
                Ok((_, latitude)) => assert_eq!(latitude, expected_latitude),
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
                    assert_eq!(info.message, "invalid latitude");
                }

                Err(other) => panic!("Expected InvalidLatitude, got: {other}"),
            }
        }

        #[test]
        fn when_valid_longitude_coordinates_then_correct_longitude_is_returned() {
            let input = "219.21E";
            let expected_longitude = 219.21;
            match parse_coordinate(input, Coordinate::Longitude).finish() {
                Ok((_, longitude)) => assert_eq!(longitude, expected_longitude),
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
                    assert_eq!(info.message, "invalid longitude");
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
            Ok((_, ground_track)) => assert_eq!(ground_track, expected_ground_track),
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
}
