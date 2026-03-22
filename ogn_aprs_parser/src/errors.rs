#[derive(Debug, thiserror::Error)]
pub enum AircraftParseError {
    #[error("{0}")]
    ParseError(#[from] APRSMessageParseError),
    #[error("Missing OGN Beacon ID in APRS message")]
    MissingOgnBeaconID,
}

#[derive(Debug, thiserror::Error)]
pub enum APRSMessageParseError {
    UnknownError(String),
    UnexpectedEndOfMessage(APRSParseContext),
    MissingSeparator(APRSParseContext),
    InvalidCallsign(APRSParseContext),
    InvalidOGNAprsProtocol(APRSParseContext),
    InvalidTimestamp(APRSParseContext),
    InvalidLatitude(APRSParseContext),
    InvalidLongitude(APRSParseContext),
    InvalidGroundTrack(APRSParseContext),
    InvalidGroundSpeed(APRSParseContext),
    InvalidGPSAltitude(APRSParseContext),
    InvalidOGNBeaconId(APRSParseContext),
}

impl std::fmt::Display for APRSMessageParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            APRSMessageParseError::UnknownError(e) => write!(f, "Failed to parse: {e}"),
            APRSMessageParseError::MissingSeparator(e)
            | APRSMessageParseError::UnexpectedEndOfMessage(e)
            | APRSMessageParseError::InvalidCallsign(e)
            | APRSMessageParseError::InvalidOGNAprsProtocol(e)
            | APRSMessageParseError::InvalidTimestamp(e)
            | APRSMessageParseError::InvalidLatitude(e)
            | APRSMessageParseError::InvalidLongitude(e)
            | APRSMessageParseError::InvalidGroundTrack(e)
            | APRSMessageParseError::InvalidGPSAltitude(e)
            | APRSMessageParseError::InvalidGroundSpeed(e)
            | APRSMessageParseError::InvalidOGNBeaconId(e) => write!(f, "{e}"),
        }
    }
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

#[derive(Debug)]
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
impl std::fmt::Display for APRSParseContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}, {1}", self.input, self.message)
    }
}
