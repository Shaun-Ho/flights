use crate::airspace::AirspaceViewer;
use eframe::egui;
use walkers;

pub struct RadarApp {
    #[allow(dead_code)]
    airspace_viewer: AirspaceViewer,
    tiles: walkers::HttpTiles,
    map_memory: walkers::MapMemory,
}

impl RadarApp {
    #[must_use]
    pub fn new(egui_ctx: egui::Context, airspace_viewer: AirspaceViewer) -> Self {
        Self {
            tiles: walkers::HttpTiles::new(walkers::sources::OpenStreetMap, egui_ctx),
            map_memory: walkers::MapMemory::default(),

            airspace_viewer,
        }
    }
}

impl eframe::App for RadarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let my_position = walkers::Position::new(0.0, 0.0);

                let mut map =
                    walkers::Map::new(Some(&mut self.tiles), &mut self.map_memory, my_position);

                map = map.zoom_with_ctrl(false).drag_pan_buttons(
                    egui::DragPanButtons::PRIMARY | egui::DragPanButtons::SECONDARY,
                );

                map.show(ui, |_ui, _response, _projector, _map_memory| {})
            });
    }
}
