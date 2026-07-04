use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use ogn_aprs_parser::ICAOAddress;

use crate::airspace::detail::Airspace;
use crate::airspace::errors::PacketConversionError;
use crate::core::parser::Aircraft;
use crate::pb::airspace::{PbAircraftHistory, PbAirspace};

impl TryFrom<PbAirspace> for Airspace {
    type Error = PacketConversionError;
    fn try_from(packet: PbAirspace) -> Result<Self, Self::Error> {
        let pb_timestamp = packet
            .timestamp
            .ok_or(PacketConversionError::MissingTimestamp)?;
        let sys_timestamp = SystemTime::try_from(pb_timestamp)?;

        let timestamp: DateTime<Utc> = sys_timestamp.into();

        let mapping = packet
            .icao_to_aircraft_map
            .into_iter()
            .map(|(icao_address_u32, aircraft_history)| {
                let icao = ICAOAddress::new(icao_address_u32)
                    .map_err(PacketConversionError::InvalidICAOAddressKey)?;

                let aircraft_history: VecDeque<Aircraft> = aircraft_history
                    .aircraft
                    .into_iter()
                    .map(|pb_aircraft| {
                        Aircraft::try_from(pb_aircraft)
                            .map_err(PacketConversionError::InvalidAircraft)
                    })
                    .collect::<Result<_, _>>()?;

                Ok((icao, aircraft_history))
            })
            .collect::<Result<HashMap<_, _>, Self::Error>>()?;
        Ok(Self::from_state(timestamp, mapping))
    }
}

impl From<Airspace> for PbAirspace {
    fn from(airspace: Airspace) -> Self {
        let (datetime, icao_to_aircraft_mapping) = airspace.into_inner();
        let sys_time: SystemTime = datetime.into();
        let timestamp = Some(sys_time.into());

        let pb_mapping = icao_to_aircraft_mapping
            .into_iter()
            .map(|(address, history)| {
                let pb_address: u32 = address.value();

                let pb_history = PbAircraftHistory {
                    aircraft: history
                        .into_iter()
                        .map(|aircraft| aircraft.into())
                        .collect(),
                };

                (pb_address, pb_history)
            })
            .collect();
        Self {
            timestamp,
            icao_to_aircraft_map: pb_mapping,
        }
    }
}
