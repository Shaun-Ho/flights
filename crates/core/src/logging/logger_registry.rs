use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::common::{JsonFormat, LogFormat, MCAP_FILE_SUFFIX, ProtoFormat};
use crate::ext::TryInsertExt;
use crate::logging::centralized_logger::{CentralizedLogger, WriteTarget};
use crate::logging::errors::{DiskloggerRegistryError, LoggingError};
use crate::logging::traits::{EncodableMessage, IntoLogMessage, LogSender, mcap_schema};

pub type LoggerID = u8;

#[derive(Debug)]
pub struct LoggerMessage {
    pub logger_id: LoggerID,
    pub publish_timestamp: DateTime<Utc>,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct LoggerHandle<F, M: Sized> {
    logger_id: LoggerID,
    sender: crossbeam_channel::Sender<LoggerMessage>,
    _marker: PhantomData<(F, M)>,
}

impl<F, M> LoggerHandle<F, M> {
    pub fn new(logger_id: LoggerID, sender: crossbeam_channel::Sender<LoggerMessage>) -> Self {
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

impl<F, M, T> LogSender<T> for LoggerHandle<F, M>
where
    F: LogFormat,
    M: EncodableMessage<F>,
    T: IntoLogMessage<M>,
    LoggingError<T>: From<T::Error>,
{
    type Error = LoggingError<T>;

    fn send(&self, message: T) -> Result<(), LoggingError<T>> {
        let publish_timestamp = message.message_timestamp();
        let payload = message.into_message()?.encode_payload()?;

        Ok(self.sender.send(LoggerMessage {
            logger_id: self.logger_id,
            publish_timestamp,
            payload,
        })?)
    }
}

pub struct LoggerRegistry {
    current_logger_id: LoggerID,
    sender: crossbeam_channel::Sender<LoggerMessage>,
    receiver: crossbeam_channel::Receiver<LoggerMessage>,
    id_to_target_mapping: HashMap<LoggerID, WriteTarget>,
}
impl LoggerRegistry {
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            current_logger_id: 0,
            sender,
            receiver,
            id_to_target_mapping: HashMap::new(),
        }
    }

    pub fn register_proto<M: EncodableMessage<ProtoFormat>>(
        &mut self,
        path: PathBuf,
        topic: String,
    ) -> Result<LoggerHandle<ProtoFormat, M>, DiskloggerRegistryError> {
        self.register::<ProtoFormat, M>(path, topic)
    }

    pub fn register_jsonl<M: EncodableMessage<JsonFormat>>(
        &mut self,
        path: PathBuf,
        topic: String,
    ) -> Result<LoggerHandle<JsonFormat, M>, DiskloggerRegistryError> {
        self.register::<JsonFormat, M>(path, topic)
    }

    pub fn register<F: LogFormat, M: EncodableMessage<F>>(
        &mut self,
        path: PathBuf,
        topic: String,
    ) -> Result<LoggerHandle<F, M>, DiskloggerRegistryError> {
        if path.extension().is_none_or(|ext| ext != MCAP_FILE_SUFFIX) {
            return Err(DiskloggerRegistryError::InvalidPath(path));
        }
        let channel = Arc::new(mcap::Channel {
            id: 0,
            topic,
            schema: Some(Arc::new(mcap_schema::<F, M>())),
            message_encoding: F::MESSAGE_ENCODING.to_string(),
            metadata: BTreeMap::new(),
        });
        self.register_channel::<F, M>(path, channel)
    }

    #[must_use]
    pub fn build(self) -> CentralizedLogger {
        CentralizedLogger::new(self.receiver, self.id_to_target_mapping)
    }

    fn register_channel<F, M>(
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
impl Default for LoggerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub type ProtoLoggerHandle<M> = LoggerHandle<ProtoFormat, M>;
pub type JsonlLoggerHandle<M> = LoggerHandle<JsonFormat, M>;

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::logging::testing::test_helpers::*;

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
                LoggingError::Conversion(_message)
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
            assert!(matches!(res.unwrap_err(), LoggingError::SendError(_)));
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
                schema: Some(Arc::new(mcap_schema::<ProtoFormat, MockTaskProto>())),
                message_encoding: ProtoFormat::MESSAGE_ENCODING.to_string(),
                metadata: BTreeMap::new(),
            });

            let mut registry = LoggerRegistry::new();
            let handle =
                registry.register_channel::<ProtoFormat, MockTaskProto>(file_path.clone(), channel);
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
                schema: Some(Arc::new(mcap_schema::<ProtoFormat, MockTaskProto>())),
                message_encoding: ProtoFormat::MESSAGE_ENCODING.to_string(),
                metadata: BTreeMap::new(),
            });

            let mut register = LoggerRegistry::new();
            let handle =
                register.register_channel::<ProtoFormat, MockTaskProto>(file_path.clone(), channel);

            assert!(handle.is_err());
        }
    }
}
