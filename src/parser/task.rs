use ogn_aprs_parser::parse_ogn_aprs_aircraft_beacon;

use crate::core::central_disk_logger::{JsonlLoggerHandle, LogSender};
use crate::core::thread_manager::{SteppableTask, TaskState};
use crate::ingestor::AprsPacket;
use crate::parser::Aircraft;
use crate::parser::conversion::convert_ogn_aprs_beacon_to_aircraft;

pub struct AircraftParser {
    receiver: crossbeam_channel::Receiver<AprsPacket>,
    sender: crossbeam_channel::Sender<Aircraft>,
    logger: Option<JsonlLoggerHandle<Aircraft>>,
}
impl AircraftParser {
    #[must_use]
    pub fn new(
        messages_receiver: crossbeam_channel::Receiver<AprsPacket>,
        aircraft_sender: crossbeam_channel::Sender<Aircraft>,
        logger: Option<JsonlLoggerHandle<Aircraft>>,
    ) -> Self {
        AircraftParser {
            receiver: messages_receiver,
            sender: aircraft_sender,
            logger,
        }
    }
}

impl SteppableTask for AircraftParser {
    fn step(&mut self) -> TaskState {
        let Ok(aprs_packet) = self.receiver.recv() else {
            log::info!("AircraftParser upstream disconnected. Task complete");
            return TaskState::Completed;
        };

        match parse_ogn_aprs_aircraft_beacon(&aprs_packet.message) {
            Ok(aircraft_beacon) => {
                let aircraft =
                    convert_ogn_aprs_beacon_to_aircraft(aircraft_beacon, aprs_packet.timestamp);

                if let Some(logger) = &self.logger {
                    let _ = logger.send(aircraft.clone());
                }
                if let Err(err) = self.sender.send(aircraft) {
                    log::error!("Failed to forward aircraft: {err}");
                }
            }
            Err(err) => log::debug!("{err:?}"),
        }

        TaskState::Running
    }
}
