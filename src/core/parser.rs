pub mod builder;
pub mod errors;
pub mod types;

use crate::core::ingestor::protobuf::PbAprsPacket;
use crate::core::parser::builder::build_aircraft_from_string;
use crate::core::thread_manager::SteppableTask;
use crate::core::types::Aircraft;

pub struct AircraftParser {
    receiver: crossbeam_channel::Receiver<PbAprsPacket>,
    sender: crossbeam_channel::Sender<Aircraft>,
}
impl AircraftParser {
    #[must_use]
    pub fn new(
        messages_receiver: crossbeam_channel::Receiver<PbAprsPacket>,
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

        match build_aircraft_from_string(&aprs_packet.payload) {
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
