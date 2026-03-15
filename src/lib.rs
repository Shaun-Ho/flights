pub mod cli;
pub mod core;
pub mod gui;
pub mod logging;

#[cfg(test)]
pub mod test_utilities;

use crate::core::airspace::{AirspaceStore, AirspaceViewer};
use crate::core::ingestor::Ingestor;
use crate::core::ingestor::config::IngestorConfig;
use crate::core::ingestor::pb::PbAprsPacket;
use crate::core::parser::AircraftParser;
use crate::core::thread_manager::{TaskID, ThreadManager};
use crate::core::types::Aircraft;

pub struct Pipeline {
    threadmanager: ThreadManager,
    end_chain_task_id: TaskID,
    renderer_viewer: AirspaceViewer,
}
impl Pipeline {
    #[must_use]
    pub fn get_airspace_viewer(&self) -> AirspaceViewer {
        self.renderer_viewer.clone()
    }
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        self.threadmanager.stop_all_tasks();
        self.threadmanager
            .wait_on_task_finish(self.end_chain_task_id);
    }
}

#[must_use]
pub fn setup_pipeline(ingestor_config: IngestorConfig) -> Pipeline {
    let (messages_sender, messages_receiver): (
        crossbeam_channel::Sender<PbAprsPacket>,
        crossbeam_channel::Receiver<PbAprsPacket>,
    ) = crossbeam_channel::unbounded();

    let (aircraft_data_sender, aircraft_data_receiver): (
        crossbeam_channel::Sender<Aircraft>,
        crossbeam_channel::Receiver<Aircraft>,
    ) = crossbeam_channel::unbounded();
    let ingestor = match ingestor_config.read_path {
        Some(read_path) => Ingestor::read_data_from_file(
            &read_path,
            messages_sender,
            ingestor_config.write_path.as_deref(),
        ),
        None => Ingestor::connect_glidernet(
            &ingestor_config.glidernet,
            messages_sender,
            ingestor_config.write_path.as_deref(),
        ),
    }
    .map_err(|e| log::error!("Error constructing ingestor: {e}"))
    .unwrap();

    let parser = AircraftParser::new(messages_receiver, aircraft_data_sender);

    let airspace_store = AirspaceStore::new(
        aircraft_data_receiver,
        chrono::TimeDelta::seconds(ingestor_config.airspace.time_buffer_seconds.into()),
    );
    let renderer_viewer = airspace_store.get_airspace_viewer();

    let mut thread_manager = ThreadManager::new();
    thread_manager.add_task(ingestor, std::time::Duration::ZERO);
    thread_manager.add_task(parser, std::time::Duration::ZERO);
    let airspace_task_id =
        thread_manager.add_task(airspace_store, std::time::Duration::from_micros(16667));

    Pipeline {
        threadmanager: thread_manager,
        end_chain_task_id: airspace_task_id,
        renderer_viewer,
    }
}
#[cfg(test)]
mod test {
    use crate::core::ingestor::config::{AirspaceConfig, GliderNetConfig};
    use crate::test_utilities::test_data_path;
    use crate::{core::ingestor::config::IngestorConfig, setup_pipeline};

    #[rstest::rstest]
    #[test_log::test]
    fn given_pipeline_setup_with_ingestor_reading_from_file_when_ingestor_terminates_then_entire_pipeline_shuts_down_gracefully(
        test_data_path: std::path::PathBuf,
    ) {
        let read_path = test_data_path.join("test_ingestor_log.txt");

        let ingestor_config = IngestorConfig {
            read_path: Some(read_path),
            write_path: None,
            glidernet: GliderNetConfig {
                host: String::new(),
                port: 0,
                filter: String::new(),
            },
            airspace: AirspaceConfig {
                time_buffer_seconds: 1,
            },
        };
        let pipeline = setup_pipeline(ingestor_config);
        drop(pipeline);
    }
}
