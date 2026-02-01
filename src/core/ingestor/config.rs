use crate::core::ingestor::error;

use serde;
use toml;

#[derive(serde::Deserialize)]
pub struct IngestorConfig {
    pub glidernet: GliderNetConfig,
    pub airspace: AirspaceConfig,
}

impl IngestorConfig {
    pub fn construct_from_path(
        path: &std::path::PathBuf,
    ) -> Result<IngestorConfig, error::IngestorConfigError> {
        let string =
            std::fs::read_to_string(path).map_err(|error| error::IngestorConfigError::Io {
                source: error,
                path: path.clone(),
            })?;

        let config: Result<IngestorConfig, error::IngestorConfigError> = toml::from_str(&string)
            .map_err(|error| error::IngestorConfigError::Parse {
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

#[derive(serde::Deserialize)]
pub struct GliderNetConfig {
    pub host: String,
    pub port: u64,
    pub filter: String,
}
