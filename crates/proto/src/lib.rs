//! Protobuf types generated from the workspace-level `proto/` directory.

use flights_core::logging::ProtoFileDescriptor;

pub const ALL_PROTOS_DESCRIPTOR: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

pub mod ingestor {
    include!(concat!(env!("OUT_DIR"), "/ingestor.rs"));
}
pub mod parser {
    include!(concat!(env!("OUT_DIR"), "/parser.rs"));
}
pub mod airspace {
    include!(concat!(env!("OUT_DIR"), "/airspace.rs"));
}

macro_rules! impl_proto_file_descriptor {
    ($($message:ty),* $(,)?) => {
        $(
            impl ProtoFileDescriptor for $message {
                const FILE_DESCRIPTOR_SET: &'static [u8] = ALL_PROTOS_DESCRIPTOR;
            }
        )*
    };
}

impl_proto_file_descriptor!(
    ingestor::PbAprsPacket,
    parser::PbAircraft,
    airspace::PbAirspaceUpdate,
);
