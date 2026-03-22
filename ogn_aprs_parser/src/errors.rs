use crate::aprs_types::ICAOAddress;

#[derive(Debug, thiserror::Error)]
pub enum AircraftParseError {
    #[error("{0}")]
    ParseError(#[from] APRSMessageParseError),
    #[error("Missing OGN Beacon ID in APRS message")]
    MissingOgnBeaconID,
}

#[derive(Debug, thiserror::Error)]
pub enum APRSMessageParseError {
    #[error("{0}")]
    UnknownError(String),
    #[error("{0}")]
    UnexpectedEndOfMessage(APRSParseContext),
    #[error("{0}")]
    MissingSeparator(APRSParseContext),
    #[error("{0}")]
    InvalidQConstruct(APRSParseContext),
    #[error("{0}")]
    InvalidReceiver(APRSParseContext),
    #[error("{0}")]
    InvalidCallsign(APRSParseContext),
    #[error("{0}")]
    InvalidOGNAprsProtocol(APRSParseContext),
    #[error("{0}")]
    InvalidTimestamp(APRSParseContext),
    #[error("{0}")]
    InvalidLatitude(APRSParseContext),
    #[error("{0}")]
    InvalidLongitude(APRSParseContext),
    #[error("{0}")]
    InvalidGroundTrack(APRSParseContext),
    #[error("{0}")]
    InvalidGroundSpeed(APRSParseContext),
    #[error("{0}")]
    InvalidGPSAltitude(APRSParseContext),
    #[error("{0}")]
    InvalidOGNBeaconId(APRSParseContext),
}

impl nom::error::ParseError<&[u8]> for APRSMessageParseError {
    fn from_error_kind(input: &[u8], _kind: nom::error::ErrorKind) -> Self {
        APRSMessageParseError::UnknownError(String::from_utf8_lossy(input).to_string())
    }

    fn append(_: &[u8], _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
impl nom::error::FromExternalError<&[u8], APRSMessageParseError> for APRSMessageParseError {
    fn from_external_error(
        _input: &[u8],
        _kind: nom::error::ErrorKind,
        e: APRSMessageParseError,
    ) -> Self {
        e
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to parse: {input}, message: {message}")]
pub struct APRSParseContext {
    pub input: String,
    pub message: String,
}

impl nom::error::ParseError<&[u8]> for APRSParseContext {
    fn from_error_kind(input: &[u8], kind: nom::error::ErrorKind) -> Self {
        APRSParseContext {
            input: String::from_utf8_lossy(input).to_string(),
            message: format!("nom error: {kind:?}"),
        }
    }

    fn append(_: &[u8], _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
impl nom::error::FromExternalError<&[u8], String> for APRSParseContext {
    fn from_external_error(input: &[u8], _kind: nom::error::ErrorKind, error: String) -> Self {
        APRSParseContext {
            input: String::from_utf8_lossy(input).to_string(),
            message: error,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum OGNAircraftTypeError {
    #[error("Invalid value: {0}")]
    InvalidEnum(u8),
}

#[derive(Debug, thiserror::Error)]
pub enum ICAOAddressError {
    #[error("Invalid hexadecimal format")]
    InvalidHexFormat,
    #[error("Value 0x{0:X} ({0}) exceeds 24-bit ICAO address limit (0x{max:X})", max = ICAOAddress::MAX_VALUE)]
    InvalidAddress(u32),
}

#[derive(Debug, thiserror::Error)]
pub enum OGNBeaconIDError {
    #[error("{0}")]
    OGNIDPrefixError(OGNIDPrefixError),
    #[error("{0}")]
    ICAOAddressError(ICAOAddressError),
    #[error("Invalid beacon format: {0}")]
    InvalidOGNBeaconFormat(String),
}

#[derive(Debug, thiserror::Error)]
pub enum OGNAddressTypeError {
    #[error("Invalid address format: {0}")]
    InvalidAddressType(u8),
}

#[derive(Debug, thiserror::Error)]
pub enum OGNIDPrefixError {
    #[error("Invalid hexadecimal format")]
    HexFormat,
    #[error("{0}")]
    AircraftType(OGNAircraftTypeError),
    #[error("{0}")]
    AddressType(OGNAddressTypeError),
}
