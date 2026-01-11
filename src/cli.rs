use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub duration: Option<u64>,

    #[arg(short, long, default_value_t = log::LevelFilter::Info)]
    pub logging_level: log::LevelFilter,

    #[arg(long)]
    pub config_file: std::path::PathBuf,
}
