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
        let aircrafts: Vec<Aircraft> = self.aircraft_receiver.try_iter().collect();

        if let Ok(mut airspace) = self.inner.write() {
            airspace.update(aircrafts);
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
