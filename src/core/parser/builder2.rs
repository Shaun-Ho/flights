use super::errors;
use crate::core::types::{Aircraft, ICAOAddress};

use nom::{
    Parser,
    bytes::complete::{tag, take, take_until},
};
fn parse_callsign(input: &str) -> nom::IResult<&str, &str, errors::APRSParseContext> {
    nom::sequence::terminated(take_until(">"), tag(">"))
        .parse(input)
        .map_err(|err| {
            err.map(|e: nom::error::Error<&str>| errors::APRSParseContext {
                input: e.input.to_string(),
                message: "invalid callsign".to_string(),
            })
        })
}

fn parse_timestamp(
    input: &str,
) -> nom::IResult<&str, chrono::DateTime<chrono::Utc>, errors::APRSParseContext> {
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

    nom::combinator::map_res(take(6usize), parse_to_datetime).parse(input)
}

fn parse_latitude(input: &str) -> nom::IResult<&str, f64, errors::APRSParseContext> {
    let parse_to_f64 = |s: &str| -> Result<f64, std::num::ParseFloatError> { s.parse::<f64>() };
    nom::combinator::map_res(
        nom::sequence::terminated(take_until("N"), tag("N")),
        parse_to_f64,
    )
    .parse(input)
    .map_err(|err| {
        err.map(|e: nom::error::Error<&str>| errors::APRSParseContext {
            input: e.input.to_string(),

            message: "invalid latitude".to_string(),
        })
    })
}

pub fn build_aircraft_from_string(input: &str) -> Result<Aircraft, errors::AircraftParseError> {
    use nom::{Finish, Parser};

    let (input, callsign) = parse_callsign(input)
        .finish()
        .map_err(errors::AircraftParseError::InvalidCallsign)?;

    let (input, _) = (take_until(":/"), tag(":/"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (input, datetime) = parse_timestamp
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::InvalidTimestamp)?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::IncorrectSeparator)?;

    let (_input, latitude) = parse_latitude
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::InvalidLatitude)?;
    println!("{latitude}");

    Ok(Aircraft {
        callsign: callsign.to_string(),
        datetime,
        latitude,
        longitude: 1.0,
        icao_address: ICAOAddress::new(0x407_F7A).unwrap(),
        ground_track: 1.0,
        ground_speed: 1.0,
        gps_altitude: 1.0,
    })
}
#[cfg(test)]
mod test {
    use crate::core::parser::builder2::build_aircraft_from_string;
    use crate::core::parser::builder2::errors::AircraftParseError;

    const VALID_APRS_MESSAGE: &str = r"ICA4B37A8>OGADSB,qAS,LELL:/190600h4121.18N\00219.21E^065/430/A=040111 !W29! id214B37A8 -64fpm FL400.00 A1:LUC2M";

    #[test]
    fn when_packet_contains_valid_callsign_identifier_is_correct_then_parsed_callsign_is_correct() {
        let expected_callsign = "ICA4B37A8";
        match build_aircraft_from_string(VALID_APRS_MESSAGE) {
            Ok(aircraft) => assert_eq!(aircraft.callsign, expected_callsign),
            Err(_) => panic!("Expected no errors."),
        }
    }
    #[test]
    fn when_packet_contains_invalid_callsign_identifier_then_correct_error_is_returned() {
        let input = "HEADER:/2a0600h";

        match build_aircraft_from_string(input) {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(AircraftParseError::InvalidCallsign(info)) => {
                assert_eq!(info.input, "HEADER:/2a0600h");
                assert_eq!(info.message, "invalid callsign");
            }

            Err(other) => panic!("Expected InvalidCallsign, got: {other}"),
        }
    }
    mod timestamps {
        use super::*;

        #[test]
        fn when_valid_timestamp_digits_parsed_then_correct_datetime_is_returned() {
            let now = chrono::Utc::now();
            let expected_datetime = now
                .date_naive()
                .and_time(chrono::NaiveTime::from_hms_opt(19, 06, 00).unwrap())
                .and_utc();

            match build_aircraft_from_string(VALID_APRS_MESSAGE) {
                Ok(aircraft) => assert_eq!(aircraft.datetime, expected_datetime),
                Err(err) => panic!("Expected no errors, received:  {err}"),
            }
        }
        #[test]
        fn when_invalid_timestamp_digits_parsed_then_error_shows_correct_digit_error() {
            let input = "ICA4B37A8>OGADSB,qAS,LELL:/2a0600h";

            match build_aircraft_from_string(input) {
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
            let input = "ICA4B37A8>OGADSB,qAS,LELL:/260600h";

            match build_aircraft_from_string(input) {
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
            let expected_latitude = 4121.18;
            match build_aircraft_from_string(VALID_APRS_MESSAGE) {
                Ok(aircraft) => assert_eq!(aircraft.latitude, expected_latitude),
                Err(e) => panic!("Expected no errors. {e}"),
            }
        }

        #[test]
        fn when_invalid_latitude_coordinates_then_correct_error_returned() {
            let input = r"ICA4B37A8>OGADSB,qAS,LELL:/190600h4121.18";

            match build_aircraft_from_string(input) {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(AircraftParseError::InvalidLatitude(info)) => {
                    assert_eq!(info.input, "4121.18");
                    assert_eq!(info.message, "invalid latitude");
                }

                Err(other) => panic!("Expected InvalidLatitude, got: {other}"),
            }
        }
    }
}
