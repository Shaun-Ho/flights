use clap::Parser;
use flights::cli::Cli;
use flights::config::ApplicationConfig;
use flights::ingestor::Ingestor;
use flights::logging::setup_logging;
use flights::thread_manager::ThreadManager;
use log::info;

fn main() {
    setup_logging(log::LevelFilter::Debug);
    let cli = Cli::parse();
    let application_config = ApplicationConfig::construct_from_path(&cli.config_file)
        .unwrap_or_else(|e| {
            log::error!("{e}");
            panic!("Config error. Exiting.")
        });
    info!("Main: Application started.");

    let (messages_sender, _messages_receiver): (
        crossbeam_channel::Sender<String>,
        crossbeam_channel::Receiver<String>,
    ) = crossbeam_channel::unbounded();

    let ingestor = Ingestor::new(&application_config.glidernet, messages_sender)
        .map_err(|e| log::error!("Error constructing ingestor: {e}"))
        .unwrap();

    let mut thread_manager = ThreadManager::new();
    let ingestor_task_id = thread_manager.add_task(ingestor, std::time::Duration::from_micros(50));

    std::thread::sleep(std::time::Duration::from_secs(5));
    thread_manager.stop_all_tasks();
    thread_manager.wait_on_task_finish(ingestor_task_id);
    info!("Main: Program finished.");
}
