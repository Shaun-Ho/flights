pub mod builder;
mod constants;
pub mod types;

use crate::parser::{builder::build_aircraft_from_string, types::Aircraft};
use crate::thread_manager::SteppableTask;

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
        self.receiver
            .recv()
            .map_err(|e| {
                log::error!("{e}");
            })
            .map(|string| {
                let maybe_aircraft = build_aircraft_from_string(&string);
                match maybe_aircraft {
                    Ok(aircraft) => {
                        let _ = self.sender.send(aircraft).map_err(|err| {
                            log::error!("Unable to send aircraft to channel: {err}");
                        });
                    }
                    Err(err) => log::error!("Unable to convert aircraft: {err:?}. Skipping."),
                }
            })
            .is_ok()
    }
}
