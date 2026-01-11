#[derive(Debug, PartialEq, Clone)]
pub struct Aircraft {
    pub callsign: String,
    pub icao_address: ICAOAddress,
    pub time: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub struct ICAOAddress(u32);

impl ICAOAddress {
    pub const MAX_VALUE: u32 = 0x00FF_FFFF;

    pub fn new(value: u32) -> Result<Self, ICAOAddressError> {
        if value <= Self::MAX_VALUE {
            Ok(ICAOAddress(value))
        } else {
            Err(ICAOAddressError::InvalidAddress(value))
        }
    }

    #[must_use]
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ICAOAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08X}", self.0)
    }
}

#[derive(Debug)]
pub enum ICAOAddressError {
    InvalidHexFormat,
    InvalidAddress(u32),
}
impl std::fmt::Display for ICAOAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ICAOAddressError::InvalidHexFormat => write!(f, "Invalid hexadecimal format"),
            ICAOAddressError::InvalidAddress(val) => {
                write!(
                    f,
                    "Value 0x{:X} ({}) exceeds 24-bit ICAO address limit (0x{:X})",
                    val,
                    val,
                    ICAOAddress::MAX_VALUE
                )
            }
        }
    }
}
