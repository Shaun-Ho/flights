use super::errors;
use crate::core::types::{Aircraft, ICAOAddress};

use nom::{
    bytes::complete::{tag, take, take_until},
    combinator::map_res,
};

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

    map_res(take(6usize), parse_to_datetime).parse(input)
}

pub fn build_aircraft_from_string(input: &str) -> Result<Aircraft, errors::AircraftParseError> {
    use nom::{Finish, Parser};

    let (input, _) = (take_until(":/"), tag(":/"))
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::HeaderMissing)?;

    let (_, datetime) = parse_timestamp
        .parse(input)
        .finish()
        .map_err(errors::AircraftParseError::InvalidTimestamp)?;

    Ok(Aircraft {
        callsign: "Unknown".to_string(),
        datetime,
        latitude: 1.0,
        longitude: 1.0,
        icao_address: ICAOAddress::new(0x407_F7A).unwrap(),
        ground_track: 1.0,
        ground_speed: 1.0,
        gps_altitude: 1.0,
    })
}
#[cfg(test)]
mod test {
    mod timestamps {
        use crate::core::parser::builder2::build_aircraft_from_string;
        use crate::core::parser::builder2::errors::AircraftParseError;

        #[test]
        fn when_valid_timestamp_digits_parsed_then_correct_datetime_is_returned() {
            let input = "ICA4B37A8>OGADSB,qAS,LELL:/190600h4121.18N\00219.21E^065/430/A=040111 !W29! id214B37A8 -64fpm FL400.00 A1:LUC2M";
            let now = chrono::Utc::now();
            let expected_datetime = now
                .date_naive()
                .and_time(chrono::NaiveTime::from_hms_opt(19, 06, 00).unwrap())
                .and_utc();

            match build_aircraft_from_string(input) {
                Ok(aircraft) => assert_eq!(aircraft.datetime, expected_datetime),
                Err(_) => panic!("Expected no errors."),
            }
        }
        #[test]
        fn when_invalid_timestamp_digits_parsed_then_error_shows_correct_digit_error() {
            let input = "HEADER:/2a0600h";

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
            let input = "HEADER:/260600h";

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
}
