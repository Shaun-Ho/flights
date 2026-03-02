pub enum AircraftParseError {
    UnknownError(String),
    IncorrectSeparator(APRSParseContext),
    InvalidAPRSSignalType(APRSParseContext),
    InvalidTimestamp(APRSParseContext),
    InvalidCallsign(APRSParseContext),
    InvalidLatitude(APRSParseContext),
    InvalidLongitude(APRSParseContext),
}

impl std::fmt::Display for AircraftParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AircraftParseError::UnknownError(e) => write!(f, "Failed to parse: {e}"),
            AircraftParseError::IncorrectSeparator(e)
            | AircraftParseError::InvalidAPRSSignalType(e)
            | AircraftParseError::InvalidTimestamp(e)
            | AircraftParseError::InvalidLatitude(e)
            | AircraftParseError::InvalidLongitude(e)
            | AircraftParseError::InvalidCallsign(e) => write!(f, "{e}"),
        }
    }
}
impl nom::error::ParseError<&str> for AircraftParseError {
    fn from_error_kind(input: &str, _kind: nom::error::ErrorKind) -> Self {
        AircraftParseError::UnknownError(input.to_string())
    }

    fn append(_: &str, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
impl nom::error::FromExternalError<&str, AircraftParseError> for AircraftParseError {
    fn from_external_error(
        _input: &str,
        _kind: nom::error::ErrorKind,
        e: AircraftParseError,
    ) -> Self {
        e
    }
}

pub struct APRSParseContext {
    pub input: String,
    pub message: String,
}

impl nom::error::ParseError<&str> for APRSParseContext {
    fn from_error_kind(input: &str, kind: nom::error::ErrorKind) -> Self {
        APRSParseContext {
            input: input.to_string(),
            message: format!("nom error: {kind:?}"),
        }
    }

    fn append(_: &str, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
impl nom::error::FromExternalError<&str, String> for APRSParseContext {
    fn from_external_error(input: &str, _kind: nom::error::ErrorKind, error: String) -> Self {
        APRSParseContext {
            input: input.to_string(),
            message: error,
        }
    }
}
impl std::fmt::Display for APRSParseContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}, {1}", self.input, self.message)
    }
}
