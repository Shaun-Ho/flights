use env_logger;
use std::io::Write;

pub fn setup_logging() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{0} {1} {2}] {3}",
                record.level(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.module_path().unwrap_or(""),
                record.args()
            )
        })
        .target(env_logger::Target::Stdout)
        .init();
}
