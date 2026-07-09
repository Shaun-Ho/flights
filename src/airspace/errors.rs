use chrono::{DateTime, Utc};
use ogn_aprs_parser::errors::ICAOAddressError;

use crate::parser::{Aircraft, errors::AircraftConversionError};

#[derive(Debug)]
pub struct ProblematicAircraftUpdate {
    pub airspace_timestamp: DateTime<Utc>,
    pub aircraft: Aircraft,
}

#[derive(Debug, thiserror::Error)]
pub enum AirspaceError {
    #[error("AirspaceUpdate has timestamp that is behind current airspace")]
    InvalidUpdateTimestamp(DateTime<Utc>),
    #[error("AirspaceUpdate contained aircraft that was newer than current timestamp: {0:?}")]
    ContainedInvalidAircraftTimestamp(Vec<ProblematicAircraftUpdate>),
}
#[derive(Debug, thiserror::Error)]
pub enum PacketConversionError {
    #[error("Missing Timestamp")]
    MissingTimestamp,
    #[error("Invalid Timestamp")]
    InvalidTimestamp(#[from] prost_types::TimestampError),
    #[error("Invalid aircraft: {0}")]
    InvalidAircraft(AircraftConversionError),
    #[error("Invalid ICAOAddress as key: {0}")]
    InvalidICAOAddressKey(#[from] ICAOAddressError),
}
