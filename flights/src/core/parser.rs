use aprs_rs::{Aircraft, parse_aircraft};

use crate::core::ingestor::AprsPacket;
use crate::core::thread_manager::SteppableTask;

pub struct AircraftParser {
    receiver: crossbeam_channel::Receiver<AprsPacket>,
    sender: crossbeam_channel::Sender<Aircraft>,
}
impl AircraftParser {
    #[must_use]
    pub fn new(
        messages_receiver: crossbeam_channel::Receiver<AprsPacket>,
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
        let Ok(aprs_packet) = self.receiver.recv() else {
            log::error!("AircraftParser upstream disconnected");
            return false;
        };

        match parse_aircraft(&aprs_packet.message) {
            Ok(aircraft) => {
                if let Err(err) = self.sender.send(aircraft) {
                    log::error!("Failed to forward aircraft: {err}");
                }
            }
            Err(err) => log::debug!("{err:?}"),
        }

        true
    }
}
