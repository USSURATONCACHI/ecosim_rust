#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use std::collections::HashMap;
use eframe::{egui, UserEvent};

use std::sync::{Arc};
use std::sync::mpsc::Sender;
use std::time::Duration;
use eframe::egui::{ComboBox, Slider};
use glow::Context;
use glutin::{PossiblyCurrent, WindowedContext};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopWindowTarget;
use winit::platform::windows::WindowBuilderExtWindows;
use winit::window::WindowBuilder;
use crate::egui::panel::Side;
use crate::egui::{Align, ColorImage, DragValue, Grid, ImageButton, Layout, ScrollArea, TextureHandle, Ui, Vec2};
use crate::update_thread::{Message, UpdThread};
use crate::util::Camera;
use crate::world::World;
use crate::world_renderer::{AntiAliasing, PaintData, SendPtr, WorldRenderer};

mod util;
mod world;
mod update_thread;
mod world_renderer;

const ICON_PLAY: &[u8] = include_bytes!("../assets/img/play.png");
const ICON_PAUSE: &[u8] = include_bytes!("../assets/img/pause.png");
const ICON_PLAY_STOP: &[u8] = include_bytes!("../assets/img/play_and_stop.png");

fn main() {
	let mut options = eframe::NativeOptions {
		initial_window_size: Some(egui::vec2(800.0, 600.0)),
		multisampling: 8,
		renderer: eframe::Renderer::Glow,
		..Default::default()
	};

	let event_loop = eframe::run::create_event_loop_builder(&mut options).build();
	let (gl_window, gl) = create_glutin_windowed_context(&event_loop, &"Ecosim | Temporary game of life".to_string());
	let gl = Arc::new(gl);


	// DO THINGS >>>>>>
	println!("Gl: {:?}", gl);

	let world = Box::new(World::new(gl.clone(), (500, 375)));
	let world_ptr = world.as_ref() as *const World;

	let (gui_tx, upd_rx) = std::sync::mpsc::channel();

	let _upd_thread = UpdThread::new(upd_rx, world).run();
	// DO THINGS <<<<<<

	// I modified a bit source code to divide window&context creation from
	// start of event loop. That allows to extract Arc<glow::Context>.
	let mut winit_app = eframe::run::glow_integration::GlowWinitApp::custom(
		&event_loop,
		"Ecosim | Temporary game of life",
		options,
		Box::new(move |cc| Box::new(App::new(gui_tx, world_ptr, cc))),

		gl_window,
		gl.clone()
	);

	eframe::run::run_and_exit(event_loop, winit_app);
}

pub fn create_glutin_windowed_context(event_loop: &EventLoopWindowTarget<UserEvent>, title: &String) -> (WindowedContext<PossiblyCurrent>, Context) {
	let window_builder = window_builder((800.0, 600.0))
		.with_title(title)
		.with_visible(false); // Keep hidden until we've painted something. See https://github.com/emilk/egui/pull/2279

	let gl_window = unsafe {
		glutin::ContextBuilder::new()
			.with_depth_buffer(0)
			.with_multisampling(8)
			.with_vsync(true)
			.build_windowed(window_builder, event_loop)
			.unwrap()
			.make_current()
			.unwrap()
	};

	let gl = unsafe { Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

	(gl_window, gl)
}

pub fn window_builder(size: (f32, f32)) -> WindowBuilder {
	let mut window_builder = WindowBuilder::new()
		.with_resizable(true)
		.with_drag_and_drop(true)
		.with_inner_size(PhysicalSize::new(size.0, size.1));

	#[cfg(target_os = "macos")]
	if *fullsize_content {
		window_builder = window_builder
			.with_title_hidden(true)
			.with_titlebar_transparent(true)
			.with_fullsize_content_view(true);
	}

	window_builder
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
	run_exactly: u32,

	selected_tab: MenuTab,
	render_mode: RenderMode,

	camera: Camera,
	cam_vel_sensitivity: f32,
	cam_zoom_sensitivity: f32,

	// This is intentional unsafe part.
	world: *const World,
	tx_to_world: Sender<Message>,

	is_ups_limited: bool,
	ups_limit: u32,

	// Behind an `Arc<Mutex<…>>` so we can pass it to [`egui::PaintCallback`] and paint later.
	world_renderer: Arc<egui::mutex::Mutex<WorldRenderer>>,
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
			("play_stop", ICON_PLAY_STOP),
		];

		let world_size = unsafe { world.as_ref().unwrap().size() };

		let images: HashMap<String, (ColorImage, Option<TextureHandle>)> = images.into_iter()
			.map(|(name, bytes)| (name.to_string(), (load_image_from_bytes(bytes).unwrap(), None)))
			.collect();

		Self {
			run_simulation: false,
			selected_tab: MenuTab::View,

			camera: Camera::new((world_size.0 as f32) / 2.0, (world_size.1 as f32) / 2.0),
			cam_vel_sensitivity: 1.0,
			cam_zoom_sensitivity: 1.0,

			render_mode: RenderMode::Food,
			world,
			tx_to_world,
			is_ups_limited: false,
			ups_limit: 1000,
			run_exactly: 1,
			world_renderer: Arc::new(egui::mutex::Mutex::new(WorldRenderer::new(gl))),
			images,
			antialiasing: AntiAliasing::SSAAx16,
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
	fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
		ctx.request_repaint_after(Duration::from_nanos(1_000_000_000 / 60));

		let world_size = unsafe { self.world.as_ref().unwrap().size() };
		//self.camera.wrap_x(world_size.0 as f32);
		//self.camera.wrap_y(world_size.1 as f32);

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
				let btn_size = ui.spacing().icon_width;
				let run_simulation = self.run_simulation;
				ui.horizontal(|ui| {
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

				ui.horizontal(|ui| {
					let play_stop = self.texture_handle("play_stop", ui);
					let response = ui.add_enabled(!run_simulation, ImageButton::new(play_stop, (btn_size, btn_size)));

					ui.label("Run exactly");
					ui.add(DragValue::new(&mut self.run_exactly));
					ui.label("ticks");

					if response.clicked() {
						self.tx_to_world.send(Message::RunUntil(tick + self.run_exactly as u64)).unwrap();
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

		let zoom_coef = 2.0_f32.powf(self.camera.zoom());

		if response.drag_started() {
			self.camera.on_drag_start();
		}
		if response.dragged() {
			let drag = response.drag_delta();
			self.camera.on_drag((-drag.x / zoom_coef / self.cam_vel_sensitivity, drag.y / zoom_coef / self.cam_vel_sensitivity));
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

		// Clone locals so we can move them into the paint callback:
		let paint_data = PaintData {
			world: SendPtr(self.world),
			screen_size: (rect.width(), rect.height()),
			camera_pos: self.camera.pos(),
			zoom: self.camera.zoom(),
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

