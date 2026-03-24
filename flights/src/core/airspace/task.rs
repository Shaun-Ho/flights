use crate::core::airspace::detail::Airspace;
use crate::core::parser::Aircraft;
use crate::core::thread_manager::SteppableTask;

pub struct AirspaceStore {
    inner: std::sync::Arc<std::sync::RwLock<Airspace>>,
    aircraft_receiver: crossbeam_channel::Receiver<Aircraft>,
}
impl AirspaceStore {
    #[must_use]
    pub fn new(
        aircraft_receiver: crossbeam_channel::Receiver<Aircraft>,
        airspace_time_buffer: chrono::TimeDelta,
    ) -> Self {
        let empty_airspace = Airspace::new(airspace_time_buffer);
        AirspaceStore {
            inner: std::sync::Arc::new(std::sync::RwLock::new(empty_airspace)),
            aircraft_receiver,
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
    fn step(&mut self) -> bool {
        let mut aircrafts = Vec::new();
        let mut disconnected_channel = false;
        loop {
            match self.aircraft_receiver.try_recv() {
                Ok(aircraft) => {
                    aircrafts.push(aircraft);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    log::error!("Upstream disconnected");
                    disconnected_channel = true;
                    break;
                }
            }
        }

        if let Ok(mut airspace) = self.inner.write() {
            airspace.update(aircrafts);
            if disconnected_channel {
                return false;
            }
            return true;
        }
        false
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
        let store = AirspaceStore::new(receiver, chrono::TimeDelta::seconds(60));
        (sender, store)
    }

    #[test]
    fn test_step_active_channel_returns_true() {
        let (_sender, mut store) = setup_store();

        assert!(store.step());
    }

    #[test]
    fn test_step_disconnected_empty_channel_returns_false() {
        let (sender, mut store) = setup_store();

        drop(sender);

        assert!(!store.step());
    }

    #[test]
    fn test_when_step_sender_is_dropped_then_store_stops_stepping() {
        let (sender, mut store) = setup_store();

        let dummy_aircraft =
            create_dummy_aircraft_at_time(chrono::Utc::now(), ICAOAddress::new(0).unwrap());

        sender.send(dummy_aircraft).unwrap();
        drop(sender);

        assert!(!store.step());
    }
}
