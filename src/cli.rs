use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long, default_value_t = -1)]
    pub duration: i32,

    #[arg(short, long, default_value_t = false)]
    pub debug: bool,

    #[arg(long)]
    pub config_file: std::path::PathBuf,
}
