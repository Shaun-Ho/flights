pub enum AircraftParseError {
    IncorrectSeparator(APRSParseContext),
    InvalidTimestamp(APRSParseContext),
    InvalidCallsign(APRSParseContext),
    InvalidLatitude(APRSParseContext),
    InvalidLongitude(APRSParseContext),
}

impl std::fmt::Display for AircraftParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg: &dyn std::fmt::Display = match self {
            AircraftParseError::IncorrectSeparator(e)
            | AircraftParseError::InvalidTimestamp(e)
            | AircraftParseError::InvalidLatitude(e)
            | AircraftParseError::InvalidLongitude(e)
            | AircraftParseError::InvalidCallsign(e) => e,
        };
        write!(f, "{msg}")
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
