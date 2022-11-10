#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use std::collections::HashMap;
use std::path::PathBuf;
use eframe::egui;

use std::sync::{Arc};
use std::sync::mpsc::Sender;
use std::time::Duration;
use crate::egui::panel::Side;
use crate::egui::{Align, ColorImage, DragValue, Grid, ImageButton, Layout, ScrollArea, TextureHandle, Ui};
use crate::update_thread::{Message, UpdThread};
use crate::world::World;

mod util;
mod world;
mod update_thread;

fn main() {
	let options = eframe::NativeOptions {
		initial_window_size: Some(egui::vec2(800.0, 600.0)),
		multisampling: 8,
		renderer: eframe::Renderer::Glow,
		..Default::default()
	};

	let world = Box::new(World::new((200, 130)));
	let world_ptr = world.as_ref() as *const World;

	let (gui_tx, upd_rx) = std::sync::mpsc::channel();

	let upd_thread = UpdThread::new(upd_rx, world).run();

	eframe::run_native(
		"Ecosim | GUI stage",
		options,
		Box::new(move |cc| Box::new(App::new(gui_tx, world_ptr, cc))),
	);
	upd_thread.join().unwrap();
/*
	let mut ctx = egui::Context::default();

	// Game loop:
	loop {
		let raw_input = egui::RawInput::default();
		let full_output = ctx.run(raw_input, |ctx| {
			egui::CentralPanel::default().show(&ctx, |ui| {
				ui.label("Hello world!");
				if ui.button("Click me").clicked() {
					// take some action here
				}
			});
		});
		handle_platform_output(full_output.platform_output);
		let clipped_primitives = ctx.tessellate(full_output.shapes); // create triangles to paint
		paint(full_output.textures_delta, clipped_primitives);
		std::thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
	}*/
}

struct App {
	run_simulation: bool,
	selected_tab: MenuTab,
	camera_pos: (f32, f32),
	camera_zoom: f32,
	render_mode: RenderMode,

	// This is intentional unsafe part.
	world: *const World,
	tx_to_world: Sender<Message>,

	is_ups_limited: bool,
	ups_limit: u32,

	// Example data, tmp
	/// Behind an `Arc<Mutex<…>>` so we can pass it to [`egui::PaintCallback`] and paint later.
	rotating_triangle: Arc<egui::mutex::Mutex<RotatingTriangle>>,
	angle: f32,

	images: HashMap<String, (ColorImage, Option<TextureHandle>)>,
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

impl App {
	fn new(tx_to_world: Sender<Message>, world: *const World, cc: &eframe::CreationContext<'_>) -> Self {
		let gl = cc
			.gl
			.as_ref()
			.expect("You need to run eframe with the glow backend");

		let images = [
			("play", "assets/img/play.png"),
			("pause", "assets/img/pause.png"),
		];

		let images: HashMap<String, (ColorImage, Option<TextureHandle>)> = images.into_iter()
			.map(|(name, path)| (name.to_string(), (load_image_from_path(&PathBuf::from(path)).unwrap(), None)))
			.collect();

		Self {
			run_simulation: false,
			selected_tab: MenuTab::View,
			camera_pos: (0.0, 0.0),
			camera_zoom: 0.0,
			render_mode: RenderMode::Food,
			world,
			tx_to_world,
			is_ups_limited: false,
			ups_limit: 1000,
			rotating_triangle: Arc::new(egui::mutex::Mutex::new(RotatingTriangle::new(gl))),
			angle: 0.0,
			images,
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

	fn total_entities(&self) -> u32 {
		12345
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
				ui.label(format!("Total entities: {}", self.total_entities()));

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

								let ups_limit_changed = ui.checkbox(&mut self.is_ups_limited, "UPS limit").changed();
								let ups_limit_changed = ups_limit_changed ||
									ui.add_enabled(self.is_ups_limited, DragValue::new(&mut self.ups_limit)).changed();

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
				self.custom_painting(ui);
			});
		});
	}

	fn on_exit(&mut self, gl: Option<&glow::Context>) {
		self.tx_to_world.send(Message::Stop).unwrap();
		if let Some(gl) = gl {
			self.rotating_triangle.lock().destroy(gl);
		}
	}
}

impl App {
	fn custom_painting(&mut self, ui: &mut Ui) {
		let rect = ui.available_rect_before_wrap();
		let (rect, response) =
			ui.allocate_exact_size(egui::Vec2::new(rect.width(), rect.height()), egui::Sense::drag());

		self.angle += response.drag_delta().x * 0.01;

		// Clone locals so we can move them into the paint callback:
		let angle = self.angle;
		let rotating_triangle = self.rotating_triangle.clone();

		let callback = egui::PaintCallback {
			rect,
			callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
				rotating_triangle.lock().paint(painter.gl(), angle);
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


struct RotatingTriangle {
	program: glow::Program,
	vertex_array: glow::VertexArray,
}

impl RotatingTriangle {
	fn new(gl: &glow::Context) -> Self {
		use glow::HasContext as _;

		let shader_version = if cfg!(target_arch = "wasm32") {
			"#version 300 es"
		} else {
			"#version 330"
		};

		unsafe {
			let program = gl.create_program().expect("Cannot create program");

			let (vertex_shader_source, fragment_shader_source) = (
				r#"
                    const vec2 verts[3] = vec2[3](
                        vec2(0.0, 1.0),
                        vec2(-1.0, -1.0),
                        vec2(1.0, -1.0)
                    );
                    const vec4 colors[3] = vec4[3](
                        vec4(1.0, 0.0, 0.0, 1.0),
                        vec4(0.0, 1.0, 0.0, 1.0),
                        vec4(0.0, 0.0, 1.0, 1.0)
                    );
                    out vec4 v_color;
                    uniform float u_angle;
                    void main() {
                        v_color = colors[gl_VertexID];
                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
                        gl_Position.x *= cos(u_angle);
                    }
                "#,
				r#"
                    precision mediump float;
                    in vec4 v_color;
                    out vec4 out_color;
                    void main() {
                        out_color = v_color;
                    }
                "#,
			);

			let shader_sources = [
				(glow::VERTEX_SHADER, vertex_shader_source),
				(glow::FRAGMENT_SHADER, fragment_shader_source),
			];

			let shaders: Vec<_> = shader_sources
				.iter()
				.map(|(shader_type, shader_source)| {
					let shader = gl
						.create_shader(*shader_type)
						.expect("Cannot create shader");
					gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
					gl.compile_shader(shader);
					if !gl.get_shader_compile_status(shader) {
						panic!("{}", gl.get_shader_info_log(shader));
					}
					gl.attach_shader(program, shader);
					shader
				})
				.collect();

			gl.link_program(program);
			if !gl.get_program_link_status(program) {
				panic!("{}", gl.get_program_info_log(program));
			}

			for shader in shaders {
				gl.detach_shader(program, shader);
				gl.delete_shader(shader);
			}

			let vertex_array = gl
				.create_vertex_array()
				.expect("Cannot create vertex array");

			Self {
				program,
				vertex_array,
			}
		}
	}

	fn destroy(&self, gl: &glow::Context) {
		use glow::HasContext as _;
		unsafe {
			gl.delete_program(self.program);
			gl.delete_vertex_array(self.vertex_array);
		}
	}

	fn paint(&self, gl: &glow::Context, angle: f32) {
		use glow::HasContext as _;
		unsafe {
			gl.use_program(Some(self.program));
			gl.uniform_1_f32(
				gl.get_uniform_location(self.program, "u_angle").as_ref(),
				angle,
			);
			gl.bind_vertex_array(Some(self.vertex_array));
			gl.draw_arrays(glow::TRIANGLES, 0, 3);
		}
	}
}

