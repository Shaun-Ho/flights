pub mod builder;
mod constants;
pub mod types;

use crate::core::parser::builder::build_aircraft_from_string;
use crate::core::thread_manager::SteppableTask;
use crate::core::types::Aircraft;

pub struct AircraftParser {
    receiver: crossbeam_channel::Receiver<String>,
    sender: crossbeam_channel::Sender<Aircraft>,
}
impl AircraftParser {
    #[must_use]
    pub fn new(
        messages_receiver: crossbeam_channel::Receiver<String>,
        aircraft_sender: crossbeam_channel::Sender<Aircraft>,
    ) -> Self {
        AircraftParser {
            receiver: messages_receiver,
            sender: aircraft_sender,
        }
    }
}

impl SteppableTask for AircraftParser {
    fn step(&mut self) -> bool {
        let Ok(message_string) = self.receiver.recv() else {
            log::error!("AircraftParser upstream disconnected");
            return false;
        };

        match build_aircraft_from_string(&message_string) {
            Ok(aircraft) => {
                if let Err(err) = self.sender.send(aircraft) {
                    log::error!("Failed to forward aircraft: {err}");
                }
            }
            Err(err) => log::debug!("Discarding noisy data: {err}"),
        }

        true
    }
}
