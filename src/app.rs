use std::collections::HashMap;
use egui_sdl2_gl::egui;
use egui_sdl2_gl::egui::{Align, ColorImage, ComboBox, DragValue, Grid, ImageButton, Layout, Rect, ScrollArea, Slider, TextureHandle, Ui, Vec2};
use egui_sdl2_gl::egui::panel::Side;
use crate::util::Camera;
use crate::world::World;

const ICON_PLAY: &[u8] = include_bytes!("../assets/img/play.png");
const ICON_PAUSE: &[u8] = include_bytes!("../assets/img/pause.png");
const ICON_PLAY_STOP: &[u8] = include_bytes!("../assets/img/play_and_stop.png");

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


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AntiAliasing {
	MSAAx1 = 1,
	MSAAx2 = 2,
	MSAAx3 = 3,
	MSAAx4 = 4,
	MSAAx5 = 5,
	SSAAx9 = 9,
	SSAAx16 = 16,
}
impl AntiAliasing {
	pub fn all_values() -> &'static [AntiAliasing] {
		&[
			AntiAliasing::MSAAx1,
			AntiAliasing::MSAAx2,
			AntiAliasing::MSAAx3,
			AntiAliasing::MSAAx4,
			AntiAliasing::MSAAx5,
			AntiAliasing::SSAAx9,
			AntiAliasing::SSAAx16,
		]
	}
}

pub struct App {
	pub run_simulation: bool,
	run_exactly: u64,
	pub run_until: u64,
	pub target_fps: u64,

	selected_tab: MenuTab,
	render_mode: RenderMode,

	pub camera: Camera,
	cam_vel_sensitivity: f32,
	cam_zoom_sensitivity: f32,

	pub is_ups_limited: bool,
	pub ups_limit: u32,
	pub antialiasing: AntiAliasing,

	images: HashMap<String, TextureHandle>,
}

impl App {
	pub fn new(ctx: &egui::Context, camera_pos: (f32, f32)) -> Self {
		let images: [(&str, &[u8]); 3] = [
			("play", ICON_PLAY),
			("pause", ICON_PAUSE),
			("play_stop", ICON_PLAY_STOP),
		];

		let images: HashMap<String, TextureHandle> = images
			.into_iter()
			.map(
				|(name, bytes)|
					(
						name.to_string(),
						ctx.load_texture(name, load_image_from_bytes(bytes).unwrap())
					)
			).collect();

		Self {
			run_simulation: false,
			run_exactly: 1,
			run_until: 0,
			target_fps: 60,
			selected_tab: MenuTab::View,

			camera: Camera::new(camera_pos.0, camera_pos.1),
			cam_vel_sensitivity: 1.0,
			cam_zoom_sensitivity: 4.0,

			render_mode: RenderMode::Food,
			is_ups_limited: false,
			ups_limit: 1000,
			images,
			antialiasing: AntiAliasing::SSAAx16,
		}
	}

	fn texture_handle(&mut self, name: &str) -> &TextureHandle {
		self.images.get(name).unwrap()
	}
}

impl App {
	pub fn update(&mut self, ctx: &egui::Context, world: &World, return_rect: &mut Option<Rect>) {
		let (tps, tick, (size_x, size_y)) = {
			(world.tps().tps_corrected(), world.cur_tick(), world.size())
		};

		egui::SidePanel::new(Side::Left, "control_panel")
			.show(ctx, |ui| {
				ScrollArea::vertical()
					.show(ui, |ui| {
						ui.add_space(5.0);
						let btn_size = ui.spacing().icon_width;
						let run_simulation = self.run_simulation;
						ui.horizontal(|ui| {
							if self.run_simulation {
								let pause = self.texture_handle("pause");
								if ui.add(ImageButton::new(pause, (btn_size, btn_size))).clicked() {
									self.run_simulation = false;
									// self.tx_to_world.send(Message::RunSimulation(false)).unwrap();
								}
							} else {
								let play = self.texture_handle("play");
								if ui.add(ImageButton::new(play, (btn_size, btn_size))).clicked() {
									self.run_simulation = true;
									// self.tx_to_world.send(Message::RunSimulation(true)).unwrap();
								}
							}

							ui.label(format!("Simulation time: {} ticks", tick));
						});

						ui.horizontal(|ui| {
							let play_stop = self.texture_handle("play_stop");
							let response = ui.add_enabled(!run_simulation, ImageButton::new(play_stop, (btn_size, btn_size)));

							ui.label("Run exactly");
							ui.add(DragValue::new(&mut self.run_exactly));
							ui.label("ticks");

							if response.clicked() {
								self.run_until = self.run_exactly + tick;
								// self.tx_to_world.send(Message::RunUntil(tick + self.run_exactly as u64)).unwrap();
							}
						});

						ui.label(format!("World size: {}×{}", size_x, size_y));
						ui.label(format!("UPS: {:.02}", tps));
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
										let (mut tmp_cam_x, mut tmp_cam_y) = self.camera.pos();
										ui.label("Camera X");
										let cam_x_changed = ui.add(DragValue::new(&mut tmp_cam_x)).changed();
										ui.end_row();

										ui.label("Camera Y");
										let cam_y_changed = ui.add(DragValue::new(&mut tmp_cam_y)).changed();
										ui.end_row();

										if cam_x_changed || cam_y_changed {
											self.camera.set_pos((tmp_cam_x, tmp_cam_y));
										}

										ui.label("Zoom (exp)");
										let mut tmp_zoom = self.camera.zoom();
										if ui.add(DragValue::new(&mut tmp_zoom).speed(0.01)).changed() {
											self.camera.set_zoom(tmp_zoom);
										}
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
											/*let limit = if self.is_ups_limited {
												Some(self.ups_limit)
											} else {
												None
											};
											self.tx_to_world.send(Message::LimitUPS(limit)).unwrap();*/
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

								ui.heading("Sensitivity");
								Grid::new("sensitivity")
									.num_columns(2)
									.spacing((40.0, 4.0))
									//.striped(true)
									.show(ui, |ui| {
										ui.label("Drag sensitivity");
										ui.add(DragValue::new(&mut self.cam_vel_sensitivity).speed(0.01));
										ui.end_row();

										ui.label("Zoom sensitivity");
										ui.add(DragValue::new(&mut self.cam_zoom_sensitivity).speed(0.01));
										ui.end_row();

										ui.label("Drag anim. exp.");
										ui.horizontal(|ui| {
											ui.label("1.0/");
											ui.add(DragValue::new(&mut self.camera.vel_exp).speed(0.1).clamp_range(2.0..=2.0e20));
										});
										ui.end_row();

										ui.label("Zoom anim. exp.");
										ui.horizontal(|ui| {
											ui.label("1.0/");
											ui.add(DragValue::new(&mut self.camera.zoom_exp).speed(0.1).clamp_range(2.0..=2.0e20));

										});
										ui.end_row();

										ui.label("Drag inertia");
										ui.add(Slider::new(&mut self.camera.vel_inertia, 0.0..=1.0));
										ui.end_row();

										ui.label("FPS");
										ui.add(DragValue::new(&mut self.target_fps).clamp_range(10..=1000));
										ui.end_row();
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
				self.custom_painting(ctx, ui, return_rect);
			});
		});
	}
}

impl App {
	fn custom_painting(&mut self, ctx: &egui::Context, ui: &mut Ui, return_rect: &mut Option<Rect>) {
		let rect = ui.available_rect_before_wrap();
		let (rect, response) = ui.allocate_exact_size(Vec2::new(rect.width(), rect.height()), egui::Sense::click_and_drag());

		let zoom_coef = 2.0_f32.powf(self.camera.zoom());

		if response.drag_started() {
			self.camera.on_drag_start();
		}
		if response.dragged() {
			let drag = response.drag_delta();
			self.camera.on_drag((-drag.x / zoom_coef * self.cam_vel_sensitivity, drag.y / zoom_coef * self.cam_vel_sensitivity));
		}
		if response.drag_released() {
			self.camera.on_drag_end();
		}


		if response.hovered() {
			let zoom_delta = ctx.input().scroll_delta.y * 0.01 * self.cam_zoom_sensitivity;
			if zoom_delta.abs() >= 0.001 {
				self.camera.on_zoom(zoom_delta);
			}
			//println!("Zoom: {}", ctx.input().scroll_delta.y);
			if ctx.input().modifiers.ctrl {
				// zoom without camera shift
			} else if ctx.input().modifiers.alt {
				// only camera shift
			} else {
				// zoom normally
			}
		}

		*return_rect = Some(rect);
	}
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