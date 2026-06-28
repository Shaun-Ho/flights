use crate::core::airspace::{AirspaceStore, AirspaceViewer};
use crate::core::ingestor::{AprsPacket, Ingestor};
use crate::core::parser::{Aircraft, AircraftParser};
use crate::core::thread_manager::{SteppableTask, TaskID, ThreadManager};
use crate::pipeline::config::{FilePathConfig, IngestorSource, PipelineConfig};

pub struct AirspaceDataPipeline {
    thread_manager: ThreadManager,
    end_chain_task_id: TaskID,
    renderer_viewer: AirspaceViewer,
}
impl AirspaceDataPipeline {
    #[must_use]
    pub fn new(
        task_order: Vec<(Box<dyn SteppableTask>, std::time::Duration)>,
        airspace_store: AirspaceStore,
        update_tick: std::time::Duration,
    ) -> Self {
        let mut thread_manager = ThreadManager::new();

        for (task, duration) in task_order {
            thread_manager.add_task(task, duration);
        }
        let renderer_viewer = airspace_store.get_airspace_viewer();
        let end_chain_task_id = thread_manager.add_task(airspace_store, update_tick);
        Self {
            thread_manager,
            end_chain_task_id,
            renderer_viewer,
        }
    }

    #[must_use]
    pub fn setup_pipeline(pipeline_config: PipelineConfig) -> Self {
        let (messages_sender, messages_receiver): (
            crossbeam_channel::Sender<AprsPacket>,
            crossbeam_channel::Receiver<AprsPacket>,
        ) = crossbeam_channel::unbounded();

        let (aircraft_data_sender, aircraft_data_receiver): (
            crossbeam_channel::Sender<Aircraft>,
            crossbeam_channel::Receiver<Aircraft>,
        ) = crossbeam_channel::unbounded();
        let ingestor = match pipeline_config.ingestor.source {
            IngestorSource::FilePath(FilePathConfig { read_path }) => {
                Ingestor::read_data_from_file(&read_path, messages_sender)
            }
            IngestorSource::GliderNet(config) => {
                Ingestor::connect_glidernet(&config, messages_sender)
            }
        }
        .map_err(|e| log::error!("Error constructing ingestor: {e}"))
        .unwrap();

        let parser = AircraftParser::new(messages_receiver, aircraft_data_sender);

        let airspace_store = AirspaceStore::new(
            aircraft_data_receiver,
            chrono::TimeDelta::seconds(pipeline_config.airspace.time_buffer_seconds.into()),
        );
        let task_order: Vec<(Box<dyn SteppableTask>, std::time::Duration)> = vec![
            (Box::new(ingestor), std::time::Duration::ZERO),
            (Box::new(parser), std::time::Duration::ZERO),
        ];
        Self::new(
            task_order,
            airspace_store,
            std::time::Duration::from_micros(16667),
        )
    }
    #[must_use]
    pub fn get_airspace_viewer(&self) -> AirspaceViewer {
        self.renderer_viewer.clone()
    }

    pub fn shutdown(&mut self) {
        self.thread_manager.stop_all_tasks();
        self.thread_manager
            .wait_on_task_finish(self.end_chain_task_id);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::ingestor::PbAprsPacket;
    use crate::pipeline::AirspaceDataPipeline;
    use crate::pipeline::config::{AirspaceConfig, IngestorConfig};
    use crate::test_utilities::{TestPath, test_path, write_pb_message_to_disk};

    #[rstest::rstest]
    #[test_log::test]
    fn given_pipeline_setup_with_ingestor_reading_from_file_when_ingestor_terminates_then_entire_pipeline_shuts_down_gracefully(
        test_path: TestPath,
    ) {
        let now = std::time::SystemTime::now();
        let timestamp = prost_types::Timestamp::from(now);
        let message = "ICA020113>OGADSB,qAS,AVX1081:/190558h5050.73N/00413.19E^222/262/A=007246 !W06! id25020113 +2880fpm FL079.69 A3:RAM831F Sq7122".into();
        let packet = PbAprsPacket {
            timestamp: Some(timestamp),
            message,
        };
        let read_path = test_path.path.join("test_ingestor_log.pb");
        let mut writer = std::io::BufWriter::new(std::fs::File::create(&read_path).unwrap());
        let _ = write_pb_message_to_disk(&mut writer, &packet);
        let ingestor_config = IngestorConfig {
            source: IngestorSource::FilePath(FilePathConfig { read_path }),
            write_path: None,
        };
        let airspace_config = AirspaceConfig {
            time_buffer_seconds: 1,
        };
        let pipeline_config = PipelineConfig {
            ingestor: ingestor_config,
            airspace: airspace_config,
        };
        let pipeline = AirspaceDataPipeline::setup_pipeline(pipeline_config);
        drop(pipeline);
    }
}
