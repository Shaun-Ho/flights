use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::core::central_disk_logger::errors;
use crate::core::central_disk_logger::interface::{DiskLoggerMessage, LoggerTaskID};
use crate::core::thread_manager::{SteppableTask, TaskState};

#[derive(Debug)]
pub struct CentralDiskLogger {
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    id_to_path_writer_pair_mapping: HashMap<LoggerTaskID, (PathBuf, BufWriter<File>)>,
}
impl CentralDiskLogger {
    pub fn new(
        receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
        id_to_path_writer_pair_mapping: HashMap<LoggerTaskID, (PathBuf, BufWriter<File>)>,
    ) -> Self {
        Self {
            receiver,
            id_to_path_writer_pair_mapping,
        }
    }
}

impl SteppableTask for CentralDiskLogger {
    fn step(&mut self) -> TaskState {
        match self.receiver.try_recv() {
            Ok(message) => {
                match self
                    .id_to_path_writer_pair_mapping
                    .get_mut(&message.logger_id)
                    .ok_or(errors::CentralDiskLoggerError::TaskNotRegistered(
                        message.logger_id,
                    )) {
                    Ok((path, writer)) => {
                        let _ = writer.write_all(&message.payload).map_err(|err| {
                            let write_error = errors::CentralDiskLoggerError::WriteError {
                                path: path.clone(),
                                payload: message.payload,
                                source: err,
                            };
                            log::warn!("{write_error}")
                        });
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
    use super::*;
    use std::fs;

    #[test]
    fn given_valid_message_when_stepped_then_writes_payload_to_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("data_log.bin");
        let mut mapping = HashMap::new();
        let task_id = 42;
        mapping.insert(
            task_id,
            (
                file_path.clone(),
                BufWriter::new(File::create_new(&file_path).unwrap()),
            ),
        );

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_path_writer_pair_mapping: mapping,
            receiver,
        };

        let expected_payload = b"test payload bytes".to_vec();
        sender
            .send(DiskLoggerMessage {
                logger_id: task_id,
                payload: expected_payload.clone(),
            })
            .unwrap();

        let state = logger.step();

        assert!(matches!(state, TaskState::Running),);

        // We must drop the logger (and its BufWriter) to ensure the internal
        // buffer flushes its contents to the actual disk before we test reading it.
        drop(logger);

        let written_contents = fs::read(&file_path).unwrap();
        assert_eq!(written_contents, expected_payload,);
    }

    #[test]
    fn given_empty_channel_when_stepped_then_returns_running() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("empty_test.bin");
        let mut mapping = HashMap::new();
        mapping.insert(
            1,
            (
                file_path.clone(),
                BufWriter::new(File::create_new(&file_path).unwrap()),
            ),
        );

        let (_sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_path_writer_pair_mapping: mapping,
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
        mapping.insert(
            1,
            (
                file_path.clone(),
                BufWriter::new(File::create_new(&file_path).unwrap()),
            ),
        );

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_path_writer_pair_mapping: mapping,
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
        mapping.insert(
            1,
            (
                file_path.clone(),
                BufWriter::new(File::create_new(&file_path).unwrap()),
            ),
        );

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger {
            id_to_path_writer_pair_mapping: mapping,
            receiver,
        };

        sender
            .send(DiskLoggerMessage {
                logger_id: 99,
                payload: b"ghost payload".to_vec(),
            })
            .unwrap();

        let state = logger.step();

        assert!(matches!(state, TaskState::Running));
    }
}
