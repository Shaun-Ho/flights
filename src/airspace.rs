use crate::parser::types::{Aircraft, ICAOAddress};

#[derive(Debug)]
pub struct Airspace {
    buffer_duration: chrono::Duration,
    datetime: chrono::DateTime<chrono::Utc>,
    icao_to_aircraft_map:
        std::collections::HashMap<ICAOAddress, std::collections::VecDeque<Aircraft>>,
}
impl Airspace {
    #[must_use]
    pub fn new(buffer_duration: chrono::Duration) -> Self {
        Airspace {
            buffer_duration,
            datetime: chrono::Utc::now(),
            icao_to_aircraft_map: std::collections::HashMap::new(),
        }
    }

    pub fn update(&mut self, aircrafts: Vec<Aircraft>) {
        let mut aircrafts = aircrafts;
        self.update_datetime_and_prune();

        while let Some(aircraft) = aircrafts.pop() {
            // check that aircraft is within buffer window
            let cutoff_time = self.datetime - self.buffer_duration;
            if aircraft.time < cutoff_time {
                continue;
            }

            let history = self.get_history_or_create_empty_history(aircraft.icao_address);

            // We expect that the new data is normally most recent data, so we check that we can push
            // back into the end of the VecDeque
            if let Some(last) = history.back() {
                if aircraft.time >= last.time {
                    history.push_back(aircraft);
                    continue;
                }
            }

            // If it is not new data, try to see the data is old enough to be front of VecDeque
            if let Some(first) = history.front() {
                if aircraft.time <= first.time {
                    history.push_front(aircraft);
                    continue;
                }
            }

            // It is somewhere in between
            let idx = history.partition_point(|x| x.time < aircraft.time);
            history.insert(idx, aircraft);
        }
    }

    #[must_use]
    pub fn get_history(
        &self,
        icao_address: ICAOAddress,
    ) -> Option<&std::collections::VecDeque<Aircraft>> {
        self.icao_to_aircraft_map.get(&icao_address)
    }

    #[must_use]
    pub fn get_datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }

    fn update_datetime_and_prune(&mut self) {
        self.datetime = chrono::Utc::now();
        let cutoff_time = self.datetime - self.buffer_duration;
        for aircraft_history in self.icao_to_aircraft_map.values_mut() {
            while let Some(aircraft) = aircraft_history.front() {
                if aircraft.time < cutoff_time {
                    aircraft_history.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    // method to get history of an address, but populates a default empty VecDeque if icao_address does not exist
    fn get_history_or_create_empty_history(
        &mut self,
        icao_address: ICAOAddress,
    ) -> &mut std::collections::VecDeque<Aircraft> {
        self.icao_to_aircraft_map.entry(icao_address).or_default()
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        airspace::Airspace,
        parser::types::{Aircraft, ICAOAddress},
    };

    fn create_dummy_aircraft_at_time(
        datetime: chrono::DateTime<chrono::Utc>,
        icao_address: ICAOAddress,
    ) -> Aircraft {
        Aircraft {
            callsign: String::from("dummy"),
            icao_address: icao_address,
            time: datetime,
            latitude: 0.0,
            longitude: 0.0,
            ground_track: 0.0,
            ground_speed: 0.0,
            gps_altitude: 0.0,
        }
    }
    fn to_datetime(time_string: &str) -> chrono::DateTime<chrono::Utc> {
        let today = chrono::Local::now().date_naive();
        let time = chrono::NaiveTime::parse_from_str(time_string, "%H:%M:%S")
            .expect("time string not in %H:%M:%S format");
        today.and_time(time).and_utc()
    }

    #[test]
    fn when_adding_aircrafts_to_empty_entries_then_correct_histories_are_created() {
        let now_datetime = chrono::Utc::now();
        let mut airspace = Airspace {
            buffer_duration: chrono::TimeDelta::seconds(5),
            datetime: now_datetime,
            icao_to_aircraft_map: std::collections::HashMap::new(),
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

        airspace.update(aircrafts);

        assert_eq!(airspace.icao_to_aircraft_map.len(), 2);

        // check aircraft 1 inserted
        let aircraft_1_history = airspace
            .icao_to_aircraft_map
            .get(&expected_aircraft_1_icao_address)
            .expect("expected a VecDeque for aircraft 1");
        assert_eq!(aircraft_1_history.len(), 1);
        assert_eq!(aircraft_1_history[0].time, expected_aircraft_1_datetime);

        // check aircraft 2 inserted
        let aircraft_2_history = airspace
            .icao_to_aircraft_map
            .get(&expected_aircraft_2_icao_address)
            .expect("expected a VecDeque for aircraft 1");
        assert_eq!(aircraft_2_history.len(), 1);
        assert_eq!(aircraft_2_history[0].time, expected_aircraft_2_datetime);
    }

    #[cfg(test)]
    mod when_adding_aircrafts_to_existing_entries {
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

            let mut airspace = Airspace {
                buffer_duration: chrono::TimeDelta::seconds(5),
                datetime: to_datetime("00:01:00"),
                icao_to_aircraft_map: existing_order_mapping.into_iter().collect(),
            };
            let new_data = vec![create_dummy_aircraft_at_time(time_c, aircraft_icao_address)];

            airspace.update(new_data);

            let history = airspace
                .get_history(aircraft_icao_address)
                .expect("expected to have history");

            assert_eq!(history.len(), 3);
            assert_eq!(history[2].time, time_c);
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

            let mut airspace = Airspace {
                buffer_duration: chrono::TimeDelta::seconds(5),
                datetime: to_datetime("00:01:00"),
                icao_to_aircraft_map: existing_order_mapping.into_iter().collect(),
            };
            let new_data = vec![create_dummy_aircraft_at_time(time_a, aircraft_icao_address)];

            airspace.update(new_data);

            let history = airspace
                .get_history(aircraft_icao_address)
                .expect("expected to have history");

            assert_eq!(history.len(), 3);
            assert_eq!(history[2].time, time_c);
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

            let mut airspace = Airspace {
                buffer_duration: chrono::TimeDelta::seconds(5),
                datetime: to_datetime("00:01:00"),
                icao_to_aircraft_map: existing_order_mapping.into_iter().collect(),
            };
            let new_data = vec![create_dummy_aircraft_at_time(time_c, aircraft_icao_address)];

            airspace.update(new_data);

            let history = airspace
                .get_history(aircraft_icao_address)
                .expect("expected to have history");

            assert_eq!(history.len(), 4);
            assert_eq!(history[2].time, time_c);
        }
    }
}
