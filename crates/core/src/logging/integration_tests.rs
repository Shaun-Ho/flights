use prost::Message;

use crate::logging::*;
use crate::test_messages::*;
use crate::thread_manager::*;

#[test]
fn given_complete_system_when_message_sent_and_stepped_then_correct_bytes_on_disk() {
    let test_dir = testdir::testdir!();
    let file_path = test_dir.join("system_log.mcap");

    let mut registry = LoggerRegistry::new();
    let handle = registry
        .register_proto::<MockTaskProto>(file_path.clone(), "test".to_string())
        .expect("Failed to register logger");
    let mut central_logger = registry.build();

    let domain_message = MockTaskStruct {
        larger_than_zero: 5,
    };
    handle
        .send(domain_message.clone())
        .expect("Failed to send message");

    let state = central_logger.step();
    assert!(matches!(state, TaskState::Running));

    drop(central_logger);

    let expected_proto = MockTaskProto {
        larger_than_zero: 5,
    };

    let contents = std::fs::read(&file_path).unwrap();
    let mcap_message = mcap::MessageStream::new(&contents)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap();

    let decoded_message = MockTaskProto::decode(&*mcap_message.data).unwrap();
    assert_eq!(decoded_message, expected_proto);
}

#[test]
fn given_multiple_handles_when_messages_sent_concurrently_then_system_routes_correctly() {
    let test_dir = testdir::testdir!();
    let file_path_1 = test_dir.join("flight_data.mcap");
    let file_path_2 = test_dir.join("engine_data.mcap");

    let mut registry = LoggerRegistry::new();

    let log_handle_1 = registry
        .register_proto::<MockTaskProto>(file_path_1.clone(), "test".to_string())
        .unwrap();
    let log_handle_2 = registry
        .register_proto::<MockTaskProto>(file_path_2.clone(), "test".to_string())
        .unwrap();

    let mut central_logger = registry.build();

    let handle_1 = std::thread::spawn(move || {
        log_handle_1
            .send(MockTaskStruct {
                larger_than_zero: 111,
            })
            .unwrap();
    });
    let handle_2 = std::thread::spawn(move || {
        log_handle_2
            .send(MockTaskStruct {
                larger_than_zero: 222,
            })
            .unwrap();
    });

    let _ = handle_1.join();
    let _ = handle_2.join();

    central_logger.step();
    central_logger.step();

    drop(central_logger);

    let expected_1 = MockTaskProto {
        larger_than_zero: 111,
    };

    let contents_1 = std::fs::read(&file_path_1).unwrap();
    let mcap_message_1 = mcap::MessageStream::new(&contents_1)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap();
    let decoded_message_1 = MockTaskProto::decode(&*mcap_message_1.data).unwrap();
    assert_eq!(decoded_message_1, expected_1);

    let contents_2 = std::fs::read(&file_path_2).unwrap();
    let mcap_message_2 = mcap::MessageStream::new(&contents_2)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap();

    let expected_2 = MockTaskProto {
        larger_than_zero: 222,
    };

    let decoded_message_2 = MockTaskProto::decode(&*mcap_message_2.data).unwrap();
    assert_eq!(decoded_message_2, expected_2);
}

#[test]
fn given_jsonl_logger_when_message_sent_and_stepped_then_correct_json_lines_on_disk() {
    let test_dir = testdir::testdir!();
    let file_path = test_dir.join("events_log.mcap");

    let mut registry = LoggerRegistry::new();
    let handle = registry
        .register_jsonl::<MockJsonMessage>(file_path.clone(), "test".to_string())
        .expect("Failed to register jsonl logger");
    let mut central_logger = registry.build();

    let domain_message = MockJsonMessage {
        contents: vec!["app_started".to_string(), "disk_ok".to_string()],
    };
    let expected_bytes = serde_json::to_vec(&domain_message).unwrap();

    handle
        .send(domain_message)
        .expect("Failed to send json message");

    let state = central_logger.step();
    assert!(matches!(state, TaskState::Running));

    drop(central_logger);

    let contents = std::fs::read(&file_path).unwrap();
    let mcap_message = mcap::MessageStream::new(&contents)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap();

    assert_eq!(mcap_message.data, expected_bytes);
}

#[test]
fn given_mixed_loggers_when_messages_sent_then_system_routes_both_formats_correctly() {
    let test_dir = testdir::testdir!();

    let proto_path = test_dir.join("flight_data.mcap");
    let jsonl_path = test_dir.join("system_events.mcap");

    let mut registry = LoggerRegistry::new();

    let proto_handle = registry
        .register_proto::<MockTaskProto>(proto_path.clone(), "test".to_string())
        .unwrap();
    let jsonl_handle = registry
        .register_jsonl::<MockJsonMessage>(jsonl_path.clone(), "test".to_string())
        .unwrap();

    let mut central_logger = registry.build();

    proto_handle
        .send(MockTaskStruct {
            larger_than_zero: 333,
        })
        .unwrap();

    let json_msg = MockJsonMessage {
        contents: vec!["concurrent_test".to_string()],
    };

    jsonl_handle.send(json_msg.clone()).unwrap();

    central_logger.step();
    central_logger.step();

    drop(central_logger);

    let expected_proto = MockTaskProto {
        larger_than_zero: 333,
    };

    let proto_contents = std::fs::read(&proto_path).unwrap();
    let proto_mcap_message = mcap::MessageStream::new(&proto_contents)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap();

    let decoded_proto = MockTaskProto::decode(&*proto_mcap_message.data).unwrap();
    assert_eq!(decoded_proto, expected_proto);

    let expected_jsonl = serde_json::to_vec(&json_msg).unwrap();
    let jsonl_contents = std::fs::read(&jsonl_path).unwrap();
    let jsonl_mcap_message = mcap::MessageStream::new(&jsonl_contents)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap();

    assert_eq!(jsonl_mcap_message.data, expected_jsonl);
}

/// Reads back the channel of the first message written to `path`.
fn first_channel(path: &std::path::Path) -> std::sync::Arc<mcap::Channel<'static>> {
    let contents = std::fs::read(path).unwrap();
    mcap::MessageStream::new(&contents)
        .unwrap()
        .next()
        .expect("Expected at least one MCAP message in the file")
        .unwrap()
        .channel
}

#[test]
fn given_proto_logger_when_message_written_then_channel_carries_protobuf_contract() {
    let test_dir = testdir::testdir!();
    let file_path = test_dir.join("proto_contract.mcap");

    let mut registry = LoggerRegistry::new();
    let handle = registry
        .register_proto::<MockTaskProto>(file_path.clone(), "test".to_string())
        .unwrap();
    let mut central_logger = registry.build();

    handle
        .send(MockTaskStruct {
            larger_than_zero: 1,
        })
        .unwrap();
    central_logger.step();
    drop(central_logger);

    let channel = first_channel(&file_path);
    let schema = channel
        .schema
        .as_ref()
        .expect("Expected a schema on the channel");
    assert_eq!(channel.message_encoding, "protobuf");
    assert_eq!(schema.name, "test.MockTaskProto");
    assert_eq!(schema.encoding, "protobuf");
}

#[test]
fn given_jsonl_logger_when_message_written_then_channel_carries_jsonschema_contract() {
    let test_dir = testdir::testdir!();
    let file_path = test_dir.join("json_contract.mcap");

    let mut registry = LoggerRegistry::new();
    let handle = registry
        .register_jsonl::<MockJsonMessage>(file_path.clone(), "test".to_string())
        .unwrap();
    let mut central_logger = registry.build();

    handle
        .send(MockJsonMessage {
            contents: vec!["contract".to_string()],
        })
        .unwrap();
    central_logger.step();
    drop(central_logger);

    let channel = first_channel(&file_path);
    let schema = channel
        .schema
        .as_ref()
        .expect("Expected a schema on the channel");
    assert_eq!(channel.message_encoding, "json");
    assert_eq!(schema.name, "MockJsonMessage");
    assert_eq!(schema.encoding, "jsonschema");
}
