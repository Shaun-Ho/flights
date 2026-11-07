use std::borrow::Cow;

use chrono::{DateTime, Utc};

use crate::common::{JsonFormat, LogFormat, ProtoFormat};
use crate::logging::errors::EncodeError;

pub trait LogSender<M> {
    type Error;
    fn send(&self, message: M) -> Result<(), Self::Error>;
}

pub trait IntoLogMessage<M> {
    type Error;
    fn into_message(self) -> Result<M, Self::Error>;
    fn message_timestamp(&self) -> DateTime<Utc>;
}

pub trait EncodableMessage<F: LogFormat> {
    fn schema_name() -> String;
    fn schema_data() -> Cow<'static, [u8]>;
    fn encode_payload(&self) -> Result<Vec<u8>, EncodeError>;
}

pub(crate) fn mcap_schema<F: LogFormat, M: EncodableMessage<F>>() -> mcap::Schema<'static> {
    mcap::Schema {
        id: 1,
        name: M::schema_name(),
        encoding: F::SCHEMA_ENCODING.to_string(),
        data: M::schema_data(),
    }
}

pub trait ProtoFileDescriptor {
    const FILE_DESCRIPTOR_SET: &'static [u8];
}

impl<T: prost::Message + prost::Name + ProtoFileDescriptor> EncodableMessage<ProtoFormat> for T {
    fn schema_name() -> String {
        format!("{}.{}", T::PACKAGE, T::NAME)
    }
    fn schema_data() -> Cow<'static, [u8]> {
        T::FILE_DESCRIPTOR_SET.into()
    }
    fn encode_payload(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.encode_to_vec())
    }
}
impl<T: serde::Serialize + schemars::JsonSchema> EncodableMessage<JsonFormat> for T {
    fn schema_name() -> String {
        <T as schemars::JsonSchema>::schema_name().into_owned()
    }
    fn schema_data() -> Cow<'static, [u8]> {
        let schema = schemars::schema_for!(T);
        Cow::Owned(serde_json::to_vec(&schema).expect("Failed to serialize JsonSchema to bytes"))
    }
    fn encode_payload(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(serde_json::to_vec(self)?)
    }
}
