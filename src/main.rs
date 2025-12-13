use flights::ingestor::{GliderNetConfig, Ingestor};
use flights::logging::setup_logging;
use flights::thread_manager::ThreadManager;
use log::info;

fn main() {
    setup_logging(log::LevelFilter::Debug);
    info!("Main: Application started.");

    let config = GliderNetConfig {
        host: "aprs.glidernet.org".to_owned(),
        port: 14580,
        filter: "r/0/0/25000".to_owned(),
    };

    let (messages_sender, _messages_receiver): (
        crossbeam_channel::Sender<String>,
        crossbeam_channel::Receiver<String>,
    ) = crossbeam_channel::unbounded();

    let ingestor = Ingestor::new(&config, messages_sender)
        .map_err(|e| log::error!("Error constructing ingestor: {e}"))
        .unwrap();

    let mut thread_manager = ThreadManager::new();
    let ingestor_task_id = thread_manager.add_task(ingestor, std::time::Duration::from_micros(50));

    std::thread::sleep(std::time::Duration::from_secs(5));
    thread_manager.stop_all_tasks();
    thread_manager.wait_on_task_finish(ingestor_task_id);
    info!("Main: Program finished.");
}
