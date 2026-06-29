use crate::core::airspace::{AirspaceStore, AirspaceViewer};
use crate::core::central_disk_logger::DiskLoggerRegistry;
use crate::core::central_disk_logger::errors::DiskloggerRegistryError;
use crate::core::ingestor::{AprsPacket, Ingestor, PbAprsPacket};
use crate::core::parser::protobuf::PbAircraft;
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

    pub fn setup_pipeline(
        pipeline_config: PipelineConfig,
    ) -> Result<Self, AircraftDataPipelineError> {
        let mut disk_logger_registry = DiskLoggerRegistry::new();

        let (ingestor_sender, ingestor_receiver): (
            crossbeam_channel::Sender<AprsPacket>,
            crossbeam_channel::Receiver<AprsPacket>,
        ) = crossbeam_channel::unbounded();

        let ingestor_logger_handle = pipeline_config
            .ingestor
            .write_path
            .map(|path| disk_logger_registry.register_proto::<PbAprsPacket>(path))
            .transpose()?;

        let ingestor = match pipeline_config.ingestor.source {
            IngestorSource::FilePath(FilePathConfig { read_path }) => {
                Ingestor::read_data_from_file(&read_path, ingestor_sender, ingestor_logger_handle)
            }
            IngestorSource::GliderNet(config) => {
                Ingestor::connect_glidernet(&config, ingestor_sender, ingestor_logger_handle)
            }
        }
        .map_err(|err| AircraftDataPipelineError::PipelineComponentSetup {
            struct_name: std::any::type_name::<Ingestor>(),
            source: err,
        })?;

        let (parser_sender, parser_receiver): (
            crossbeam_channel::Sender<Aircraft>,
            crossbeam_channel::Receiver<Aircraft>,
        ) = crossbeam_channel::unbounded();

        let parser_logger_handle = pipeline_config
            .parser
            .write_path
            .map(|path| disk_logger_registry.register::<PbAircraft>(path))
            .transpose()?;

        let parser = AircraftParser::new(ingestor_receiver, parser_sender, parser_logger_handle);

        let airspace_store = AirspaceStore::new(
            parser_receiver,
            chrono::TimeDelta::seconds(pipeline_config.airspace.time_buffer_seconds.into()),
        );
        let task_order: Vec<(Box<dyn SteppableTask>, std::time::Duration)> = vec![
            (Box::new(ingestor), std::time::Duration::ZERO),
            (Box::new(parser), std::time::Duration::ZERO),
        ];
        Ok(Self::new(
            task_order,
            airspace_store,
            std::time::Duration::from_micros(16667),
        ))
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
#[derive(Debug, thiserror::Error)]
pub enum AircraftDataPipelineError {
    #[error(
        "Failed to construct AircraftDataPipeline due to component initialization failure: {struct_name}. {source}"
    )]
    PipelineComponentSetup {
        struct_name: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to register to disk_logger : {0}")]
    CentralDiskLogger(#[from] DiskloggerRegistryError),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::ingestor::PbAprsPacket;
    use crate::pipeline::AirspaceDataPipeline;
    use crate::pipeline::config::{AircraftParserConfig, AirspaceConfig, IngestorConfig};
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
        let parser_config = AircraftParserConfig { write_path: None };
        let airspace_config = AirspaceConfig {
            time_buffer_seconds: 1,
        };
        let pipeline_config = PipelineConfig {
            ingestor: ingestor_config,
            airspace: airspace_config,
            parser: parser_config,
        };
        let pipeline = AirspaceDataPipeline::setup_pipeline(pipeline_config);
        drop(pipeline);
    }
}
