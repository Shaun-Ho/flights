use ogn_aprs_parser::errors::ICAOAddressError;

#[derive(Debug, thiserror::Error)]
pub enum PacketConversionError {
    #[error("Missing Timestamp")]
    MissingTimestamp,
    #[error("Invalid Timestamp")]
    InvalidTimestamp(#[from] prost_types::TimestampError),
    #[error("Invalid aircraft: {0}")]
    InvalidAircraft(crate::parser::errors::PacketConversionError),
    #[error("Invalid ICAOAddress as key: {0}")]
    InvalidICAOAddressKey(#[from] ICAOAddressError),
}
