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
        match self.receiver.recv() {
            Ok(message_string) => {
                match build_aircraft_from_string(&message_string) {
                    Ok(aircraft) => match self.sender.send(aircraft) {
                        Ok(()) => true,
                        Err(err) => {
                            log::error!("{err}");
                            true
                        }
                    },
                    Err(err) => {
                        log::debug!("{err}");
                        true
                    }
                };
                true
            }
            Err(err) => {
                log::error!("{err}");
                false
            }
        }
    }
}
