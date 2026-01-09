pub const CALLSIGN_DELITMETER: &str = ">";
pub const HEADER_BODY_DELIMITER: &str = ":/";

pub const TIME: &str = "time";
pub const LATITUDE_DEGREES: &str = "latitude_degrees";
pub const LONGITUDE_DEGREES: &str = "longitude_degrees";
pub const LATITUDE_MINUTES: &str = "latitude_minutes";
pub const LONGITUDE_MINUTES: &str = "longitude_minutes";
pub const GROUND_TRACK: &str = "ground_track";
pub const GROUND_SPEED: &str = "ground_speed";
pub const GPS_ALTITUDE: &str = "gps_altitude";
pub const OGN_BEACON_ID: &str = "ogn_beacon_id";

pub static GPS_DATA_REGEX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    let regex_string = format!(
        r"^(?P<{TIME}>\d{{6}})h(?P<{LATITUDE_DEGREES}>\d{{2}})(?P<{LATITUDE_MINUTES}>\d{{2}}\.\d{{2}})N[\\\/](?P<{LONGITUDE_DEGREES}>\d{{3}})(?P<{LONGITUDE_MINUTES}>\d{{2}}\.\d{{2}})E\^(?P<{GROUND_TRACK}>\d{{3}})\/(?P<{GROUND_SPEED}>\d{{3}})\/A=(?P<{GPS_ALTITUDE}>\d{{6}})",
    );
    regex::Regex::new(&regex_string).unwrap()
});
pub static OGN_BEACON_ID_REGEX: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| {
        let regex_string = format!(r"id(?P<{OGN_BEACON_ID}>[0-9A-Fa-f]{{8}})");
        regex::Regex::new(&regex_string).unwrap()
    });
