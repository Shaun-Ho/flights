use chrono::Utc;

use crate::airspace::detail::{Airspace, AirspaceUpdate};
use crate::core::central_disk_logger::{LogSender, ProtoLoggerHandle};
use crate::core::thread_manager::{SteppableTask, TaskState};
use crate::parser::Aircraft;
use crate::pb::airspace::PbAirspaceUpdate;

pub struct AirspaceStore {
    inner: std::sync::Arc<std::sync::RwLock<Airspace>>,
    aircraft_receiver: crossbeam_channel::Receiver<Aircraft>,
    airspace_time_buffer: chrono::TimeDelta,
    logger: Option<ProtoLoggerHandle<PbAirspaceUpdate>>,
}
impl AirspaceStore {
    #[must_use]
    pub fn new(
        aircraft_receiver: crossbeam_channel::Receiver<Aircraft>,
        airspace_time_buffer: chrono::TimeDelta,
        logger: Option<ProtoLoggerHandle<PbAirspaceUpdate>>,
    ) -> Self {
        let empty_airspace = Airspace::new();
        AirspaceStore {
            inner: std::sync::Arc::new(std::sync::RwLock::new(empty_airspace)),
            aircraft_receiver,
            airspace_time_buffer,
            logger,
        }
    }
    #[must_use]
    pub fn get_airspace_viewer(&self) -> AirspaceViewer {
        AirspaceViewer {
            inner: self.inner.clone(),
        }
    }
}

impl SteppableTask for AirspaceStore {
    fn step(&mut self) -> TaskState {
        let mut aircraft_vec = Vec::new();
        let mut is_disconnected = false;

        // drain the channel to check status of channel
        loop {
            match self.aircraft_receiver.try_recv() {
                Ok(aircraft) => {
                    aircraft_vec.push(aircraft);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    break;
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    log::error!("AirspaceStore upstream disconnected");
                    is_disconnected = true;
                    break;
                }
            }
        }

        let airspace_update = AirspaceUpdate {
            timestamp: Utc::now(),
            updates: aircraft_vec,
        };
        if let Some(logger) = &self.logger {
            let _ = logger.send(airspace_update.clone());
        }

        if let Ok(mut airspace) = self.inner.write() {
            match airspace.update(airspace_update, self.airspace_time_buffer) {
                Ok(_) => (),
                Err(e) => log::error!("{e}"),
            };
        }

        if is_disconnected {
            TaskState::Completed
        } else {
            TaskState::Running
        }
    }
}
#[derive(Clone)]
pub struct AirspaceViewer {
    inner: std::sync::Arc<std::sync::RwLock<Airspace>>,
}
impl AirspaceViewer {
    #[allow(clippy::missing_panics_doc)]
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Airspace> {
        self.inner.read().expect("Read lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};

    use ogn_aprs_parser::ICAOAddress;

    use super::*;
    use crate::{airspace::detail::AircraftTrack, test_utilities::create_dummy_aircraft_at_time};

    fn setup_store() -> (crossbeam_channel::Sender<Aircraft>, AirspaceStore) {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let store = AirspaceStore::new(receiver, chrono::TimeDelta::seconds(60), None);
        (sender, store)
    }

    #[test]
    fn when_upstream_channel_is_non_empty_and_connected_then_step_returns_running_state() {
        let (sender, mut store) = setup_store();
        let dummy_aircraft =
            create_dummy_aircraft_at_time(chrono::Utc::now(), ICAOAddress::new(0).unwrap());
        sender.send(dummy_aircraft).unwrap();

        assert!(matches!(store.step(), TaskState::Running));
    }

    #[test]
    fn when_upstream_channel_is_empty_and_disconnected_then_step_returns_errored_state() {
        let (sender, mut store) = setup_store();

        drop(sender);

        assert!(matches!(store.step(), TaskState::Completed));
    }

    #[test]
    fn when_upstream_channel_is_non_empty_and_disconnected_then_step_returns_running_state_then_errors_on_next()
     {
        let (sender, mut store) = setup_store();

        let dummy_aircraft =
            create_dummy_aircraft_at_time(chrono::Utc::now(), ICAOAddress::new(0).unwrap());

        let expected_mapping = HashMap::from([(
            dummy_aircraft.icao_address,
            AircraftTrack::create_with_history(
                dummy_aircraft.icao_address,
                VecDeque::from([dummy_aircraft.clone().into()]),
            ),
        )]);
        sender.send(dummy_aircraft).unwrap();
        drop(sender);

        assert!(matches!(store.step(), TaskState::Completed));
        let viewer = store.get_airspace_viewer();
        let airspace = viewer.read();
        let mapping = airspace.get_tracks();
        assert_eq!(mapping, &expected_mapping);
    }
}
