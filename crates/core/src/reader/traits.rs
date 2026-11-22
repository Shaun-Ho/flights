use crate::common::{JsonFormat, LogFormat, ProtoFormat};
use crate::logging::EncodableMessage;

pub trait DecodeMessage<Message>: LogFormat {
    type Error: std::error::Error + Send + Sync + 'static;
    fn decode(bytes: &[u8]) -> Result<Message, Self::Error>;
}

impl<M: prost::Message + Default> DecodeMessage<M> for ProtoFormat {
    type Error = prost::DecodeError;
    fn decode(bytes: &[u8]) -> Result<M, Self::Error> {
        M::decode(bytes)
    }
}

impl<M: serde::de::DeserializeOwned> DecodeMessage<M> for JsonFormat {
    type Error = serde_json::Error;
    fn decode(bytes: &[u8]) -> Result<M, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

pub trait ReplayableMessage<F: DecodeMessage<Self::Message>>: Sized + Send + 'static {
    type Message: EncodableMessage<F>;
    type Error: std::error::Error + Send + Sync + 'static;

    fn from_stored(stored: Self::Message) -> Result<Self, Self::Error>;
}
