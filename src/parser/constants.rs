pub const CALLSIGN_DELIMETER: &str = ">";
pub const HEADER_BODY_DELIMITER: &str = ":/";
pub static GPS_DATA_REGEX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(
        r"^(?P<time>\d{6})h(?P<latitude_degrees>\d{2})(?P<latitude_minutes>\d{2}\.\d{2})N[\\\/](?P<longitude_degrees>\d{3})(?P<longitude_minutes>\d{2}\.\d{2})E\^(?P<ground_track>\d{3})\/(?P<ground_speed>\d{3})\/A=(?P<gps_altitude>\d{6})"
    ).unwrap()
});
pub static OGN_BEACON_ID_REGEX: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"id(?P<ogn_beacon_id>[0-9A-Fa-f]{8})").unwrap()
    });
