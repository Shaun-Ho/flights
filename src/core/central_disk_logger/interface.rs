use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::marker::PhantomData;
use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::core::central_disk_logger::errors::{
    DiskloggerRegistryError, JsonLoggingError, ProtoLoggingError,
};
use crate::core::central_disk_logger::traits::{JsonToMcapSchema, ProtoToMcapSchema};
use crate::core::central_disk_logger::{CentralDiskLogger, IntoLogMessage, LogSender};
use crate::ext::TryInsertExt;

pub type LoggerID = u8;

const PROTO_FILE_FORMAT: &str = "pb";
const JSONL_FILE_FORMAT: &str = "jsonl";

#[derive(Debug)]
pub struct DiskLoggerMessage {
    pub logger_id: LoggerID,
    pub publish_timestamp: DateTime<Utc>,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct LoggerHandle<F, M: ?Sized> {
    logger_id: LoggerID,
    sender: crossbeam_channel::Sender<DiskLoggerMessage>,
    _marker: PhantomData<(F, M)>,
}

impl<F, M> LoggerHandle<F, M> {
    pub fn new(logger_id: LoggerID, sender: crossbeam_channel::Sender<DiskLoggerMessage>) -> Self {
        Self {
            logger_id,
            sender,
            _marker: PhantomData,
        }
    }

    pub fn logger_id(&self) -> LoggerID {
        self.logger_id
    }
}

impl<M, T> LogSender<T> for LoggerHandle<ProtoFormat, M>
where
    T: IntoLogMessage<M>,
    M: prost::Message + ProtoToMcapSchema,
    ProtoLoggingError<T>: From<T::Error>,
{
    type Error = ProtoLoggingError<T>;

    fn send(&self, message: T) -> Result<(), ProtoLoggingError<T>> {
        let publish_timestamp = message.message_timestamp();
        let proto_message: M = message.into_message()?;

        let payload = proto_message.encode_length_delimited_to_vec();

        Ok(self.sender.send(DiskLoggerMessage {
            logger_id: self.logger_id,
            publish_timestamp,
            payload,
        })?)
    }
}

impl<M, T> LogSender<T> for LoggerHandle<JsonFormat, M>
where
    T: IntoLogMessage<M>,
    M: serde::Serialize + JsonToMcapSchema,
    JsonLoggingError<T>: From<T::Error>,
{
    type Error = JsonLoggingError<T>;

    fn send(&self, message: T) -> Result<(), JsonLoggingError<T>> {
        let publish_timestamp = message.message_timestamp();
        let json_message: M = message.into_message()?;

        let mut payload =
            serde_json::to_vec(&json_message).map_err(JsonLoggingError::Serialization)?;

        payload.push(b'\n');

        self.sender
            .send(DiskLoggerMessage {
                logger_id: self.logger_id,
                publish_timestamp,
                payload,
            })
            .map_err(JsonLoggingError::SendError)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct DiskLoggerRegistry {
    current_logger_id: LoggerID,
    sender: crossbeam_channel::Sender<DiskLoggerMessage>,
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    task_to_path_mapping: HashMap<LoggerID, (PathBuf, BufWriter<File>)>,
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
    ) -> Result<LoggerHandle<ProtoFormat, M>, DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != PROTO_FILE_FORMAT) {
            return Err(DiskloggerRegistryError::InvalidPath(path));
        }
        self.register::<ProtoFormat, M>(path)
    }

    pub fn register_jsonl<M>(
        &mut self,
        path: PathBuf,
    ) -> Result<LoggerHandle<JsonFormat, M>, DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != JSONL_FILE_FORMAT) {
            return Err(DiskloggerRegistryError::InvalidPath(path));
        }
        self.register::<JsonFormat, M>(path)
    }

    pub fn build(self) -> CentralDiskLogger {
        CentralDiskLogger::new(self.receiver, self.task_to_path_mapping)
    }

    fn register<F, M>(
        &mut self,
        path: PathBuf,
    ) -> Result<LoggerHandle<F, M>, DiskloggerRegistryError> {
        let file = match File::create_new(&path) {
            Ok(f) => f,
            Err(err) => {
                return Err(DiskloggerRegistryError::LogFileCreationError { path, source: err });
            }
        };
        let writer = BufWriter::new(file);

        let logger_id = self.current_logger_id;
        let _ = TryInsertExt::try_insert(&mut self.task_to_path_mapping, logger_id, (path, writer))
            .map_err(|err| {
                let (rejected_path, _rejected_writer) = err.value;
                DiskloggerRegistryError::PathAlreadyRegisteredError(rejected_path)
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

pub struct JsonFormat;
pub struct ProtoFormat;

pub type ProtoLoggerHandle<M> = LoggerHandle<ProtoFormat, M>;
pub type JsonlLoggerHandle<M> = LoggerHandle<JsonFormat, M>;

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
                ProtoLoggingError::Conversion(_message)
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
            assert!(matches!(res.unwrap_err(), ProtoLoggingError::SendError(_)));
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
