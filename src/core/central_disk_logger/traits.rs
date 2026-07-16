use std::borrow::Cow;

use chrono::{DateTime, Utc};

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

use crate::pb::ALL_PROTOS_DESCRIPTOR;
impl<T: prost::Name> ProtoToMcapSchema for T {
    fn translate_schema() -> mcap::Schema<'static> {
        mcap::Schema {
            id: 1,
            name: format!("{}.{}", T::PACKAGE, T::NAME),
            encoding: "protobuf".to_string(),
            data: ALL_PROTOS_DESCRIPTOR.to_vec().into(),
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
            encoding: "json".to_string(),
            data: Cow::Owned(schema_bytes),
        }
    }
}
