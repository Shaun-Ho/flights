#[derive(Debug, thiserror::Error)]
pub enum IngestorConfigError {
    #[error("Failed to read config file '{0}': {1}", path.display(), source)]
    Parse {
        source: toml::de::Error,
        path: std::path::PathBuf,
    },
    #[error("Failed to read config file '{0}': {1}", path.display(), source)]
    Io {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum PacketConversionError {
    #[error("Missing timestamp field")]
    MissingTimestamp,
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(#[from] prost_types::TimestampError),
}

#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("Packet Conversion Error: {0}")]
    Conversion(PacketConversionError),
    #[error("Failed to read line from source: {0}")]
    StreamReadError(#[from] std::io::Error),
    #[error("Failed to decode protobuf message from file: {0}")]
    DecodeReadError(#[from] prost::DecodeError),
}
