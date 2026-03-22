use super::errors::{APRSMessageParseError, APRSParseContext};
use crate::{
    aprs_types::{OGNBeaconID, OgnAprsProtocol},
    errors::AircraftParseError,
};

use nom::{
    Parser,
    bytes::complete::{tag, take, take_until},
};

#[derive(Debug, PartialEq, Clone)]
pub struct AircraftBeacon {
    pub callsign: String,
    pub q_construct: String,
    pub receiver: String,
    pub ogn_aprs_protocol: OgnAprsProtocol,
    pub time: chrono::NaiveTime,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
    pub ogn_beacon_id: OGNBeaconID,
}

pub fn parse_ogn_aprs_aircraft_beacon(input: &[u8]) -> Result<AircraftBeacon, AircraftParseError> {
    use nom::Finish;

    let (input, callsign) = parse_callsign(input).finish()?;

    let (input, ogn_aprs_protocol) = parse_aprs_signal_type(input).finish()?;

    let (input, q_construct) = parse_q_construct(input).finish()?;

    let (input, receiver) = parse_receiver(input).finish()?;

    // Following fields are from the position 'block'
    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(APRSMessageParseError::UnexpectedEndOfMessage)?;

    let (input, time) = parse_naive_time(input).finish()?;

    let parse_specific_coordinate = |input, coord| parse_coordinate(input, coord);

    let (input, latitude) = parse_specific_coordinate(input, Coordinate::Latitude).finish()?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(APRSMessageParseError::UnexpectedEndOfMessage)?;

    let (input, longitude) = parse_specific_coordinate(input, Coordinate::Longitude).finish()?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(APRSMessageParseError::UnexpectedEndOfMessage)?;

    let (input, ground_track) = parse_ground_track(input).finish()?;

    let (input, _) = take(1usize)
        .parse(input)
        .finish()
        .map_err(APRSMessageParseError::UnexpectedEndOfMessage)?;

    let (input, ground_speed) = parse_ground_speed(input).finish()?;

    let (input, _) = (take_until(b"A=".as_slice()), tag(b"A=".as_slice()))
        .parse(input)
        .finish()
        .map_err(APRSMessageParseError::UnexpectedEndOfMessage)?;

    let (input, gps_altitude) = parse_gps_altitude(input).finish()?;

    // following that, tokens are separated by whitespace
    let mut parsed_ogn_beacon_id = None;
    for token in input.split(|&b| b == b' ').filter(|t| !t.is_empty()) {
        if token.starts_with(b"id") {
            let (_, ogn_beacon_id) = parse_ogn_beacon_id(token).finish()?;
            parsed_ogn_beacon_id = Some(ogn_beacon_id);
        }
    }
    let ogn_beacon_id = parsed_ogn_beacon_id.ok_or(AircraftParseError::MissingOgnBeaconID)?;

    Ok(AircraftBeacon {
        callsign: callsign.to_string(),
        q_construct: q_construct.to_string(),
        receiver: receiver.to_string(),
        ogn_aprs_protocol,
        time,
        latitude,
        longitude,
        ground_track,
        ground_speed,
        gps_altitude,
        ogn_beacon_id,
    })
}

fn parse_q_construct(input: &[u8]) -> nom::IResult<&[u8], &str, APRSMessageParseError> {
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(b",".as_slice()), tag(b",".as_slice())),
        std::str::from_utf8,
    )
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            APRSMessageParseError::InvalidQConstruct(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid q construct".to_string(),
            })
        })
    })
}

fn parse_receiver(input: &[u8]) -> nom::IResult<&[u8], &str, APRSMessageParseError> {
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(b":".as_slice()), tag(b":".as_slice())),
        std::str::from_utf8,
    )
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            APRSMessageParseError::InvalidReceiver(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid receiver".to_string(),
            })
        })
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

fn parse_callsign(input: &[u8]) -> nom::IResult<&[u8], &str, APRSMessageParseError> {
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(b">".as_slice()), tag(b">".as_slice())),
        std::str::from_utf8,
    )
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            APRSMessageParseError::InvalidCallsign(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid callsign".to_string(),
            })
        })
    })
}

fn parse_naive_time(input: &[u8]) -> nom::IResult<&[u8], chrono::NaiveTime, APRSMessageParseError> {
    let parse_to_datetime = |s: &[u8]| -> Result<chrono::NaiveTime, String> {
        let s_str = std::str::from_utf8(s).map_err(|_| "invalid utf8")?;
        let h = s_str[0..2]
            .parse::<u32>()
            .map_err(|_| "invalid hour digits")?;
        let m = s_str[2..4]
            .parse::<u32>()
            .map_err(|_| "invalid minute digits")?;
        let s = s_str[4..6]
            .parse::<u32>()
            .map_err(|_| "invalid second digits")?;

        let naive_time = chrono::NaiveTime::from_hms_opt(h, m, s).ok_or("invalid time")?;
        Ok(naive_time)
    };

    nom::combinator::map_res(
        nom::sequence::terminated(take(6usize), take(1usize)),
        parse_to_datetime,
    )
    .parse(input)
    .map_err(|err| err.map(APRSMessageParseError::InvalidTimestamp))
}

#[allow(clippy::needless_pass_by_value)]
fn parse_coordinate(
    input: &[u8],
    coord: Coordinate,
) -> nom::IResult<&[u8], f64, APRSMessageParseError> {
    let create_error = |msg: &str| {
        let context = APRSParseContext {
            input: String::from_utf8_lossy(input).to_string(),
            message: msg.to_string(),
        };
        match coord {
            Coordinate::Latitude => APRSMessageParseError::InvalidLatitude(context),
            Coordinate::Longitude => APRSMessageParseError::InvalidLongitude(context),
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

fn parse_aprs_signal_type(
    input: &[u8],
) -> nom::IResult<&[u8], OgnAprsProtocol, APRSMessageParseError> {
    let parse_to_aprs_signal_type = |s: &[u8]| -> Result<OgnAprsProtocol, APRSMessageParseError> {
        OgnAprsProtocol::parse_protocol(std::str::from_utf8(s).unwrap_or(""))
    };
    nom::combinator::map_res(
        nom::sequence::terminated(take_until(b",".as_slice()), tag(b",".as_slice())),
        parse_to_aprs_signal_type,
    )
    .parse(input)
}

fn parse_ground_track(input: &[u8]) -> nom::IResult<&[u8], f64, APRSMessageParseError> {
    nom::combinator::map_res(take(3usize), |s: &[u8]| {
        std::str::from_utf8(s).unwrap_or("").parse::<f64>()
    })
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            APRSMessageParseError::InvalidGroundTrack(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid ground track".to_string(),
            })
        })
    })
}

fn parse_ground_speed(input: &[u8]) -> nom::IResult<&[u8], f64, APRSMessageParseError> {
    nom::combinator::map_res(take(3usize), |s: &[u8]| {
        std::str::from_utf8(s).unwrap_or("").parse::<f64>()
    })
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            APRSMessageParseError::InvalidGroundSpeed(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid ground speed".to_string(),
            })
        })
    })
}

fn parse_gps_altitude(input: &[u8]) -> nom::IResult<&[u8], f64, APRSMessageParseError> {
    nom::combinator::map_res(take(6usize), |s: &[u8]| {
        std::str::from_utf8(s).unwrap_or("").parse::<f64>()
    })
    .parse(input)
    .map_err(|e| {
        e.map(|_e: nom::error::Error<&[u8]>| {
            APRSMessageParseError::InvalidGPSAltitude(APRSParseContext {
                input: String::from_utf8_lossy(input).to_string(),
                message: "invalid gps altitude".to_string(),
            })
        })
    })
}

fn parse_ogn_beacon_id(input: &[u8]) -> nom::IResult<&[u8], OGNBeaconID, APRSMessageParseError> {
    // string is of format `idXXYYYYYY`
    let (remainder, id_bytes) = nom::sequence::preceded(tag(b"id".as_slice()), take(8usize))
        .parse(input)
        .map_err(|e| {
            e.map(|_nom_err: nom::error::Error<&[u8]>| {
                APRSMessageParseError::InvalidOGNBeaconId(APRSParseContext {
                    input: String::from_utf8_lossy(input).to_string(),
                    message: "invalid ogn beacon id format".to_string(),
                })
            })
        })?;

    let id_str = std::str::from_utf8(id_bytes).unwrap_or("");
    match id_str.parse::<OGNBeaconID>() {
        Ok(beacon_id) => Ok((remainder, beacon_id)),
        Err(err) => Err(nom::Err::Failure(
            APRSMessageParseError::InvalidOGNBeaconId(APRSParseContext {
                input: id_str.to_string(),
                message: format!("invalid ogn beacon id: {err}"),
            }),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::aprs_types::{OGNBeaconID, OgnAprsProtocol};
    use crate::parse::{APRSMessageParseError, AircraftBeacon};
    use crate::parse::{
        Coordinate, parse_aprs_signal_type, parse_callsign, parse_coordinate, parse_gps_altitude,
        parse_ground_speed, parse_ground_track, parse_naive_time, parse_ogn_beacon_id,
    };
    use crate::parse::{parse_ogn_aprs_aircraft_beacon, parse_q_construct, parse_receiver};
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

            Err(APRSMessageParseError::InvalidCallsign(info)) => {
                assert_eq!(info.input, "HEADER:/2a0600h");
                assert_eq!(info.message, "invalid callsign");
            }

            Err(other) => panic!("Expected InvalidCallsign, got: {other}"),
        }
    }

    #[test]
    fn when_packet_contains_valid_q_construct_identifier_is_correct_then_parsed_q_construct_is_correct()
     {
        let input = b"qAS,".as_slice();
        let expected_q_construct = "qAS";
        match parse_q_construct(input).finish() {
            Ok((_, callsign)) => assert_eq!(callsign, expected_q_construct),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }

    #[test]
    fn when_packet_contains_invalid_q_construct_identifier_then_correct_error_is_returned() {
        let input = b"qAS.".as_slice();

        match parse_q_construct(input).finish() {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(APRSMessageParseError::InvalidQConstruct(info)) => {
                assert_eq!(info.input, "qAS.");
                assert_eq!(info.message, "invalid q construct");
            }

            Err(other) => panic!("Expected InvalidQConstruct, got: {other}"),
        }
    }

    #[test]
    fn when_packet_contains_valid_receiver_identifier_is_correct_then_parsed_receiver_is_correct() {
        let input = b"ABC:".as_slice();
        let expected_callsign = "ABC";
        match parse_receiver(input).finish() {
            Ok((_, callsign)) => assert_eq!(callsign, expected_callsign),
            Err(err) => panic!("Expected no errors. {err}"),
        }
    }

    #[test]
    fn when_packet_contains_invalid_receiver_identifier_then_correct_error_is_returned() {
        let input = b"ABC.".as_slice();

        match parse_receiver(input).finish() {
            Ok(_) => panic!("Expected an error, but got an Aircraft"),

            Err(APRSMessageParseError::InvalidReceiver(info)) => {
                assert_eq!(info.input, "ABC.");
                assert_eq!(info.message, "invalid receiver");
            }

            Err(other) => panic!("Expected InvalidReceiver, got: {other}"),
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
            let expected_time = chrono::NaiveTime::from_hms_opt(19, 0o6, 00).unwrap();

            match parse_naive_time(input) {
                Ok((_, datetime)) => assert_eq!(datetime, expected_time),
                Err(err) => panic!("Expected no errors, received:  {err}"),
            }
        }
        #[test]
        fn when_invalid_timestamp_digits_parsed_then_error_shows_correct_digit_error() {
            let input = b"2a0600h".as_slice();

            match parse_naive_time(input).finish() {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(APRSMessageParseError::InvalidTimestamp(info)) => {
                    assert_eq!(info.input, "2a0600h");
                    assert_eq!(info.message, "invalid hour digits");
                }

                Err(other) => panic!("Expected InvalidTimestamp, got: {other}"),
            }
        }
        #[test]
        fn when_invalid_timestamp_parsed_then_error_shows_correct_time_conversion_error() {
            let input = b"260600h".as_slice();

            match parse_naive_time(input).finish() {
                Ok(_) => panic!("Expected an error, but got an Aircraft"),

                Err(APRSMessageParseError::InvalidTimestamp(info)) => {
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

                Err(APRSMessageParseError::InvalidLatitude(info)) => {
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

                Err(APRSMessageParseError::InvalidLongitude(info)) => {
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

            Err(APRSMessageParseError::InvalidGroundTrack(info)) => {
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

            Err(APRSMessageParseError::InvalidGroundSpeed(info)) => {
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

            Err(APRSMessageParseError::InvalidGPSAltitude(info)) => {
                assert_eq!(info.input, "12a");
                assert_eq!(info.message, "invalid gps altitude");
            }

            Err(other) => panic!("Expected InvalidGPSAltitude, got: {other}"),
        }
    }
    #[test]
    fn when_when_valid_ogn_beacon_id_is_given_then_correct_ogn_beacon_id_returned() {
        let input = b"id253007EE".as_slice();
        let expected_ogn_beacon_id = OGNBeaconID::from_str("253007EE").unwrap();
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

            Err(APRSMessageParseError::InvalidOGNBeaconId(info)) => {
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

            Err(APRSMessageParseError::InvalidOGNBeaconId(info)) => {
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
        q_construct: &'static str,
        ogn_aprs_protocol: &'static str,
        receiver: &'static str,
        hms: (u32, u32, u32),
        lat_lon: (f64, f64),
        specs: (f64, f64, f64), // track, ground_speed, alt
        ogn_beacon_id: &'static str,
    }

    impl AircraftComparison {
        fn to_comparison_tuple(&self) -> (&'static [u8], AircraftBeacon) {
            let (h, m, s) = self.hms;
            let (lat, lon) = self.lat_lon;
            let (track, gs, alt) = self.specs;

            let aircraft = AircraftBeacon {
                callsign: self.expected_callsign.to_string(),
                q_construct: self.q_construct.to_string(),
                ogn_aprs_protocol: OgnAprsProtocol::from_str(self.ogn_aprs_protocol).unwrap(),
                receiver: self.receiver.to_string(),
                time: chrono::NaiveTime::from_hms_opt(h, m, s).unwrap(),
                latitude: lat,
                longitude: lon,
                ground_track: track,
                ground_speed: gs,
                gps_altitude: alt,
                ogn_beacon_id: OGNBeaconID::from_str(self.ogn_beacon_id).unwrap(),
            };
            (self.raw, aircraft)
        }
    }

    #[rstest::rstest]
    #[case::case_1(AircraftComparison {
        raw: b"ICA4400DC>OGADSB,qAS,HLST:/190606h5158.29N/01013.06E^066/488/A=034218 !W10! id254400DC -832fpm FL353.00 A3:EJU47ML".as_slice(),
        expected_callsign: "ICA4400DC",
        ogn_aprs_protocol: "OGADSB",
        q_construct: "qAS",
        ogn_beacon_id:"254400DC",
        receiver:"HLST",
        hms: (19, 6, 6),
        lat_lon: (51.9715, 10.217_666_666_666_666),
        specs: (66.0, 488.0, 34218.0),
    })]
    #[case::case_2(AircraftComparison {
        raw: b"ICA4B027D>OGADSB,qAS,AVX1224:/190606h4651.87N/00118.95W^356/328/A=012618 !W37! id254B027D -1792fpm FL131.75 A3:EZS14TJ".as_slice(),
        expected_callsign: "ICA4B027D",
        ogn_aprs_protocol: "OGADSB",
        q_construct: "qAS",
        ogn_beacon_id: "254B027D",
        receiver: "AVX1224",
        hms: (19, 6, 6),
        lat_lon: (46.8645, -1.315_833_333_333_333_4),
        specs: (356.0, 328.0, 12618.0),
    })]
    fn test_when_valid_aprs_message_received_then_correct_aircraft_beacon_is_constructed(
        #[case] scenario: AircraftComparison,
    ) {
        let (raw_input, expected) = scenario.to_comparison_tuple();
        let result = parse_ogn_aprs_aircraft_beacon(raw_input).unwrap();

        assert_eq!(result, expected);
    }

    mod ogn_aprs_protocol {
        use super::*;

        #[rstest::rstest]
        #[case(br"ICA4CA1FF>OGADSB,qAS,LEMDadsb:/140833h4044.38N\00356.29W^024/426/A=040000 id254CA1FF +000fpm  fnRYR6ZM".as_slice())]
        #[case(br"ICA4CA4EB>OGADSB,qAS,LEMDadsb:/142346h4034.03N\00315.64W^008/370/A=038000 id254CA4EB +000fpm  0.0rot fnRYR4057  regEI-DPG modelB738".as_slice())]
        #[case(br"ICAA8CBA8>OGFLR,qAS,MontCAIO:/231150z4512.12N\01059.03E^192/106/A=009519 !W20! id21A8CBA8 -039fpm +0.0rot 3.5dB 2e -8.7kHz gps1x2 s6.09 h43 rDF0267".as_slice())]
        #[case(br"FLR200295>OGFLR,qAS,TT:/071005h4613.92N/01427.53Eg000/000/A=001313 !W00! id1E200295 +000fpm +0.0rot 37.0dB -1.8kHz gps3x5".as_slice())]
        #[case(br"SKY3E5906>OGNSKY,qAS,SafeSky:/072449h5103.95N/00524.50E'193/034/A=001250 !W65! id1C3E5906 +000fpm gps4x1".as_slice())]
        fn test_support_for_different_aprs_protocol_types(#[case] input: &[u8]) {
            parse_ogn_aprs_aircraft_beacon(input).unwrap();
        }
    }
}
