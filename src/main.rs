use flights::ingestor::Ingestor;
use flights::logging::setup_logging;
use log::{error, info, warn};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration; // Ensure chrono is in Cargo.toml

fn main() {
    setup_logging();
    info!("Main: Application started.");

    let ingestor = Ingestor::new(String::from("aprs.glidernet.org"), 14580);

    let (tx_data, rx_data): (
        crossbeam_channel::Sender<String>,
        crossbeam_channel::Receiver<String>,
    ) = crossbeam_channel::unbounded();

    let running = Arc::new(AtomicBool::new(true));

    let listen_handle = {
        let ingestor_clone = ingestor;
        let tx_data_clone = tx_data.clone();
        let running_clone = running.clone();
        std::thread::spawn(move || {
            loop {
                ingestor_clone.listen(tx_data_clone.clone(), "r/0/0/25000");

                if running_clone.load(Ordering::SeqCst) {
                    warn!("Ingestor: Listen cycle ended, reconnecting in 5s...");
                    std::thread::sleep(Duration::from_secs(5));
                } else {
                    info!("Ingestor: Shutting down gracefully.");
                    break;
                }
            }
        })
    };

    let process_handle = {
        let running_clone = running.clone();
        std::thread::spawn(move || {
            loop {
                match rx_data.recv_timeout(Duration::from_millis(100)) {
                    Ok(data) => {
                        info!("Received APRS Data: {data}");
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        if !running_clone.load(Ordering::SeqCst) {
                            info!("Parser: No more data, shutting down.");
                            break;
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        error!("Parser: Ingestor channel disconnected.");
                        break;
                    }
                }
            }
            info!("Parser: Shutting down gracefully.");
        })
    };

    // --- Main thread waits for a period before signalling shutdown ---
    info!("Main: Running for 15 seconds before initiating shutdown...");
    std::thread::sleep(Duration::from_secs(15)); // Wait for 15 seconds

    // Signal all spawned threads to stop
    info!("Main: Signaling threads to shut down.");
    running.store(false, Ordering::SeqCst);

    // Wait for the spawned threads to finish
    listen_handle.join().expect("Ingestor listener panicked");
    process_handle.join().expect("Data processor panicked");

    info!("Main: Program finished.");
}
