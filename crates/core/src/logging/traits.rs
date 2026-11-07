use std::borrow::Cow;

use chrono::{DateTime, Utc};

use crate::common::{JsonFormat, LogFormat, ProtoFormat};

pub trait LogSender<M> {
    type Error;
    fn send(&self, message: M) -> Result<(), Self::Error>;
}

pub trait IntoLogMessage<M> {
    type Error;
    fn into_message(self) -> Result<M, Self::Error>;
    fn message_timestamp(&self) -> DateTime<Utc>;
}

pub trait ProtoToMcapSchema {
    fn translate_schema() -> mcap::Schema<'static>;
}
pub trait JsonToMcapSchema {
    fn translate_schema() -> mcap::Schema<'static>;
}

/// Provides the encoded `FileDescriptorSet` containing a protobuf message's definition,
/// which is embedded as the MCAP schema for that message.
pub trait ProtoFileDescriptor {
    const FILE_DESCRIPTOR_SET: &'static [u8];
}
impl<T: prost::Name + ProtoFileDescriptor> ProtoToMcapSchema for T {
    fn translate_schema() -> mcap::Schema<'static> {
        mcap::Schema {
            id: 1,
            name: format!("{}.{}", T::PACKAGE, T::NAME),
            encoding: ProtoFormat::SCHEMA_ENCODING.to_string(),
            data: T::FILE_DESCRIPTOR_SET.into(),
        }
    }
}
impl<T: schemars::JsonSchema> JsonToMcapSchema for T {
    fn translate_schema() -> mcap::Schema<'static> {
        let schema = schemars::schema_for!(T);
        let schema_bytes =
            serde_json::to_vec(&schema).expect("Failed to serialize JsonSchema to bytes");

        let schema_name = std::any::type_name::<T>().replace("::", ".");
        mcap::Schema {
            id: 1,
            name: schema_name,
            encoding: JsonFormat::SCHEMA_ENCODING.to_string(),
            data: Cow::Owned(schema_bytes),
        }
    }
}
