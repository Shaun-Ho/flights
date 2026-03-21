use eframe::egui;

pub const AIRCRAFT_REFERENCE_SHAPE: [egui::Pos2; 4] = [
    egui::pos2(0.0, -10.0), // Nose
    egui::pos2(7.0, 8.0),   // Right Wing tip
    egui::pos2(0.0, 2.0),   // Tail center indentation
    egui::pos2(-7.0, 8.0),  // Left Wing tip
];
