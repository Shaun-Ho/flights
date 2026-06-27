use std::path::PathBuf;

use serde;
use toml;

use crate::core::ingestor::config::GliderNetConfig;

#[derive(serde::Deserialize)]
pub struct PipelineConfig {
    pub ingestor: IngestorConfig,
    pub airspace: AirspaceConfig,
}

impl PipelineConfig {
    pub fn construct_from_path(
        path: &PathBuf,
    ) -> Result<PipelineConfig, errors::PipelineConfigError> {
        let string =
            std::fs::read_to_string(path).map_err(|error| errors::PipelineConfigError::Io {
                source: error,
                path: path.to_path_buf(),
            })?;

        let config: Result<PipelineConfig, errors::PipelineConfigError> = toml::from_str(&string)
            .map_err(|error| errors::PipelineConfigError::Parse {
                source: error,
                path: path.to_path_buf(),
            });
        config
    }
}

#[derive(serde::Deserialize)]
pub struct IngestorConfig {
    pub source: IngestorSource,
    pub write_path: Option<PathBuf>,
}
#[derive(serde::Deserialize)]
pub struct AirspaceConfig {
    pub time_buffer_seconds: u8,
}

pub mod errors {
    #[derive(Debug, thiserror::Error)]
    pub enum PipelineConfigError {
        #[error("Failed to parse config file: {path}\n {source}")]
        Parse {
            #[source]
            source: toml::de::Error,
            path: std::path::PathBuf,
        },
        #[error("Failed to open config file: {path}\n {source}")]
        Io {
            #[source]
            source: std::io::Error,
            path: std::path::PathBuf,
        },
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum IngestorSource {
    GliderNet(GliderNetConfig),
    FilePath(FilePathConfig),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilePathConfig {
    pub read_path: PathBuf,
}
