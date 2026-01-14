use crate::airspace::AirspaceViewer;
use crate::thread_manager::SteppableTask;

pub struct TerminalRenderer {
    pub viewer: AirspaceViewer,
}

impl TerminalRenderer {
    #[must_use]
    pub fn new(viewer: AirspaceViewer) -> Self {
        Self { viewer }
    }
}

impl SteppableTask for TerminalRenderer {
    fn step(&mut self) -> bool {
        true
    }
}
