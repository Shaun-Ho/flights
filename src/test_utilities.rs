use std::{io::Write, time::SystemTime};

use ogn_aprs_parser::ICAOAddress;

use crate::core::parser::Aircraft;

pub struct TestPath {
    _guard: tempfile::TempDir,
    pub path: std::path::PathBuf,
}

#[rstest::fixture]
pub fn test_path() -> TestPath {
    let guard = tempfile::tempdir().expect("Failed to create temporary directory");
    let path = guard.path().to_path_buf();
    TestPath {
        _guard: guard,
        path,
    }
}

#[rstest::fixture]
pub fn test_data_path() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data");
    path
}

pub fn create_dummy_aircraft_at_time(
    datetime: chrono::DateTime<chrono::Utc>,
    icao_address: ICAOAddress,
) -> Aircraft {
    Aircraft {
        timestamp: SystemTime::now(),
        callsign: String::from("dummy"),
        icao_address,
        recorded_datetime: datetime,
        latitude: 0.0,
        longitude: 0.0,
        ground_track: 0.0,
        ground_speed: 0.0,
        gps_altitude: 0.0,
    }
}

pub fn write_pb_message_to_disk<M: prost::Message>(
    writer: &mut std::io::BufWriter<std::fs::File>,
    pb_message: &M,
) -> Result<(), std::io::Error> {
    let mut buf = Vec::new();
    let () = pb_message.encode_length_delimited(&mut buf)?;
    writer.write_all(&buf)
}
