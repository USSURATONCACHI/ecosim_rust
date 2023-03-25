use egui_sdl2_gl::egui::{Ui, Grid, DragValue};

use crate::map;

use super::{EditMap, MapType};

#[derive(Debug, Clone)]
pub struct LandscapeEditor {
    create_size: (u32, u32),
    create_height: f64,
}

impl LandscapeEditor {
    pub fn new() -> Self {
        LandscapeEditor {
            create_size: (256, 256),
            create_height: 1.0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, map: &mut Option<EditMap>, page: &mut MapType) {
        const SPACE: f32 = 15.0;
        ui.heading("Load or create");
        
        if ui.button("Load from image").clicked() {};

        ui.add_space(SPACE);
        Grid::new("tab_grid")
            .num_columns(2)
            .spacing((40.0, 4.0))
            .show(ui, |ui| {
                ui.label("Size X");
                ui.add(DragValue::new(&mut self.create_size.0).clamp_range(0..=map::MAX_MAP_SIZE.0));
                ui.end_row();
                
                ui.label("Size Y");
                ui.add(DragValue::new(&mut self.create_size.0).clamp_range(0..=map::MAX_MAP_SIZE.0));
                ui.end_row();

                ui.label("Landscape height");
                ui.add(DragValue::new(&mut self.create_height).clamp_range(0.0..=1.0));
                ui.end_row();
            });

        if ui.button("Create new map").clicked() {};
    }
}