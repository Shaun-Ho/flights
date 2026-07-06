use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::vec_deque;
use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use ogn_aprs_parser::ICAOAddress;

use crate::airspace::errors::{AirspaceError, ProblematicAircraftUpdate};
use crate::parser::Aircraft;

#[derive(Debug, Clone)]
pub struct Airspace {
    timestamp: chrono::DateTime<chrono::Utc>,
    tracks: HashMap<ICAOAddress, AircraftTrack>,
}
impl Airspace {
    #[must_use]
    pub fn new() -> Self {
        Airspace {
            timestamp: chrono::DateTime::<chrono::Utc>::MIN_UTC,
            tracks: HashMap::new(),
        }
    }
    pub fn create_with_state(
        timestamp: DateTime<Utc>,
        tracks: HashMap<ICAOAddress, AircraftTrack>,
    ) -> Self {
        Airspace { timestamp, tracks }
    }

    pub fn update(
        &mut self,
        airspace_update: AirspaceUpdate,
        buffer_duration: chrono::Duration,
    ) -> Result<(), AirspaceError> {
        let mut aircrafts = airspace_update.updates;

        if self.timestamp > airspace_update.timestamp {
            return Err(AirspaceError::InvalidUpdateTimestamp(
                airspace_update.timestamp,
            ));
        }
        self.timestamp = airspace_update.timestamp;

        let mut problematic = Vec::new();

        while let Some(aircraft) = aircrafts.pop() {
            // if aircraft broadcasted timestamp is greater than airspace timestamp,
            // we discard it
            if aircraft.broadcasted_timestamp > self.timestamp {
                problematic.push(ProblematicAircraftUpdate {
                    airspace_timestamp: self.timestamp,
                    // aircraft,
                    aircraft: aircraft.clone(),
                });
            }

            // check that aircraft is within buffer window
            let cutoff_time = self.timestamp - buffer_duration;
            if aircraft.broadcasted_timestamp < cutoff_time {
                continue;
            }

            self.update_or_register_track(aircraft);
        }
        self.prune(buffer_duration);
        if problematic.is_empty() {
            Ok(())
        } else {
            Err(AirspaceError::ContainedInvalidAircraftTimestamp(
                problematic,
            ))
        }
    }

    #[must_use]
    pub fn get_aircraft_track(&self, icao_address: ICAOAddress) -> Option<&AircraftTrack> {
        self.tracks.get(&icao_address)
    }

    #[must_use]
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    #[must_use]
    pub fn get_tracks(&self) -> &HashMap<ICAOAddress, AircraftTrack> {
        &self.tracks
    }

    fn update_or_register_track(&mut self, aircraft: Aircraft) {
        match self.tracks.entry(aircraft.icao_address) {
            Occupied(mut entry) => {
                entry.get_mut().insert(aircraft.into());
            }
            Vacant(entry) => {
                entry.insert(AircraftTrack::new(aircraft.icao_address, aircraft.into()));
            }
        };
    }

    #[must_use]
    pub fn into_inner(self) -> (DateTime<Utc>, HashMap<ICAOAddress, AircraftTrack>) {
        (self.timestamp, self.tracks)
    }

    fn prune(&mut self, buffer_duration: chrono::Duration) {
        let cutoff_time = self
            .timestamp
            .checked_sub_signed(buffer_duration)
            .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC);

        for track in self.tracks.values_mut() {
            while let Some(aircraft) = track.history.front() {
                if aircraft.broadcasted_timestamp < cutoff_time {
                    track.history.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}

impl Default for Airspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AirspaceUpdate {
    pub timestamp: DateTime<Utc>,
    pub updates: Vec<Aircraft>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AircraftTrack {
    icao_address: ICAOAddress,
    history: VecDeque<AircraftState>,
}
impl AircraftTrack {
    pub fn new(icao_address: ICAOAddress, current: AircraftState) -> Self {
        Self {
            icao_address,
            history: VecDeque::from([current]),
        }
    }
    pub fn create_with_history(
        icao_address: ICAOAddress,
        history: VecDeque<AircraftState>,
    ) -> Self {
        Self {
            icao_address,
            history,
        }
    }

    pub fn insert(&mut self, state: AircraftState) {
        // We expect that the new data is normally most recent data, so we check that we can push
        // back into the end of the VecDeque
        if let Some(last) = self.history.back()
            && state.broadcasted_timestamp >= last.broadcasted_timestamp
        {
            self.history.push_back(state);
            return;
        }

        // If it is not new data, try to see the data is old enough to be front of VecDeque
        if let Some(first) = self.history.front()
            && state.broadcasted_timestamp <= first.broadcasted_timestamp
        {
            self.history.push_front(state);
            return;
        }

        // It is somewhere in between
        let idx = self
            .history
            .partition_point(|x| x.broadcasted_timestamp < state.broadcasted_timestamp);

        self.history.insert(idx, state);
    }

    #[must_use]
    pub fn latest_state(&self) -> Option<&AircraftState> {
        self.history.back()
    }

    pub fn icao_address(&self) -> ICAOAddress {
        self.icao_address
    }

    pub fn iter_history(&self) -> vec_deque::Iter<'_, AircraftState> {
        self.history.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AircraftState {
    pub broadcasted_timestamp: chrono::DateTime<chrono::Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub ground_track: f64,
    pub ground_speed: f64,
    pub gps_altitude: f64,
}
impl From<Aircraft> for AircraftState {
    fn from(aircraft: Aircraft) -> Self {
        Self {
            broadcasted_timestamp: aircraft.broadcasted_timestamp,
            latitude: aircraft.latitude,
            longitude: aircraft.longitude,
            ground_track: aircraft.ground_track,
            ground_speed: aircraft.ground_speed,
            gps_altitude: aircraft.gps_altitude,
        }
    }
}

#[cfg(test)]
mod tests {

    use ogn_aprs_parser::ICAOAddress;

    use crate::airspace::detail::{Airspace, AirspaceUpdate};
    use crate::test_utilities::create_dummy_aircraft_at_time;

    fn to_datetime(time_string: &str) -> chrono::DateTime<chrono::Utc> {
        let today = chrono::Utc::now().date_naive();
        let time = chrono::NaiveTime::parse_from_str(time_string, "%H:%M:%S")
            .expect("time string not in %H:%M:%S format");
        today.and_time(time).and_utc()
    }

    #[test]
    fn when_adding_aircrafts_to_empty_entries_then_correct_histories_are_created() {
        let now_datetime = chrono::Utc::now();
        let buffer_duration = chrono::TimeDelta::seconds(5);
        let mut airspace = Airspace {
            timestamp: now_datetime,
            tracks: std::collections::HashMap::new(),
        };

        let expected_aircraft_1_icao_address = ICAOAddress::new(0).unwrap();
        let expected_aircraft_1_datetime = now_datetime - chrono::TimeDelta::seconds(1);

        let expected_aircraft_2_icao_address = ICAOAddress::new(1).unwrap();
        let expected_aircraft_2_datetime = now_datetime - chrono::TimeDelta::seconds(1);

        #[rustfmt::skip]
        let aircrafts = vec![
            create_dummy_aircraft_at_time(expected_aircraft_1_datetime, expected_aircraft_1_icao_address),
            create_dummy_aircraft_at_time(expected_aircraft_2_datetime, expected_aircraft_2_icao_address),
        ];
        let airspace_update = AirspaceUpdate {
            timestamp: now_datetime,
            updates: aircrafts,
        };

        let _ = airspace.update(airspace_update, buffer_duration);

        assert_eq!(airspace.tracks.len(), 2);

        // check aircraft 1 inserted
        let aircraft_1_track = airspace
            .tracks
            .get(&expected_aircraft_1_icao_address)
            .expect("expected a VecDeque for aircraft 1");
        assert_eq!(aircraft_1_track.history.len(), 1);
        assert_eq!(
            aircraft_1_track.history[0].broadcasted_timestamp,
            expected_aircraft_1_datetime
        );

        // check aircraft 2 inserted
        let aircraft_2_track = airspace
            .tracks
            .get(&expected_aircraft_2_icao_address)
            .expect("expected a VecDeque for aircraft 1");
        assert_eq!(aircraft_2_track.history.len(), 1);
        assert_eq!(
            aircraft_2_track.history[0].broadcasted_timestamp,
            expected_aircraft_2_datetime
        );
    }
    #[test]
    fn when_adding_aircrafts_to_empty_entries_then_airspace_datetime_is_correctly_updated() {
        let buffer_duration = chrono::TimeDelta::seconds(5);
        let mut airspace = Airspace::new();
        let now_datetime = chrono::Utc::now();

        let expected_aircraft_1_icao_address = ICAOAddress::new(0).unwrap();
        let expected_aircraft_1_datetime = now_datetime;

        let expected_aircraft_2_icao_address = ICAOAddress::new(1).unwrap();
        let expected_aircraft_2_datetime = now_datetime - chrono::TimeDelta::seconds(1);

        #[rustfmt::skip]
        let aircrafts = vec![
            create_dummy_aircraft_at_time(expected_aircraft_1_datetime, expected_aircraft_1_icao_address),
            create_dummy_aircraft_at_time(expected_aircraft_2_datetime, expected_aircraft_2_icao_address),
        ];
        let airspace_update = AirspaceUpdate {
            timestamp: now_datetime,
            updates: aircrafts,
        };

        let _ = airspace.update(airspace_update, buffer_duration);
        assert_eq!(airspace.timestamp, now_datetime);
    }

    #[cfg(test)]
    mod when_adding_aircrafts_to_existing_entries {

        use crate::airspace::detail::AircraftTrack;

        use super::*;
        #[test]
        fn and_aircraft_timestamp_is_newest_then_correct_order_is_added() {
            // existing:
            // aircraft: [time_a, time_b]
            // expect:
            // aircraft: [time_a, time_b, time_c]
            let now = chrono::Utc::now();
            let aircraft_icao_address = ICAOAddress::new(0).unwrap();
            let time_a = now - chrono::TimeDelta::seconds(3);
            let time_b = now - chrono::TimeDelta::seconds(2);
            let time_c = now - chrono::TimeDelta::seconds(1);

            let existing_order_mapping = [(
                aircraft_icao_address,
                std::collections::VecDeque::from([
                    create_dummy_aircraft_at_time(time_a, aircraft_icao_address),
                    create_dummy_aircraft_at_time(time_b, aircraft_icao_address),
                ]),
            )];
            let buffer_duration = chrono::TimeDelta::seconds(5);
            let existing = existing_order_mapping
                .into_iter()
                .map(|(icao_address, deque)| {
                    let history = deque.into_iter().map(Into::into).collect();
                    let track = AircraftTrack::create_with_history(icao_address, history);
                    (icao_address, track)
                })
                .collect();
            let mut airspace = Airspace {
                timestamp: to_datetime("00:01:00"),
                tracks: existing,
            };

            let aircrafts = vec![create_dummy_aircraft_at_time(time_c, aircraft_icao_address)];

            let airspace_update = AirspaceUpdate {
                timestamp: now,
                updates: aircrafts,
            };

            let _ = airspace.update(airspace_update, buffer_duration);

            let track = airspace
                .get_aircraft_track(aircraft_icao_address)
                .expect("expected to have history");

            assert_eq!(track.history.len(), 3);
            assert_eq!(track.history[2].broadcasted_timestamp, time_c);
        }
        #[test]
        fn and_aircraft_timestamp_is_oldest_then_correct_order_is_added() {
            // existing:
            // aircraft: [time_b, time_c]
            // expect:
            // aircraft: [time_a, time_b, time_c]
            let now = chrono::Utc::now();
            let aircraft_icao_address = ICAOAddress::new(0).unwrap();
            let time_a = now - chrono::TimeDelta::seconds(2);
            let time_b = now - chrono::TimeDelta::seconds(1);
            let time_c = now;

            let existing_order_mapping = [(
                aircraft_icao_address,
                std::collections::VecDeque::from([
                    create_dummy_aircraft_at_time(time_b, aircraft_icao_address),
                    create_dummy_aircraft_at_time(time_c, aircraft_icao_address),
                ]),
            )];

            let existing = existing_order_mapping
                .into_iter()
                .map(|(icao_address, deque)| {
                    let history = deque.into_iter().map(Into::into).collect();
                    let track = AircraftTrack::create_with_history(icao_address, history);
                    (icao_address, track)
                })
                .collect();

            let buffer_duration = chrono::TimeDelta::seconds(5);
            let mut airspace = Airspace {
                timestamp: to_datetime("00:01:00"),
                tracks: existing,
            };
            let aircrafts = vec![create_dummy_aircraft_at_time(time_a, aircraft_icao_address)];

            let airspace_update = AirspaceUpdate {
                timestamp: now,
                updates: aircrafts,
            };

            let _ = airspace.update(airspace_update, buffer_duration);

            let tracks = airspace
                .get_aircraft_track(aircraft_icao_address)
                .expect("expected to have history");

            assert_eq!(tracks.history.len(), 3);
            assert_eq!(tracks.history[2].broadcasted_timestamp, time_c);
        }
        #[test]
        fn and_aircraft_timestamp_is_somewhere_in_between_then_correct_order_is_added() {
            // existing:
            // aircraft: [time_a, time_b, time_d]
            // expect:
            // aircraft: [time_a, time_b, time_c, time_d]
            let now = chrono::Utc::now();

            let aircraft_icao_address = ICAOAddress::new(0).unwrap();
            let time_a = now - chrono::TimeDelta::seconds(3);
            let time_b = now - chrono::TimeDelta::seconds(2);
            let time_c = now - chrono::TimeDelta::seconds(1);
            let time_d = now;

            let existing_order_mapping = [(
                aircraft_icao_address,
                std::collections::VecDeque::from([
                    create_dummy_aircraft_at_time(time_a, aircraft_icao_address),
                    create_dummy_aircraft_at_time(time_b, aircraft_icao_address),
                    create_dummy_aircraft_at_time(time_d, aircraft_icao_address),
                ]),
            )];

            let buffer_duration = chrono::TimeDelta::seconds(5);

            let existing = existing_order_mapping
                .into_iter()
                .map(|(icao_address, deque)| {
                    let history = deque.into_iter().map(Into::into).collect();
                    let track = AircraftTrack::create_with_history(icao_address, history);
                    (icao_address, track)
                })
                .collect();

            let mut airspace = Airspace {
                timestamp: to_datetime("00:01:00"),
                tracks: existing,
            };
            let aircrafts = vec![create_dummy_aircraft_at_time(time_c, aircraft_icao_address)];

            let airspace_update = AirspaceUpdate {
                timestamp: now,
                updates: aircrafts,
            };

            let _ = airspace.update(airspace_update, buffer_duration);

            let tracks = airspace
                .get_aircraft_track(aircraft_icao_address)
                .expect("expected to have history");

            assert_eq!(tracks.history.len(), 4);
            assert_eq!(tracks.history[2].broadcasted_timestamp, time_c);
        }
    }
}
