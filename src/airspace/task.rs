use crate::airspace::detail::Airspace;
use crate::core::central_disk_logger::{LogSender, ProtoLoggerHandle};
use crate::core::thread_manager::{SteppableTask, TaskState};
use crate::parser::Aircraft;
use crate::pb::airspace::PbAirspace;

pub struct AirspaceStore {
    inner: std::sync::Arc<std::sync::RwLock<Airspace>>,
    aircraft_receiver: crossbeam_channel::Receiver<Aircraft>,
    airspace_time_buffer: chrono::TimeDelta,
    logger: Option<ProtoLoggerHandle<PbAirspace>>,
}
impl AirspaceStore {
    #[must_use]
    pub fn new(
        aircraft_receiver: crossbeam_channel::Receiver<Aircraft>,
        airspace_time_buffer: chrono::TimeDelta,
        logger: Option<ProtoLoggerHandle<PbAirspace>>,
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
        let mut aircrafts = Vec::new();
        let mut is_disconnected = false;

        // drain the channel to check status of channel
        loop {
            match self.aircraft_receiver.try_recv() {
                Ok(aircraft) => {
                    aircrafts.push(aircraft);
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

        if !aircrafts.is_empty()
            && let Ok(mut airspace) = self.inner.write()
        {
            airspace.update(aircrafts, self.airspace_time_buffer);
            if let Some(logger) = &self.logger {
                let _ = logger.send((*airspace).clone());
            }
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
    use ogn_aprs_parser::ICAOAddress;

    use super::*;
    use crate::test_utilities::create_dummy_aircraft_at_time;

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

        sender.send(dummy_aircraft).unwrap();
        drop(sender);
        // we still continue to finish processing the disconnected queue
        assert!(matches!(store.step(), TaskState::Running));

        // when queue is empty, and channel is disconnected, next step() should error
        assert!(matches!(store.step(), TaskState::Completed));
    }
}
