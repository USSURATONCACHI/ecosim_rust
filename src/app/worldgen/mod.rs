use egui_sdl2_gl::egui::{Ui, Layout, Align, Grid, DragValue};

use crate::map;

use self::{landscape::LandscapeEditor, biomes::BiomesEditor};

mod biomes;
mod landscape;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeightBrush {
    AddHeight,
    RemoveHeight,
    SetHeight,
    Aline,
}
impl HeightBrush {
    pub fn all() -> &'static [HeightBrush] {
        &[HeightBrush::AddHeight, HeightBrush::RemoveHeight, HeightBrush::SetHeight, HeightBrush::Aline]
    }
}

#[derive(Debug, Clone)]
pub enum EditMap {
    Landscape(map::Landscape),
    Biomes(map::Map),
}
impl EditMap {
    pub fn get_type(&self) -> MapType {
        match self {
            Self::Landscape(_) => MapType::Landscape,
            Self::Biomes(_) => MapType::Biomes, 
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapType {
    Landscape,
    Biomes,
}

pub struct WorldgenMenu {
    map: Option<EditMap>,
    page: MapType,

    landscape: LandscapeEditor,
    biomes: BiomesEditor,
}

impl WorldgenMenu {
    pub fn new() -> Self {
        WorldgenMenu {
            map: None,
            page: MapType::Landscape,
            landscape: LandscapeEditor::new(),
            biomes: BiomesEditor::new(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading("Mode");
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.page, MapType::Landscape, "Landscape");
            ui.selectable_value(&mut self.page, MapType::Biomes, "Biomes/Resources");
        });
        ui.separator();

        match self.page.clone() {
            MapType::Landscape => self.landscape.show(ui, &mut self.map, &mut self.page),
            MapType::Biomes => self.biomes.show(ui, &mut self.map, &mut self.page),
        }
    }
}

/*const SPACING: f32 = 15.0;
        ui.heading("Load from");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Save file").clicked() {}
            if ui.button("Image").clicked() {}
            if ui.button("Paint.net image").clicked() {}
        });

        ui.add_space(SPACING);
        ui.heading("View");
        ui.horizontal_wrapped(|ui| {
            for view in View::all() {
                ui.selectable_value(&mut self.view, *view, format!("{:?}", view));
            }
        });

        ui.add_space(SPACING);
        ui.heading("Resize");
        Grid::new("Resize")
            .num_columns(2)
            .spacing((40.0, 4.0))
            .show(ui, |ui| {
                ui.label("Width (X)");
                ui.add(DragValue::new(&mut self.resize_size.0).clamp_range(1..=16384));
                ui.end_row();
                
                ui.label("Height (Y)");
                ui.add(DragValue::new(&mut self.resize_size.1).clamp_range(1..=16384));
                ui.end_row();

                if ui.button("Resize!").clicked() {}
                ui.end_row();
            });

        ui.add_space(SPACING);
        ui.heading("Reset");
        ui.horizontal_wrapped(|ui| {
            if ui.button("To flat sea").clicked() {}
            if ui.button("To flat land").clicked() {}
        });

        ui.add_space(SPACING);
        ui.heading("Landscape generation");
        if ui.button("Perlin noise").clicked() {}
        if ui.button("Brownian movement").clicked() {}
        
        ui.add_space(SPACING);
        ui.heading("Editing");
        if ui.button("Perlin noise").clicked() {}
        if ui.button("Brownian movement").clicked() {}*/

/*

Empty Map:
- Sea
- Land

From images
From paint.net image

View mode:
    - Height
    - Biomes
    - Resources

Generate landscape:
    - Brounian motion
        - Continents "size"
        - Continents count
        - Should fill
    - Perlin noise
        - Scale

Manual editing:
    - Height brush

== Convert to biomes map ==

Manual editing:
    - Biomes brush
    - Resources brush

Compound amount: 1000
    - Set to 800
    - Compound brush

Erode (ticks)

-- Export --
Use this map
As paint.net image
As png (information losses)
As save file

*/