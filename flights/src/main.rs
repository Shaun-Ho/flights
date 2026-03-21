use clap::Parser;

use flights::cli::Cli;
use flights::core::ingestor::config::IngestorConfig;
use flights::gui::RadarApp;
use flights::logging::setup_logging;
use flights::setup_pipeline;

fn main() {
    let cli = Cli::parse();
    let ingestor_config =
        IngestorConfig::construct_from_path(&cli.config_file).unwrap_or_else(|e| {
            log::error!("{e}");
            panic!("Config error. Exiting.")
        });

    setup_logging(cli.logging_level);
    log::info!("Main: Application started.");

    let pipeline = setup_pipeline(ingestor_config);

    let run_duration = cli.duration.map(std::time::Duration::from_secs);

    if cli.gui {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "Airspace Radar",
            options,
            Box::new(|cc| {
                if let Some(duration) = run_duration {
                    let ctx = cc.egui_ctx.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(duration);
                        log::info!("Duration reached. Requesting GUI close.");
                        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
                    });
                }
                Ok(Box::new(RadarApp::new(
                    cc.egui_ctx.clone(),
                    pipeline.get_airspace_viewer(),
                )))
            }),
        )
        .unwrap();
    } else if let Some(duration) = run_duration {
        std::thread::sleep(duration);
    }
    log::info!("Shutting down application.");
}
