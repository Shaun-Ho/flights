use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::core::central_disk_logger::errors;
use crate::core::thread_manager::{SteppableTask, TaskState};

pub type LoggerTaskID = u8;

pub struct DiskLoggerMessage {
    pub logger_id: LoggerTaskID,
    pub payload: Vec<u8>,
}

pub struct CentralDiskLogger {
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    id_to_path_writer_pair_mapping: HashMap<LoggerTaskID, (PathBuf, BufWriter<File>)>,
}
impl CentralDiskLogger {
    pub fn new(
        id_to_path_mapping: HashMap<LoggerTaskID, PathBuf>,
        receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    ) -> Result<Self, std::io::Error> {
        let id_to_path_writer_pair_mapping = id_to_path_mapping
            .into_iter()
            .map(|(logger_id, path)| {
                let file = File::create_new(path.clone())?;
                Ok((logger_id, (path, BufWriter::new(file))))
            })
            .collect::<Result<HashMap<LoggerTaskID, (PathBuf, BufWriter<File>)>, std::io::Error>>(
            )?;

        Ok(Self {
            receiver,
            id_to_path_writer_pair_mapping,
        })
    }
}

impl SteppableTask for CentralDiskLogger {
    fn step(&mut self) -> TaskState {
        match self.receiver.try_recv() {
            Ok(message) => {
                match self
                    .id_to_path_writer_pair_mapping
                    .get_mut(&message.logger_id)
                    .ok_or(errors::CentralisedLoggerError::TaskNotRegistered(
                        message.logger_id,
                    )) {
                    Ok((path, writer)) => {
                        let _ = writer.write_all(&message.payload).map_err(|err| {
                            let write_error = errors::CentralisedLoggerError::WriteError {
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
    fn given_valid_paths_when_creating_logger_then_files_are_created() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_log_1.bin");
        let mut mapping = HashMap::new();
        mapping.insert(1, file_path.clone());

        let (_sender, receiver) = crossbeam_channel::unbounded();

        let logger = CentralDiskLogger::new(mapping, receiver);

        assert!(logger.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn given_existing_file_when_creating_logger_then_returns_io_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("already_exists.bin");
        fs::File::create(&file_path).unwrap();

        let mut mapping = HashMap::new();
        mapping.insert(1, file_path);

        let (_sender, receiver) = crossbeam_channel::unbounded();

        let logger_result = CentralDiskLogger::new(mapping, receiver);

        assert!(logger_result.is_err(),);
    }

    #[test]
    fn given_valid_message_when_stepped_then_writes_payload_to_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("data_log.bin");
        let mut mapping = HashMap::new();
        let task_id = 42;
        mapping.insert(task_id, file_path.clone());

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger::new(mapping, receiver).unwrap();

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
        mapping.insert(1, file_path);

        let (_sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger::new(mapping, receiver).unwrap();

        let state = logger.step();

        assert!(matches!(state, TaskState::Running),);
    }

    #[test]
    fn given_disconnected_channel_when_stepped_then_returns_completed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("disconnect_test.bin");
        let mut mapping = HashMap::new();
        mapping.insert(1, file_path);

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger::new(mapping, receiver).unwrap();

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
        mapping.insert(1, file_path);

        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut logger = CentralDiskLogger::new(mapping, receiver).unwrap();

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
