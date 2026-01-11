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

#[derive(Debug, PartialEq)]
pub struct OGNBeaconID {
    pub prefix: OGNIDPrefix,
    pub icao_address: ICAOAddress,
}

impl OGNBeaconID {
    #[must_use]
    pub fn new(prefix: OGNIDPrefix, icao_address: ICAOAddress) -> Self {
        OGNBeaconID {
            prefix,
            icao_address,
        }
    }
}
impl std::str::FromStr for OGNBeaconID {
    type Err = OGNBeaconIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 8 {
            let prefix_hex = &s[..2]; // "XX"
            let address_hex = &s[2..]; // "YYYYYY"

            let prefix = OGNIDPrefix::from_hex_str(prefix_hex)
                .map_err(OGNBeaconIDError::OGNIDPrefixError)?;
            let icao_address =
                ICAOAddress::new(u32::from_str_radix(address_hex, 16).map_err(|_| {
                    OGNBeaconIDError::ICAOAddressError(ICAOAddressError::InvalidHexFormat)
                })?)
                .map_err(OGNBeaconIDError::ICAOAddressError)?;

            Ok(OGNBeaconID::new(prefix, icao_address))
        } else {
            Err(OGNBeaconIDError::InvalidOGNBeaconFormat(s.to_string()))
        }
    }
}
pub enum OGNBeaconIDError {
    OGNIDPrefixError(OGNIDPrefixError),
    ICAOAddressError(ICAOAddressError),
    InvalidOGNBeaconFormat(String),
}
impl std::fmt::Display for OGNBeaconIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OGNBeaconIDError::OGNIDPrefixError(e) => write!(f, "{e}"),
            OGNBeaconIDError::ICAOAddressError(e) => write!(f, "{e}"),
            OGNBeaconIDError::InvalidOGNBeaconFormat(string) => {
                write!(f, "Invalid beacon format: {string}")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OGNAddressType {
    Unknown = 0,
    ICAO = 1,
    FLARM = 2,
    OgnTracker = 3,
}
impl OGNAddressType {
    pub fn from_u8(value: u8) -> Result<Self, OGNAddressTypeError> {
        match value {
            0 => Ok(OGNAddressType::Unknown),
            1 => Ok(OGNAddressType::ICAO),
            2 => Ok(OGNAddressType::FLARM),
            3 => Ok(OGNAddressType::OgnTracker),
            other => Err(OGNAddressTypeError::InvalidAddressType(other)),
        }
    }
}

#[derive(Debug)]
pub enum OGNAddressTypeError {
    InvalidAddressType(u8),
}
impl std::fmt::Display for OGNAddressTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OGNAddressTypeError::InvalidAddressType(address) => {
                write!(f, "Invalid address format {address}")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OGNIDPrefix {
    hex: u8,
    pub aircraft_type: OGNAircraftType,
    pub address_type: OGNAddressType,
    pub no_track: bool,
    pub stealth_mode: bool,
}

impl OGNIDPrefix {
    pub fn new(value: u8) -> Result<Self, OGNIDPrefixError> {
        let raw_type = (value >> 2) & 0b1111; // extract 4 bits
        let aircraft_type =
            OGNAircraftType::from_u8(raw_type).map_err(OGNIDPrefixError::InvalidAircraftType)?;

        let raw_address = value & 0b11; // extract 2 bits
        let address_type =
            OGNAddressType::from_u8(raw_address).map_err(OGNIDPrefixError::InvalidAddressType)?;

        let no_track = ((value >> 6) & 0b1) == 1;
        let stealth_mode = ((value >> 7) & 0b1) == 1;
        Ok(OGNIDPrefix {
            hex: value,
            aircraft_type,
            address_type,
            no_track,
            stealth_mode,
        })
    }

    pub fn from_hex_str(s: &str) -> Result<Self, OGNIDPrefixError> {
        if s.len() != 2 {
            return Err(OGNIDPrefixError::InvalidHexFormat);
        }
        let parsed_value =
            u8::from_str_radix(s, 16).map_err(|_| OGNIDPrefixError::InvalidHexFormat)?;

        OGNIDPrefix::new(parsed_value)
    }
}
#[derive(Debug)]
pub enum OGNIDPrefixError {
    InvalidHexFormat,
    InvalidAircraftType(OGNAircraftTypeError),
    InvalidAddressType(OGNAddressTypeError),
}
impl std::fmt::Display for OGNIDPrefixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OGNIDPrefixError::InvalidHexFormat => write!(f, "Invalid hexadecimal format"),
            OGNIDPrefixError::InvalidAircraftType(e) => write!(f, "{e}"),
            OGNIDPrefixError::InvalidAddressType(e) => write!(f, "{e}"),
        }
    }
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OGNAircraftType {
    Reserved = 0,
    Glider = 1,
    TowPlane = 2,
    Helicopter = 3,
    Parachute = 4,
    DropPlane = 5,
    HangGlider = 6,
    Paraglider = 7,
    ReciprocatingEngineAircraft = 8,
    JetTurbopropAircraft = 9,
    Unknown = 10,
    Balloon = 11,
    Airship = 12,
    UAVs = 13,
    StaticObstacle = 15,
}

impl OGNAircraftType {
    pub fn from_u8(value: u8) -> Result<Self, OGNAircraftTypeError> {
        match value {
            0 | 14 => Ok(OGNAircraftType::Reserved),
            1 => Ok(OGNAircraftType::Glider),
            2 => Ok(OGNAircraftType::TowPlane),
            3 => Ok(OGNAircraftType::Helicopter),
            4 => Ok(OGNAircraftType::Parachute),
            5 => Ok(OGNAircraftType::DropPlane),
            6 => Ok(OGNAircraftType::HangGlider),
            7 => Ok(OGNAircraftType::Paraglider),
            8 => Ok(OGNAircraftType::ReciprocatingEngineAircraft),
            9 => Ok(OGNAircraftType::JetTurbopropAircraft),
            10 => Ok(OGNAircraftType::Unknown),
            11 => Ok(OGNAircraftType::Balloon),
            12 => Ok(OGNAircraftType::Airship),
            13 => Ok(OGNAircraftType::UAVs),
            15 => Ok(OGNAircraftType::StaticObstacle),
            other => Err(OGNAircraftTypeError::InvalidEnum(other)),
        }
    }
}
#[derive(Debug)]
pub enum OGNAircraftTypeError {
    InvalidEnum(u8),
}

impl std::fmt::Display for OGNAircraftTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OGNAircraftTypeError::InvalidEnum(value) => write!(f, "Invalid value: {value}"),
        }
    }
}
