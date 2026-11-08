use ogn_aprs_parser::errors::ICAOAddressError;

#[derive(Debug, thiserror::Error)]
pub enum AircraftConversionError {
    #[error("Invalid datetime")]
    InvalidTimestamp(#[from] prost_types::TimestampError),
    #[error("Missing datetime field")]
    MissingDatetime,
    #[error("Invalid icao_address field")]
    InvalidICAOAddress(#[from] ICAOAddressError),
}
