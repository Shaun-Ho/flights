//! Test-only message types, one per log format, shared by the logger and reader tests.

use std::convert::Infallible;

use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logging::errors::LoggingError;
use crate::logging::{IntoLogMessage, JsonFormat, ProtoFileDescriptor, ProtoFormat};
use crate::reader::traits::ReplayableMessage;

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

/// The read-side mirror of `into_message`: a stored value of zero or less is rejected.
#[derive(Debug, thiserror::Error)]
#[error("larger_than_zero must be greater than zero, got {0}")]
pub struct NotPositive(pub i32);

impl TryFrom<MockTaskProto> for MockTaskStruct {
    type Error = NotPositive;
    fn try_from(stored: MockTaskProto) -> Result<Self, Self::Error> {
        if stored.larger_than_zero <= 0 {
            Err(NotPositive(stored.larger_than_zero))
        } else {
            Ok(Self {
                larger_than_zero: stored.larger_than_zero,
            })
        }
    }
}

impl ReplayableMessage<ProtoFormat> for MockTaskStruct {
    type Message = MockTaskProto;
    type Error = NotPositive;

    fn from_stored(stored: MockTaskProto) -> Result<Self, Self::Error> {
        stored.try_into()
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
    /// Deliberately not a real descriptor: the reader never compares schema bytes.
    const FILE_DESCRIPTOR_SET: &'static [u8] = &[1, 2];
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
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

impl ReplayableMessage<JsonFormat> for MockJsonMessage {
    type Message = MockJsonMessage;
    type Error = Infallible;

    fn from_stored(stored: MockJsonMessage) -> Result<Self, Self::Error> {
        Ok(stored)
    }
}
