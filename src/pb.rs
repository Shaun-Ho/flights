pub mod ingestor {
    include!(concat!(env!("OUT_DIR"), "/ingestor.rs"));
}
pub mod parser {
    include!(concat!(env!("OUT_DIR"), "/parser.rs"));
}
pub mod airspace {
    include!(concat!(env!("OUT_DIR"), "/airspace.rs"));
}
