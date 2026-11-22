use std::convert::Infallible;

use chrono::Utc;
use schemars::JsonSchema;
use serde::Serialize;

use crate::logging::errors::LoggingError;
use crate::logging::{IntoLogMessage, ProtoFileDescriptor};

#[derive(Clone, Debug, PartialEq)]
pub struct MockTaskStruct {
    pub larger_than_zero: i32,
}
impl IntoLogMessage<MockTaskProto> for MockTaskStruct {
    type Error = LoggingError<MockTaskStruct>;

    fn message_timestamp(&self) -> chrono::prelude::DateTime<chrono::prelude::Utc> {
        Utc::now()
    }
    fn into_message(self) -> Result<MockTaskProto, Self::Error> {
        if self.larger_than_zero <= 0 {
            Err(LoggingError::Conversion(self))
        } else {
            Ok(MockTaskProto {
                larger_than_zero: self.larger_than_zero,
            })
        }
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct MockTaskProto {
    #[prost(int32, tag = "1")]
    pub larger_than_zero: i32,
}
impl prost::Name for MockTaskProto {
    const NAME: &'static str = "MockTaskProto";
    const PACKAGE: &'static str = "test";
}
impl ProtoFileDescriptor for MockTaskProto {
    const FILE_DESCRIPTOR_SET: &'static [u8] = &[1, 2];
}

#[derive(Debug, Serialize, JsonSchema, Clone)]
pub struct MockJsonMessage {
    pub contents: Vec<String>,
}
impl IntoLogMessage<MockJsonMessage> for MockJsonMessage {
    type Error = Infallible;
    fn into_message(self) -> Result<MockJsonMessage, Self::Error> {
        Ok(self)
    }
    fn message_timestamp(&self) -> chrono::prelude::DateTime<chrono::prelude::Utc> {
        Utc::now()
    }
}
