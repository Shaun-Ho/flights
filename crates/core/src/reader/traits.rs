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

#[cfg(test)]
mod tests {
    use crate::logging::{LogSender, LoggerRegistry};
    use crate::test_messages::{MockJsonMessage, MockTaskProto, MockTaskStruct};
    use crate::thread_manager::SteppableTask;

    use super::*;

    /// Logs `messages` through the real writer and returns the stored payloads.
    fn logged_payloads(
        register: impl FnOnce(&mut LoggerRegistry, std::path::PathBuf) -> Box<dyn Fn()>,
    ) -> Vec<Vec<u8>> {
        let path = testdir::testdir!().join("log.mcap");
        let mut registry = LoggerRegistry::new();
        let send = register(&mut registry, path.clone());
        let mut logger = registry.build();
        send();
        logger.step();
        drop(logger);
        let bytes = std::fs::read(&path).unwrap();
        mcap::MessageStream::new(&bytes)
            .unwrap()
            .map(|message| message.unwrap().data.into_owned())
            .collect()
    }

    #[test]
    fn given_mock_proto_logged_by_registry_when_decoded_then_equals_original() {
        let original = MockTaskStruct {
            larger_than_zero: 7,
        };
        let sent = original.clone();
        let payloads = logged_payloads(|registry, path| {
            let handle = registry
                .register_proto::<MockTaskProto>(path, "test".to_owned())
                .unwrap();
            Box::new(move || handle.send(sent.clone()).unwrap())
        });

        let stored: MockTaskProto =
            <ProtoFormat as DecodeMessage<_>>::decode(&payloads[0]).unwrap();
        assert_eq!(
            <MockTaskStruct as ReplayableMessage<ProtoFormat>>::from_stored(stored).unwrap(),
            original
        );
    }

    #[test]
    fn given_mock_json_logged_by_registry_when_decoded_then_equals_original() {
        let original = MockJsonMessage {
            contents: vec!["a".to_owned(), "b".to_owned()],
        };
        let sent = original.clone();
        let payloads = logged_payloads(|registry, path| {
            let handle = registry
                .register_jsonl::<MockJsonMessage>(path, "test".to_owned())
                .unwrap();
            Box::new(move || handle.send(sent.clone()).unwrap())
        });

        let stored: MockJsonMessage =
            <JsonFormat as DecodeMessage<_>>::decode(&payloads[0]).unwrap();
        assert_eq!(
            <MockJsonMessage as ReplayableMessage<JsonFormat>>::from_stored(stored).unwrap(),
            original
        );
    }

    #[test]
    fn given_garbage_payload_when_decoded_then_decode_error() {
        // An unterminated varint: not decodable as any protobuf message, and not JSON either.
        let garbage = [0xFF; 8];

        assert!(<ProtoFormat as DecodeMessage<MockTaskProto>>::decode(&garbage).is_err());
        assert!(<JsonFormat as DecodeMessage<MockJsonMessage>>::decode(&garbage).is_err());
    }

    #[test]
    fn given_mock_proto_with_zero_when_converted_then_conversion_error() {
        let stored = MockTaskProto {
            larger_than_zero: 0,
        };

        assert!(<MockTaskStruct as ReplayableMessage<ProtoFormat>>::from_stored(stored).is_err());
    }
}
