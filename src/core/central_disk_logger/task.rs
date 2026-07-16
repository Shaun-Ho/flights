use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::central_disk_logger::errors;
use crate::core::central_disk_logger::interface::{DiskLoggerMessage, LoggerID};
use crate::core::thread_manager::{SteppableTask, TaskState};

pub struct WriteTarget {
    pub path: PathBuf,
    pub writer: mcap::Writer<BufWriter<File>>,
    pub channel: Arc<mcap::Channel<'static>>,
}
pub struct CentralDiskLogger {
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    id_to_target_mapping: HashMap<LoggerID, WriteTarget>,
}
impl CentralDiskLogger {
    pub fn new(
        receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
        id_to_target_mapping: HashMap<LoggerID, WriteTarget>,
    ) -> Self {
        Self {
            receiver,
            id_to_target_mapping,
        }
    }
}

impl SteppableTask for CentralDiskLogger {
    fn step(&mut self) -> TaskState {
        match self.receiver.try_recv() {
            Ok(message) => {
                match self.id_to_target_mapping.get_mut(&message.logger_id).ok_or(
                    errors::CentralDiskLoggerError::TaskNotRegistered(message.logger_id),
                ) {
                    Ok(target) => {
                        let mcap_message = mcap::Message {
                            channel: target.channel.clone(),
                            sequence: 0,
                            log_time: message.publish_timestamp.timestamp_nanos_opt().unwrap()
                                as u64,
                            publish_time: message.publish_timestamp.timestamp_nanos_opt().unwrap()
                                as u64,
                            data: message.payload.into(),
                        };
                        if let Err(err) = target
                            .writer
                            .write(&mcap_message)
                            .map_err(errors::CentralDiskLoggerError::WriteError)
                        {
                            log::warn!("{err}")
                        }
                    }
                    Err(err) => log::warn!("{err}"),
                };
                TaskState::Running
            }
            Err(crossbeam_channel::TryRecvError::Empty) => TaskState::Running,
            Err(crossbeam_channel::TryRecvError::Disconnected) => TaskState::Completed,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;
    use prost::Message;

    use crate::core::central_disk_logger::{
        ProtoToMcapSchema, testing::test_helpers::MockTaskProto,
    };

    use super::*;

    #[test]
    fn given_valid_message_when_stepped_then_writes_payload_to_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("data_log.bin");
        let mut mapping = HashMap::new();
        let task_id = 42;

        let expected_payload = MockTaskProto {
            larger_than_zero: 1,
        };

        let writer =
            mcap::Writer::new(BufWriter::new(File::create_new(&file_path).unwrap())).unwrap();

        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic: "topic".to_string(),
            schema: Some(Arc::new(MockTaskProto::translate_schema())),
            message_encoding: "protobuf".to_string(),
            metadata: BTreeMap::new(),
        });
        mapping.insert(
            task_id,
            WriteTarget {
                path: file_path.clone(),
                writer,
                channel,
            },
        );

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_target_mapping: mapping,
            receiver,
        };

        sender
            .send(DiskLoggerMessage {
                logger_id: task_id,
                publish_timestamp: Utc::now(),
                payload: expected_payload.encode_length_delimited_to_vec(),
            })
            .unwrap();

        let state = logger.step();

        assert!(matches!(state, TaskState::Running),);

        // We must drop the logger (and its McapWriter) to ensure the internal
        // buffer flushes its contents to the actual disk before we test reading it.
        drop(logger);

        let contents = std::fs::read(&file_path).unwrap();
        let mut mcap_stream = mcap::MessageStream::new(&contents).unwrap();

        // Pull the first message from the MCAP file
        let mcap_message = mcap_stream
            .next()
            .expect("Expected at least one MCAP message in the file")
            .unwrap();

        // Check that the data stripped from the MCAP framing matches our raw protobuf bytes
        assert_eq!(
            mcap_message.data,
            expected_payload.encode_length_delimited_to_vec()
        );

        assert!(mcap_stream.next().is_none());
    }

    #[test]
    fn given_empty_channel_when_stepped_then_returns_running() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("empty_test.bin");
        let mut mapping = HashMap::new();

        let logger_id = 1;
        let writer =
            mcap::Writer::new(BufWriter::new(File::create_new(&file_path).unwrap())).unwrap();
        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic: "topic".to_string(),
            schema: Some(Arc::new(MockTaskProto::translate_schema())),
            message_encoding: "protobuf".to_string(),
            metadata: BTreeMap::new(),
        });
        mapping.insert(
            logger_id,
            WriteTarget {
                path: file_path.clone(),
                writer,
                channel,
            },
        );

        let (_sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_target_mapping: mapping,
            receiver,
        };

        let state = logger.step();

        assert!(matches!(state, TaskState::Running),);
    }

    #[test]
    fn given_disconnected_channel_when_stepped_then_returns_completed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("disconnect_test.bin");
        let mut mapping = HashMap::new();

        let logger_id = 1;
        let writer =
            mcap::Writer::new(BufWriter::new(File::create_new(&file_path).unwrap())).unwrap();
        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic: "topic".to_string(),
            schema: Some(Arc::new(MockTaskProto::translate_schema())),
            message_encoding: "protobuf".to_string(),
            metadata: BTreeMap::new(),
        });
        mapping.insert(
            logger_id,
            WriteTarget {
                path: file_path.clone(),
                writer,
                channel,
            },
        );

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_target_mapping: mapping,
            receiver,
        };

        // Explicitly drop the sender to disconnect the channel
        drop(sender);

        let state = logger.step();

        assert!(matches!(state, TaskState::Completed),);
    }

    #[test]
    fn given_unregistered_logger_id_when_stepped_then_ignores_and_returns_running() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("unregistered_test.bin");
        let mut mapping = HashMap::new();

        let logger_id = 1;
        let writer =
            mcap::Writer::new(BufWriter::new(File::create_new(&file_path).unwrap())).unwrap();
        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic: "topic".to_string(),
            schema: Some(Arc::new(MockTaskProto::translate_schema())),
            message_encoding: "protobuf".to_string(),
            metadata: BTreeMap::new(),
        });
        mapping.insert(
            logger_id,
            WriteTarget {
                path: file_path.clone(),
                writer,
                channel,
            },
        );

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_target_mapping: mapping,
            receiver,
        };

        sender
            .send(DiskLoggerMessage {
                logger_id: 99,
                publish_timestamp: Utc::now(),
                payload: b"ghost payload".to_vec(),
            })
            .unwrap();

        let state = logger.step();

        assert!(matches!(state, TaskState::Running));
    }
}
