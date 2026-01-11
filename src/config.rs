use crate::ingestor::GliderNetConfig;
use serde;
use toml;

#[derive(serde::Deserialize)]
pub struct ApplicationConfig {
    pub glidernet: GliderNetConfig,
    pub airspace: AirspaceConfig,
}

impl ApplicationConfig {
    pub fn construct_from_path(
        path: &std::path::PathBuf,
    ) -> Result<ApplicationConfig, errors::ApplicationConfigError> {
        let string =
            std::fs::read_to_string(path).map_err(|error| errors::ApplicationConfigError::Io {
                source: error,
                path: path.clone(),
            })?;

        let config: Result<ApplicationConfig, errors::ApplicationConfigError> =
            toml::from_str(&string).map_err(|error| errors::ApplicationConfigError::Parse {
                source: error,
                path: path.clone(),
            });
        config
    }
}

#[derive(serde::Deserialize)]
pub struct AirspaceConfig {
    pub time_buffer_seconds: u8,
}

pub mod errors {

    #[derive(Debug)]
    pub enum ApplicationConfigError {
        Parse {
            source: toml::de::Error,
            path: std::path::PathBuf,
        },
        Io {
            source: std::io::Error,
            path: std::path::PathBuf,
        },
    }
    impl std::fmt::Display for ApplicationConfigError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ApplicationConfigError::Io {
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
                ApplicationConfigError::Parse {
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
    impl std::error::Error for ApplicationConfigError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ApplicationConfigError::Io { source: error, .. } => Some(error),
                ApplicationConfigError::Parse { source: error, .. } => Some(error),
            }
        }
    }
}
