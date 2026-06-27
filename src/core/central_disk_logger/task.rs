use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::core::central_disk_logger::errors;
use crate::core::thread_manager::{SteppableTask, TaskState};
use crate::ext::TryInsertExt;

pub type LoggerTaskID = u8;

#[derive(Debug)]
pub struct DiskLoggerMessage {
    pub logger_id: LoggerTaskID,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct LoggerHandle {
    #[allow(unused)]
    logger_id: LoggerTaskID,
    #[allow(unused)]
    sender: crossbeam_channel::Sender<DiskLoggerMessage>,
}

#[derive(Debug)]
pub struct DiskLoggerRegistry {
    current_logger_id: LoggerTaskID,
    sender: crossbeam_channel::Sender<DiskLoggerMessage>,
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    task_to_path_mapping: HashMap<LoggerTaskID, (PathBuf, BufWriter<File>)>,
}
impl DiskLoggerRegistry {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            current_logger_id: 0,
            sender,
            receiver,
            task_to_path_mapping: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        path: PathBuf,
    ) -> Result<LoggerHandle, errors::DiskloggerRegistryError> {
        let file = match File::create_new(&path) {
            Ok(f) => f,
            Err(err) => {
                return Err(errors::DiskloggerRegistryError::LogFileCreationError {
                    path,
                    source: err,
                });
            }
        };
        let writer = BufWriter::new(file);

        let logger_id = self.current_logger_id;
        let _ = TryInsertExt::try_insert(&mut self.task_to_path_mapping, logger_id, (path, writer))
            .map_err(|err| {
                let (rejected_path, _rejected_writer) = err.value;
                errors::DiskloggerRegistryError::PathAlreadyRegisteredError(rejected_path)
            })?;

        let handle = LoggerHandle {
            logger_id,
            sender: self.sender.clone(),
        };
        self.current_logger_id += 1;

        Ok(handle)
    }

    pub fn build(self) -> CentralDiskLogger {
        CentralDiskLogger {
            id_to_path_writer_pair_mapping: self.task_to_path_mapping,
            receiver: self.receiver,
        }
    }
}

#[derive(Debug)]
pub struct CentralDiskLogger {
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    id_to_path_writer_pair_mapping: HashMap<LoggerTaskID, (PathBuf, BufWriter<File>)>,
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

    mod disk_logger_registry {
        use super::*;
        use std::fs;

        #[test]
        fn given_valid_paths_when_creating_logger_then_files_are_created() {
            let temp_dir = tempfile::tempdir().unwrap();
            let file_path = temp_dir.path().join("test_log_1.bin");

            let mut registry = DiskLoggerRegistry::new();
            let handle = registry.register(file_path.clone());
            assert!(handle.is_ok());
            assert!(file_path.exists());
        }

        #[test]
        fn given_existing_file_when_creating_logger_then_returns_io_error() {
            let temp_dir = tempfile::tempdir().unwrap();
            let file_path = temp_dir.path().join("already_exists.bin");
            fs::File::create(&file_path).unwrap();

            let mut register = DiskLoggerRegistry::new();
            let handle = register.register(file_path.clone());

            assert!(handle.is_err());
        }
    }

    mod central_disk_logger {
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
}
