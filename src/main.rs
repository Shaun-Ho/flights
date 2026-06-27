use clap::Parser;

use flights::AirspaceDataPipeline;
use flights::Cli;
use flights::RadarApp;
use flights::logging::setup_logging;
use flights::pipeline::config::PipelineConfig;

fn main() {
    let cli = Cli::parse();
    let pipeline_config =
        PipelineConfig::construct_from_path(&cli.config_file).unwrap_or_else(|e| {
            log::error!("{e}");
            panic!("Config error. Exiting.")
        });

    setup_logging(cli.logging_level);
    log::info!("Main: Application started.");

    let mut data_pipeline = AirspaceDataPipeline::setup_pipeline(pipeline_config);

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
                    data_pipeline.get_airspace_viewer(),
                )))
            }),
        )
        .unwrap();
    } else if let Some(duration) = run_duration {
        std::thread::sleep(duration);
    } else {
        // run indefinitely until something breaks the chain
        std::thread::park();
    }
    data_pipeline.shutdown();
    log::info!("Shutting down application.");
}
