use egui_sdl2_gl::egui::Ui;

use super::{EditMap, MapType};

#[derive(Debug, Clone)]
pub struct BiomesEditor {
    
}

impl BiomesEditor {
    pub fn new() -> Self {
        BiomesEditor {}
    }

    pub fn show(&mut self, ui: &mut Ui, map: &mut Option<EditMap>, page: &mut MapType) {
        ui.heading("Biomes");
    }
}