use egui_sdl2_gl::egui::{Ui, Grid, DragValue, self};

use crate::map;

use super::{EditMap, MapType};


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Height = 0,
    HeightColors,
    BasicBiomes,
}
impl View {
    pub fn all() -> &'static [View] {
        &[View::Height, View::HeightColors, View::BasicBiomes]
    }

    pub fn localized_name(&self) -> &'static str {
        match self {
            View::Height => "Height",
            View::HeightColors => "Height (colors)",
            View::BasicBiomes => "Basic biomes",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LandscapeEditor {
    create_size: (u32, u32),
    create_height: f64,

    view: View,
}

impl LandscapeEditor {
    pub fn new() -> Self {
        LandscapeEditor {
            create_size: (256, 256),
            create_height: 1.0,

            view: View::BasicBiomes,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, map: &mut Option<EditMap>, page: &mut MapType) {
        const SPACE: f32 = 15.0;
        ui.heading("Load or create");
        
        if ui.button("Load from image").clicked() {};

        let enabled = match map {
            Some(EditMap::Biomes(_)) => true,
            _ => false,
        };
        if ui.add_enabled(enabled, egui::Button::new("Convert biomes map (roughly)")).clicked() {};
        if ui.add_enabled(false, egui::Button::new("Convert simulation map (roughly)")).clicked() {};

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
        let enabled = match map {
            Some(EditMap::Landscape(_)) => true,
            _ => false,
        };
        if ui.add_enabled(enabled, egui::Button::new("Resize this map")).clicked() {}
        
        ui.add_space(SPACE);
        ui.heading("View mode");
        ui.horizontal_wrapped(|ui| {
            for view in View::all() {
                ui.selectable_value(&mut self.view, *view, view.localized_name());
            }
        });
    }
}