use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::core::central_disk_logger::errors;
use crate::core::central_disk_logger::task::CentralDiskLogger;
use crate::ext::TryInsertExt;

pub type LoggerTaskID = u8;

const PROTO_FILE_FORMAT: &str = "pb";
const JSONL_FILE_FORMAT: &str = "jsonl";
#[derive(Debug)]
pub struct DiskLoggerMessage {
    pub logger_id: LoggerTaskID,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct LoggerHandle<F, M: ?Sized> {
    logger_id: LoggerTaskID,
    sender: crossbeam_channel::Sender<DiskLoggerMessage>,
    _marker: PhantomData<(F, M)>,
}

pub trait LogSender<Input> {
    type Error;
    fn send(&self, message: Input) -> Result<(), Self::Error>;
}

impl<F, M> LoggerHandle<F, M> {
    pub fn new(
        logger_id: LoggerTaskID,
        sender: crossbeam_channel::Sender<DiskLoggerMessage>,
    ) -> Self {
        Self {
            logger_id,
            sender,
            _marker: PhantomData,
        }
    }

    pub fn logger_id(&self) -> LoggerTaskID {
        self.logger_id
    }
}

impl<M, T, E> LogSender<T> for LoggerHandle<ProtoFormat, M>
where
    T: TryInto<M, Error = E>,
    M: prost::Message,
{
    type Error = errors::LoggingError<E>;

    fn send(&self, message: T) -> Result<(), Self::Error> {
        let proto_message: M = message
            .try_into()
            .map_err(errors::LoggingError::Conversion)?;

        let payload = proto_message.encode_length_delimited_to_vec();

        Ok(self.sender.send(DiskLoggerMessage {
            logger_id: self.logger_id,
            payload,
        })?)
    }
}

impl<'a, M> LogSender<&'a M> for LoggerHandle<JsonlFormat, M>
where
    M: serde::Serialize + ?Sized,
{
    type Error = errors::LoggingError<serde_json::Error>;

    fn send(&self, message: &'a M) -> Result<(), Self::Error> {
        let mut payload = serde_json::to_vec(message).map_err(errors::LoggingError::Conversion)?;

        payload.push(b'\n');

        self.sender
            .send(DiskLoggerMessage {
                logger_id: self.logger_id,
                payload,
            })
            .map_err(errors::LoggingError::SendError)?;

        Ok(())
    }
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

    pub fn register_proto<M>(
        &mut self,
        path: PathBuf,
    ) -> Result<LoggerHandle<ProtoFormat, M>, errors::DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != PROTO_FILE_FORMAT) {
            return Err(errors::DiskloggerRegistryError::InvalidPath(path));
        }
        self.register::<ProtoFormat, M>(path)
    }

    pub fn register_jsonl<M>(
        &mut self,
        path: PathBuf,
    ) -> Result<LoggerHandle<JsonlFormat, M>, errors::DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != JSONL_FILE_FORMAT) {
            return Err(errors::DiskloggerRegistryError::InvalidPath(path));
        }
        self.register::<JsonlFormat, M>(path)
    }

    pub fn build(self) -> CentralDiskLogger {
        CentralDiskLogger::new(self.receiver, self.task_to_path_mapping)
    }

    fn register<F, M>(
        &mut self,
        path: PathBuf,
    ) -> Result<LoggerHandle<F, M>, errors::DiskloggerRegistryError> {
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
            _marker: PhantomData,
        };
        self.current_logger_id += 1;

        Ok(handle)
    }
}
impl Default for DiskLoggerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct JsonlFormat {}
pub struct ProtoFormat {}

pub type ProtoLoggerHandle<M> = LoggerHandle<ProtoFormat, M>;
pub type JsonlLoggerHandle<M> = LoggerHandle<JsonlFormat, M>;

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::central_disk_logger::testing::test_helpers::*;

    mod logger_handle {
        use super::*;

        #[test]
        fn given_valid_conversion_from_rust_to_proto_struct_then_send_is_ok() {
            let message = MockTaskStruct {
                larger_than_zero: 1,
            };
            let (sender, receiver) = crossbeam_channel::unbounded();
            let handler = LoggerHandle::<ProtoFormat, MockTaskProto>::new(1, sender);
            let res = handler.send(message);
            assert!(receiver.try_recv().is_ok());
            assert!(res.is_ok());
        }

        #[test]
        fn given_invalid_conversion_from_rust_to_proto_struct_then_send_returns_correct_error() {
            let message = MockTaskStruct {
                larger_than_zero: 0,
            };
            let (sender, receiver) = crossbeam_channel::unbounded();
            let handler = LoggerHandle::<ProtoFormat, MockTaskProto>::new(1, sender);
            let res = handler.send(message);
            assert!(matches!(
                res.err().unwrap(),
                errors::LoggingError::Conversion(MockConversionError)
            ));
            assert!(receiver.try_recv().is_err());
        }
        #[test]
        fn given_valid_conversion_when_channel_dropped_then_send_return_correct_error() {
            let message = MockTaskStruct {
                larger_than_zero: 1,
            };
            let (sender, receiver) = crossbeam_channel::unbounded();
            let handler = LoggerHandle::<ProtoFormat, MockTaskProto>::new(1, sender);
            drop(receiver);
            let res = handler.send(message);
            assert!(matches!(
                res.unwrap_err(),
                errors::LoggingError::SendError(_)
            ));
        }
    }

    mod disk_logger_registrt {
        use super::*;

        #[test]
        fn given_valid_paths_when_creating_logger_then_files_are_created() {
            let temp_dir = tempfile::tempdir().unwrap();
            let file_path = temp_dir.path().join("test_log_1.bin");

            let mut registry = DiskLoggerRegistry::new();
            let handle = registry.register::<ProtoFormat, MockTaskProto>(file_path.clone());
            assert!(handle.is_ok());
            assert!(file_path.exists());
        }

        #[test]
        fn given_existing_file_when_creating_logger_then_returns_io_error() {
            let temp_dir = tempfile::tempdir().unwrap();
            let file_path = temp_dir.path().join("already_exists.bin");
            fs::File::create(&file_path).unwrap();

            let mut register = DiskLoggerRegistry::new();
            let handle = register.register::<ProtoFormat, MockTaskProto>(file_path.clone());

            assert!(handle.is_err());
        }
    }
}
