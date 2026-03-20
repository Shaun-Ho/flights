#[derive(Debug)]
pub enum IngestorConfigError {
    Parse {
        source: toml::de::Error,
        path: std::path::PathBuf,
    },
    Io {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
}

impl std::fmt::Display for IngestorConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestorConfigError::Io {
                source: error,
                path,
            } => {
                write!(
                    f,
                    "Failed to read config file '{}': {}",
                    path.display(),
                    error
                )
            }
            IngestorConfigError::Parse {
                source: error,
                path,
            } => {
                write!(
                    f,
                    "Failed to parse config file '{}': {}",
                    path.display(),
                    error
                )
            }
        }
    }
}

impl std::error::Error for IngestorConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IngestorConfigError::Io { source: error, .. } => Some(error),
            IngestorConfigError::Parse { source: error, .. } => Some(error),
        }
    }
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
