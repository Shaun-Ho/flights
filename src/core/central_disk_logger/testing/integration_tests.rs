use std::fs;

use prost::Message;

use super::test_helpers::*;
use crate::core::central_disk_logger::interface::LogSender;
use crate::core::central_disk_logger::*;
use crate::core::thread_manager::*;

#[test]
fn given_complete_system_when_message_sent_and_stepped_then_correct_bytes_on_disk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("system_log.pb");

    let mut registry = DiskLoggerRegistry::new();
    let handle = registry
        .register_proto::<MockTaskProto>(file_path.clone())
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
    let expected_bytes = expected_proto.encode_length_delimited_to_vec();

    let disk_contents = fs::read(&file_path).unwrap();
    assert_eq!(disk_contents, expected_bytes);
}

#[test]
fn given_multiple_handles_when_messages_sent_concurrently_then_system_routes_correctly() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path_1 = temp_dir.path().join("flight_data.pb");
    let file_path_2 = temp_dir.path().join("engine_data.pb");

    let mut registry = DiskLoggerRegistry::new();

    let log_handle_1 = registry
        .register_proto::<MockTaskProto>(file_path_1.clone())
        .unwrap();
    let log_handle_2 = registry
        .register_proto::<MockTaskProto>(file_path_2.clone())
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
    }
    .encode_length_delimited_to_vec();
    let expected_2 = MockTaskProto {
        larger_than_zero: 222,
    }
    .encode_length_delimited_to_vec();

    assert_eq!(fs::read(&file_path_1).unwrap(), expected_1);
    assert_eq!(fs::read(&file_path_2).unwrap(), expected_2);
}
