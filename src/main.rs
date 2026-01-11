use clap::Parser;
use flights::airspace::AirspaceStore;
use flights::cli::Cli;
use flights::config::ApplicationConfig;
use flights::ingestor::Ingestor;
use flights::logging::setup_logging;
use flights::parser::AircraftParser;
use flights::thread_manager::ThreadManager;
use flights::types::Aircraft;
use log::info;

fn main() {
    let cli = Cli::parse();
    let application_config = ApplicationConfig::construct_from_path(&cli.config_file)
        .unwrap_or_else(|e| {
            log::error!("{e}");
            panic!("Config error. Exiting.")
        });

    setup_logging(cli.logging_level);
    info!("Main: Application started.");

    let (messages_sender, messages_receiver): (
        crossbeam_channel::Sender<String>,
        crossbeam_channel::Receiver<String>,
    ) = crossbeam_channel::unbounded();

    let (aircraft_data_sender, aircraft_data_receiver): (
        crossbeam_channel::Sender<Aircraft>,
        crossbeam_channel::Receiver<Aircraft>,
    ) = crossbeam_channel::unbounded();

    let ingestor = Ingestor::new(&application_config.glidernet, messages_sender)
        .map_err(|e| log::error!("Error constructing ingestor: {e}"))
        .unwrap();

    let parser = AircraftParser::new(messages_receiver, aircraft_data_sender);

    let airspace_store = AirspaceStore::new(
        aircraft_data_receiver,
        chrono::TimeDelta::seconds(application_config.airspace.time_buffer_seconds.into()),
    );

    let mut thread_manager = ThreadManager::new();
    thread_manager.add_task(ingestor, std::time::Duration::from_micros(50));
    thread_manager.add_task(parser, std::time::Duration::from_micros(50));
    let airspace_task_id =
        thread_manager.add_task(airspace_store, std::time::Duration::from_millis(500));

    if let Some(duration) = cli.duration {
        std::thread::sleep(std::time::Duration::from_secs(duration));
        thread_manager.stop_all_tasks();
    }

    thread_manager.wait_on_task_finish(airspace_task_id);

    info!("Main: Program finished.");
}
