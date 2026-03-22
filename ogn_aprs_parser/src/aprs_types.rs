use crate::errors::{
    APRSMessageParseError, APRSParseContext, ICAOAddressError, OGNAddressTypeError,
    OGNAircraftTypeError, OGNBeaconIDError, OGNIDPrefixError,
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OgnAprsProtocol {
    OGADSB,
}

impl std::str::FromStr for OgnAprsProtocol {
    type Err = APRSMessageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OGADSB" => Ok(OgnAprsProtocol::OGADSB),
            _ => Err(APRSMessageParseError::InvalidOGNAprsProtocol(
                APRSParseContext {
                    input: s.to_owned(),
                    message: "Unsupported OGN APRS Protocol".to_owned(),
                },
            )),
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OGNAddressType {
    Unknown = 0,
    ICAO = 1,
    FLARM = 2,
    OGNTracker = 3,
}
impl OGNAddressType {
    pub fn from_u8(value: u8) -> Result<Self, OGNAddressTypeError> {
        match value {
            0 => Ok(OGNAddressType::Unknown),
            1 => Ok(OGNAddressType::ICAO),
            2 => Ok(OGNAddressType::FLARM),
            3 => Ok(OGNAddressType::OGNTracker),
            other => Err(OGNAddressTypeError::InvalidAddressType(other)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OGNIDPrefix {
    pub aircraft_type: OGNAircraftType,
    pub address_type: OGNAddressType,
    pub no_track: bool,
    pub stealth_mode: bool,
}

impl OGNIDPrefix {
    pub fn new(value: u8) -> Result<Self, OGNIDPrefixError> {
        let raw_type = (value >> 2) & 0b1111; // extract 4 bits
        let aircraft_type =
            OGNAircraftType::from_u8(raw_type).map_err(OGNIDPrefixError::AircraftType)?;

        let raw_address = value & 0b11; // extract 2 bits
        let address_type =
            OGNAddressType::from_u8(raw_address).map_err(OGNIDPrefixError::AddressType)?;

        let no_track = ((value >> 6) & 0b1) == 1;
        let stealth_mode = ((value >> 7) & 0b1) == 1;
        Ok(OGNIDPrefix {
            aircraft_type,
            address_type,
            no_track,
            stealth_mode,
        })
    }

    pub fn from_hex_str(s: &str) -> Result<Self, OGNIDPrefixError> {
        if s.len() != 2 {
            return Err(OGNIDPrefixError::HexFormat);
        }
        let parsed_value = u8::from_str_radix(s, 16).map_err(|_| OGNIDPrefixError::HexFormat)?;

        OGNIDPrefix::new(parsed_value)
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
