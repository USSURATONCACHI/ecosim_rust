#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use std::collections::HashMap;
use eframe::egui;

use std::sync::{Arc};
use std::sync::mpsc::Sender;
use std::time::Duration;
use eframe::egui::ComboBox;
use crate::egui::panel::Side;
use crate::egui::{Align, ColorImage, DragValue, Grid, ImageButton, Layout, ScrollArea, TextureHandle, Ui, Vec2};
use crate::update_thread::{Message, UpdThread};
use crate::world::World;
use crate::world_renderer::{AntiAliasing, PaintData, SendPtr, WorldRenderer};

mod util;
mod world;
mod update_thread;
mod world_renderer;

const ICON_PLAY: &[u8] = include_bytes!("../assets/img/play.png");
const ICON_PAUSE: &[u8] = include_bytes!("../assets/img/pause.png");

fn main() {
	let options = eframe::NativeOptions {
		initial_window_size: Some(egui::vec2(800.0, 600.0)),
		multisampling: 8,
		renderer: eframe::Renderer::Glow,
		..Default::default()
	};

	let world = Box::new(World::new((100, 75)));
	let world_ptr = world.as_ref() as *const World;

	let (gui_tx, upd_rx) = std::sync::mpsc::channel();

	let upd_thread = UpdThread::new(upd_rx, world).run();

	eframe::run_native(
		"Ecosim | Temporary game of life",
		options,
		Box::new(move |cc| Box::new(App::new(gui_tx, world_ptr, cc))),
	);
	upd_thread.join().unwrap();
}



#[derive(Debug, Clone, Copy, PartialEq)]
enum MenuTab {
	View,
	Params,
	Entity,
	Stats,
	ProgramSettings,
}
impl MenuTab {
	pub fn all() -> [MenuTab; 5] {
		[
			MenuTab::View,
			MenuTab::Params,
			MenuTab::Entity,
			MenuTab::Stats,
			MenuTab::ProgramSettings,
		]
	}
}
impl ToString for MenuTab {
	fn to_string(&self) -> String {
		match self {
			MenuTab::View => "View",
			MenuTab::Params => "Params",
			MenuTab::Entity => "Entity Info",
			MenuTab::Stats => "Statistics",
			MenuTab::ProgramSettings => "Program Settings",
		}.to_string()
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RenderMode {
	Food,
	Energy,
	Health,
	Alive,
	Dead,
}

struct App {
	run_simulation: bool,
	selected_tab: MenuTab,
	render_mode: RenderMode,

	// This is intentional unsafe part.
	world: *const World,
	tx_to_world: Sender<Message>,

	is_ups_limited: bool,
	ups_limit: u32,

	// Behind an `Arc<Mutex<…>>` so we can pass it to [`egui::PaintCallback`] and paint later.
	world_renderer: Arc<egui::mutex::Mutex<WorldRenderer>>,
	camera_pos: (f32, f32),
	camera_zoom: f32,
	antialiasing: AntiAliasing,

	images: HashMap<String, (ColorImage, Option<TextureHandle>)>,
}

impl App {
	fn new(tx_to_world: Sender<Message>, world: *const World, cc: &eframe::CreationContext<'_>) -> Self {
		let gl = cc
			.gl
			.as_ref()
			.expect("You need to run eframe with the glow backend");

		let images = [
			("play", ICON_PLAY),
			("pause", ICON_PAUSE),
		];

		let world_size = unsafe { world.as_ref().unwrap().size() };

		let images: HashMap<String, (ColorImage, Option<TextureHandle>)> = images.into_iter()
			.map(|(name, bytes)| (name.to_string(), (load_image_from_bytes(bytes).unwrap(), None)))
			.collect();

		Self {
			run_simulation: false,
			selected_tab: MenuTab::View,
			camera_pos: ((world_size.0 as f32) / 2.0, (world_size.1 as f32) / 2.0),
			camera_zoom: 0.0,
			render_mode: RenderMode::Food,
			world,
			tx_to_world,
			is_ups_limited: false,
			ups_limit: 1000,
			world_renderer: Arc::new(egui::mutex::Mutex::new(WorldRenderer::new(gl))),
			images,
			antialiasing: AntiAliasing::SSAAx16
		}
	}

	fn texture_handle(&mut self, name: impl Into<String>, ui: &mut Ui) -> &TextureHandle {
		let name_owned = name.into();
		let name = &name_owned;
		match self.images.get_mut(name) {
			None => panic!("Image '{}' was not added :(", name),
			Some((image, handle)) => {
				handle.get_or_insert_with(|| {
					ui.ctx().load_texture(
						name,
						image.clone(),
						Default::default()
					)
				})
			}
		}
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		ctx.request_repaint_after(Duration::from_nanos(1_000_000_000 / 60));
		let (tps, tick, (size_x, size_y)) = {
			let world = unsafe {
				self.world.as_ref().unwrap()
			};
			(world.tps().tps_corrected(), world.cur_tick(), world.size())
		};

		egui::SidePanel::new(Side::Left, "control_panel")
			.show(ctx, |ui| {
			ScrollArea::vertical()
				.show(ui, |ui| {
				ui.add_space(5.0);
				ui.horizontal(|ui| {
					let btn_size = ui.spacing().icon_width;

					if self.run_simulation {
						let pause = self.texture_handle("pause", ui);
						if ui.add(ImageButton::new(pause, (btn_size, btn_size))).clicked() {
							self.run_simulation = false;
							self.tx_to_world.send(Message::RunSimulation(false)).unwrap();
						}
					} else {
						let play = self.texture_handle("play", ui);
						if ui.add(ImageButton::new(play, (btn_size, btn_size))).clicked() {
							self.run_simulation = true;
							self.tx_to_world.send(Message::RunSimulation(true)).unwrap();
						}
					}

					ui.label(format!("Simulation time: {} ticks", tick));
				});
				ui.label(format!("World size: {}×{}", size_x, size_y));
				ui.label(format!("TPS: {:.02}", tps));
				ui.label(format!("Total entities: -"));

				ui.separator();
				ui.horizontal_wrapped(|ui| {
					for tab in MenuTab::all() {
						ui.selectable_value(&mut self.selected_tab, tab, tab.to_string());
					}
				});
				ui.separator();


				match self.selected_tab {
					MenuTab::View => {
						Grid::new("tab_grid")
							.num_columns(2)
							.spacing((40.0, 4.0))
							//.striped(true)
							.show(ui, |ui| {
								ui.label("Camera X");
								ui.add(DragValue::new(&mut self.camera_pos.0));
								ui.end_row();

								ui.label("Camera Y");
								ui.add(DragValue::new(&mut self.camera_pos.1));
								ui.end_row();

								ui.label("Zoom (exp)");
								ui.add(DragValue::new(&mut self.camera_zoom));
								ui.end_row();

								ui.label("Anti-Aliasing");
								ComboBox::new("antialiasing", "")
									.selected_text(format!("{:?}", self.antialiasing))
									.show_ui(ui, |ui| {
										for aa_type in AntiAliasing::all_values() {
											ui.selectable_value(&mut self.antialiasing, *aa_type, format!("{:?}", aa_type));
										}
									});
								ui.end_row();

								let ups_limit_changed = ui.checkbox(&mut self.is_ups_limited, "UPS limit").changed();
								let ups_limit_changed = ups_limit_changed ||
									ui.add_enabled(self.is_ups_limited, DragValue::new(&mut self.ups_limit)).changed();
								self.ups_limit = self.ups_limit.max(1);

								if ups_limit_changed {
									let limit = if self.is_ups_limited {
										Some(self.ups_limit)
									} else {
										None
									};
									self.tx_to_world.send(Message::LimitUPS(limit)).unwrap();
								}

								ui.end_row();
							});

						ui.heading("View mode:");
						ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
							ui.selectable_value(&mut self.render_mode, RenderMode::Food, "Food type");
							ui.selectable_value(&mut self.render_mode, RenderMode::Energy, "Energy");
							ui.selectable_value(&mut self.render_mode, RenderMode::Health, "Health");
							ui.selectable_value(&mut self.render_mode, RenderMode::Alive, "Alive");
							ui.selectable_value(&mut self.render_mode, RenderMode::Dead, "Dead");
						});
					}
					MenuTab::Params => {

					}
					MenuTab::Entity => {}
					MenuTab::Stats => {}
					MenuTab::ProgramSettings => {}
				}
			});
		});

		egui::CentralPanel::default().show(ctx, |ui| {
			egui::Frame::canvas(ui.style()).show(ui, |ui| {
				self.custom_painting(ctx, ui);
			});
		});
	}

	fn on_exit(&mut self, gl: Option<&glow::Context>) {
		self.tx_to_world.send(Message::Stop).unwrap();
		if let Some(gl) = gl {
			self.world_renderer.lock().destroy(gl);
		}
	}
}

impl App {
	fn custom_painting(&mut self, ctx: &egui::Context, ui: &mut Ui) {
		let rect = ui.available_rect_before_wrap();
		let (rect, response) = ui.allocate_exact_size(Vec2::new(rect.width(), rect.height()), egui::Sense::click_and_drag());

		let zoom_coef = 2.0_f32.powf(self.camera_zoom);
		self.camera_pos.0 -= response.drag_delta().x / zoom_coef;
		self.camera_pos.1 += response.drag_delta().y / zoom_coef;

		if response.hovered() {
			self.camera_zoom += ctx.input().scroll_delta.y * 0.01;
			if ctx.input().modifiers.ctrl {
				// zoom without camera shift
			} else if ctx.input().modifiers.alt {
				// only camera shift
			} else {
				// zoom normally
			}
		}

		// Clone locals so we can move them into the paint callback:
		let paint_data = PaintData {
			world: SendPtr(self.world),
			screen_size: (rect.width(), rect.height()),
			camera_pos: self.camera_pos.clone(),
			zoom: self.camera_zoom.clone(),
			antialiasing: self.antialiasing,
		};
		let world_renderer = self.world_renderer.clone();

		let callback = egui::PaintCallback {
			rect,
			callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
				world_renderer.lock().paint(painter.gl(), paint_data.clone());
			})),
		};
		ui.painter().add(callback);
	}
}

fn load_image_from_path(path: &std::path::Path) -> Result<ColorImage, image::ImageError> {
	let image = image::io::Reader::open(path)?.decode()?;
	let size = [image.width() as _, image.height() as _];
	let image_buffer = image.to_rgba8();
	let pixels = image_buffer.as_flat_samples();
	Ok(ColorImage::from_rgba_unmultiplied(
		size,
		pixels.as_slice(),
	))
}

fn load_image_from_bytes(bytes: &[u8]) -> Result<ColorImage, image::ImageError> {
	let image = image::load_from_memory(bytes).unwrap();
	let size = [image.width() as _, image.height() as _];
	let image_buffer = image.to_rgba8();
	let pixels = image_buffer.as_flat_samples();
	Ok(ColorImage::from_rgba_unmultiplied(
		size,
		pixels.as_slice(),
	))
}

