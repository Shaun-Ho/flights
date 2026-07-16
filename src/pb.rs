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
