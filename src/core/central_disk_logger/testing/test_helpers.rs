use chrono::Utc;

use crate::core::central_disk_logger::{IntoLogMessage, McapSchemaDescriptor, ProtoLoggingError};

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

#[derive(Debug, PartialEq)]
pub struct MockConversionError;

#[derive(Clone, PartialEq, prost::Message)]
pub struct MockTaskProto {
    #[prost(int32, tag = "1")]
    pub larger_than_zero: i32,
}
impl McapSchemaDescriptor for MockTaskProto {
    fn encoding() -> &'static str {
        "protobuf"
    }
    fn schema_bytes() -> Vec<u8> {
        b"mock_protobuf_descriptor_set_bytes".to_vec()
    }
    fn schema_name() -> String {
        "tests.MockTaskProto".to_string()
    }
    fn topic() -> String {
        "/test/mock_task".to_string()
    }
}
