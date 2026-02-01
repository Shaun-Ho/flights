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
