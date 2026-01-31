mod constants;

use crate::airspace::AirspaceViewer;
use crate::types::Aircraft;
use constants::AIRCRAFT_REFERENCE_SHAPE;
use eframe::{egui, epaint};
use walkers;

pub struct RadarApp {
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

                let airspace_plugin = AirspacePlugin::new(self.airspace_viewer.clone());

                map = map
                    .zoom_with_ctrl(false)
                    .drag_pan_buttons(
                        egui::DragPanButtons::PRIMARY | egui::DragPanButtons::SECONDARY,
                    )
                    .with_plugin(airspace_plugin);

                map.show(ui, |_ui, _response, _projector, _map_memory| {})
            });
    }
}

pub struct AirspacePlugin {
    viewer: AirspaceViewer,
}
impl AirspacePlugin {
    #[must_use]
    pub fn new(viewer: AirspaceViewer) -> Self {
        AirspacePlugin { viewer }
    }
}

impl walkers::Plugin for AirspacePlugin {
    fn run(
        self: Box<Self>,
        ui: &mut egui::Ui,
        _response: &egui::Response,
        projector: &walkers::Projector,
        map_memory: &walkers::MapMemory,
    ) {
        let zoom = map_memory.zoom();

        #[allow(clippy::cast_possible_truncation)]
        let scale_factor = (zoom * 0.1).clamp(0.1, 1.0) as f32;

        // read from airspace and render information on screen.
        let airspace = self.viewer.read();

        for aircraft_queue in airspace.icao_to_aircraft_mapping().values() {
            if aircraft_queue.is_empty() {
                continue;
            }

            // convert every position in the history to a screen x,y
            let aircraft_and_points: Vec<(&Aircraft, egui::Pos2)> = aircraft_queue
                .iter()
                .map(|aircraft| {
                    (
                        aircraft,
                        projector
                            .project(walkers::lat_lon(aircraft.latitude, aircraft.longitude))
                            .to_pos2(),
                    )
                })
                .collect();

            // draw most recent position
            if let Some((aircraft, current_position)) = aircraft_and_points.last() {
                // don't draw if the dot is off-screen
                if ui.max_rect().contains(*current_position) {
                    // calculate shape of aircraft drawn on screen based on the actual point
                    #[allow(clippy::cast_possible_truncation)]
                    let aircraft_bearing = aircraft.ground_track.to_radians() as f32;
                    let shape = build_aircraft_path_shape(
                        *current_position,
                        scale_factor,
                        aircraft_bearing,
                        epaint::Color32::RED,
                    );

                    ui.painter().add(shape);
                }
            }
            // draw trails
            ui.painter().line(
                aircraft_and_points
                    .iter()
                    .map(|(_aircraft, point)| *point)
                    .collect::<Vec<egui::Pos2>>(),
                egui::epaint::Stroke::new(1.0, epaint::Color32::BLUE),
            );
        }
    }
}

fn apply_shape_on_point(
    center_point: egui::Pos2,
    raw_shape: &[egui::Pos2],
    rotation: egui::emath::Rot2,
    scale: f32,
) -> Vec<egui::Pos2> {
    raw_shape
        .iter()
        .map(|&shape_point| center_point + rotation * (shape_point.to_vec2() * scale))
        .collect::<Vec<egui::Pos2>>()
}

fn build_aircraft_path_shape(
    current_position: egui::Pos2,
    scale_factor: f32,
    rotation_radians: f32,
    fill_color: epaint::Color32,
) -> epaint::PathShape {
    let aircraft_shape_points = apply_shape_on_point(
        current_position,
        &AIRCRAFT_REFERENCE_SHAPE,
        egui::emath::Rot2::from_angle(rotation_radians),
        scale_factor,
    );
    epaint::PathShape {
        points: aircraft_shape_points,
        closed: true,
        fill: fill_color,
        stroke: egui::epaint::PathStroke::new(0.3, epaint::Color32::BLACK),
    }
}
