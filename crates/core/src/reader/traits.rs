use crate::common::{JsonFormat, LogFormat, ProtoFormat};

pub trait DecodeMessage<M>: LogFormat {
    type Error;
    fn decode(bytes: &[u8]) -> Result<M, Self::Error>;
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
