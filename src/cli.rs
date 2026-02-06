use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub duration: Option<u64>,

    #[arg(long, default_value_t = false)]
    pub gui: bool,

    #[command(flatten)]
    pub ingestor: IngestorConfig,

    #[arg(short, long, default_value_t = log::LevelFilter::Info)]
    pub logging_level: log::LevelFilter,

    #[arg(long)]
    pub config_file: std::path::PathBuf,
}

#[derive(Parser, Debug, Clone)]
pub struct IngestorConfig {
    #[arg(long, default_value = None)]
    pub log_input_data_stream: Option<std::path::PathBuf>,

    #[arg(long, default_value = None)]
    pub read_input_data_stream: Option<std::path::PathBuf>,
}
