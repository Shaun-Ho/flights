use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::core::central_disk_logger::errors::{
    DiskloggerRegistryError, JsonLoggingError, ProtoLoggingError,
};
use crate::core::central_disk_logger::task::{CentralDiskLogger, WriteTarget};
use crate::core::central_disk_logger::traits::{
    IntoLogMessage, JsonToMcapSchema, LogSender, ProtoToMcapSchema,
};
use crate::ext::TryInsertExt;

pub type LoggerID = u8;

const MCAP_FILE_SUFFIX: &str = "mcap";

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

        let payload = serde_json::to_vec(&json_message).map_err(JsonLoggingError::Serialization)?;

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

pub struct DiskLoggerRegistry {
    current_logger_id: LoggerID,
    sender: crossbeam_channel::Sender<DiskLoggerMessage>,
    receiver: crossbeam_channel::Receiver<DiskLoggerMessage>,
    id_to_target_mapping: HashMap<LoggerID, WriteTarget>,
}
impl DiskLoggerRegistry {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            current_logger_id: 0,
            sender,
            receiver,
            id_to_target_mapping: HashMap::new(),
        }
    }

    pub fn register_proto<M: ProtoToMcapSchema>(
        &mut self,
        path: PathBuf,
        topic: String,
    ) -> Result<LoggerHandle<ProtoFormat, M>, DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != MCAP_FILE_SUFFIX) {
            return Err(DiskloggerRegistryError::InvalidPath(path));
        }
        let schema = M::translate_schema();
        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic,
            schema: Some(Arc::new(schema)),
            message_encoding: "protobuf".to_string(),
            metadata: BTreeMap::new(),
        });
        self.register::<ProtoFormat, M>(path, channel)
    }

    pub fn register_jsonl<M: JsonToMcapSchema>(
        &mut self,
        path: PathBuf,
        topic: String,
    ) -> Result<LoggerHandle<JsonFormat, M>, DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != MCAP_FILE_SUFFIX) {
            return Err(DiskloggerRegistryError::InvalidPath(path));
        }

        let schema = M::translate_schema();

        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic,
            schema: Some(Arc::new(schema)),
            message_encoding: "json".to_string(),
            metadata: BTreeMap::new(),
        });

        self.register::<JsonFormat, M>(path, channel)
    }

    pub fn build(self) -> CentralDiskLogger {
        CentralDiskLogger::new(self.receiver, self.id_to_target_mapping)
    }

    fn register<F, M>(
        &mut self,
        path: PathBuf,
        channel: Arc<mcap::Channel<'static>>,
    ) -> Result<LoggerHandle<F, M>, DiskloggerRegistryError> {
        let file = File::create_new(&path).map_err(|err| {
            DiskloggerRegistryError::LogFileCreationError {
                path: path.clone(),
                source: err,
            }
        })?;
        let writer = mcap::Writer::new(BufWriter::new(file))
            .map_err(DiskloggerRegistryError::WriterCreationError)?;

        let logger_id = self.current_logger_id;

        let _ = TryInsertExt::try_insert(
            &mut self.id_to_target_mapping,
            logger_id,
            WriteTarget {
                path,
                writer,
                channel,
            },
        )
        .map_err(|err| {
            let target = err.value;
            DiskloggerRegistryError::PathAlreadyRegisteredError(target.path)
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

    mod disk_logger_registry {
        use super::*;
        use testdir;

        #[test]
        fn given_valid_paths_when_creating_logger_then_files_are_created() {
            let test_dir = testdir::testdir!();
            let file_path = test_dir.join("test_log_1.mcap");

            let channel = Arc::new(mcap::Channel {
                id: 0,
                topic: "test".to_string(),
                schema: Some(Arc::new(MockTaskProto::translate_schema())),
                message_encoding: "protobuf".to_string(),
                metadata: BTreeMap::new(),
            });

            let mut registry = DiskLoggerRegistry::new();
            let handle =
                registry.register::<ProtoFormat, MockTaskProto>(file_path.clone(), channel);
            assert!(handle.is_ok());
            assert!(file_path.exists());
        }

        #[test]
        fn given_existing_file_when_creating_logger_then_returns_io_error() {
            let test_dir = testdir::testdir!();
            let file_path = test_dir.join("already_exists.bin");
            fs::File::create(&file_path).unwrap();

            let channel = Arc::new(mcap::Channel {
                id: 0,
                topic: "test".to_string(),
                schema: Some(Arc::new(MockTaskProto::translate_schema())),
                message_encoding: "protobuf".to_string(),
                metadata: BTreeMap::new(),
            });

            let mut register = DiskLoggerRegistry::new();
            let handle =
                register.register::<ProtoFormat, MockTaskProto>(file_path.clone(), channel);

            assert!(handle.is_err());
        }
    }
}
