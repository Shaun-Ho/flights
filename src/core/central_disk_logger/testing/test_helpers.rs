use chrono::Utc;

use crate::core::central_disk_logger::errors::ProtoLoggingError;
use crate::core::central_disk_logger::{IntoLogMessage, ProtoToMcapSchema};

#[derive(Clone, Debug, PartialEq)]
pub struct MockTaskStruct {
    pub larger_than_zero: i32,
}
impl IntoLogMessage<MockTaskProto> for MockTaskStruct {
    type Error = ProtoLoggingError<MockTaskStruct>;

    fn message_timestamp(&self) -> chrono::prelude::DateTime<chrono::prelude::Utc> {
        Utc::now()
    }
    fn into_message(self) -> Result<MockTaskProto, Self::Error> {
        if self.larger_than_zero <= 0 {
            Err(ProtoLoggingError::Conversion(self))
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
impl ProtoToMcapSchema for MockTaskProto {
    fn translate_schema() -> mcap::Schema<'static> {
        mcap::Schema {
            id: 1,
            name: "test".to_string(),
            encoding: "protobuf".to_string(),
            data: (&[1, 2]).into(),
        }
    }
}
