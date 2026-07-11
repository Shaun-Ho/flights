const ALL_PROTOS_DESCRIPTOR: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

pub mod ingestor {
    include!(concat!(env!("OUT_DIR"), "/ingestor.rs"));

    use super::ALL_PROTOS_DESCRIPTOR;
    use crate::core::central_disk_logger::traits::McapSchemaDescriptor;

    impl McapSchemaDescriptor for PbAprsPacket {
        fn schema_name() -> String {
            "ingestor.PbAprsPacket".to_string() // Your proto package + message name
        }
        fn encoding() -> &'static str {
            "protobuf"
        }
        fn schema_bytes() -> Vec<u8> {
            ALL_PROTOS_DESCRIPTOR.to_vec()
        }
        fn topic() -> String {
            "/ingestor/aprs".to_string()
        }
    }
}
pub mod parser {
    include!(concat!(env!("OUT_DIR"), "/parser.rs"));

    use super::ALL_PROTOS_DESCRIPTOR;
    use crate::core::central_disk_logger::traits::McapSchemaDescriptor;

    impl McapSchemaDescriptor for PbAircraft {
        fn schema_name() -> String {
            "parser.PbAircraft".to_string()
        }
        fn encoding() -> &'static str {
            "protobuf"
        }
        fn schema_bytes() -> Vec<u8> {
            ALL_PROTOS_DESCRIPTOR.to_vec()
        }
        fn topic() -> String {
            "/parser/aircraft".to_string()
        }
    }
}
pub mod airspace {
    include!(concat!(env!("OUT_DIR"), "/airspace.rs"));

    use super::ALL_PROTOS_DESCRIPTOR;
    use crate::core::central_disk_logger::traits::McapSchemaDescriptor;

    impl McapSchemaDescriptor for PbAirspaceUpdate {
        fn schema_name() -> String {
            "airspace.PbAirspaceUpdate".to_string()
        }
        fn encoding() -> &'static str {
            "protobuf"
        }
        fn schema_bytes() -> Vec<u8> {
            ALL_PROTOS_DESCRIPTOR.to_vec()
        }
        fn topic() -> String {
            "/airspace/airspace_update".to_string()
        }
    }
}
