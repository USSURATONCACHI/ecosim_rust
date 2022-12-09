#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod util;
mod world;

use std::sync::Arc;
use egui_backend::sdl2::video::GLProfile;
use egui_backend::{egui, sdl2};
use egui_backend::{sdl2::event::Event, DpiScaling, ShaderVersion};
use std::time::{Instant};
// Alias the backend to something less mouthful
use egui_sdl2_gl as egui_backend;
use egui_sdl2_gl::egui::Rect;
use egui_sdl2_gl::EguiStateHandler;
use egui_sdl2_gl::painter::Painter;
use glow::{Context, HasContext};
use sdl2::{EventPump, Sdl, VideoSubsystem};
use sdl2::video::{GLContext, SwapInterval, Window};

use crate::app::App;
use crate::util::RateManager;
use crate::world::{PaintData, World};

pub struct WindowData {
	pub sdl_context: Sdl,
	pub video_subsystem: VideoSubsystem,
	pub window: Window,
	pub ctx: GLContext,
	pub gl: Arc<Context>,
	pub event_pump: EventPump,
}

pub struct TediousDataBundle {
	pub sdl_context: Sdl,					// used once
	pub video_subsystem: VideoSubsystem,	// used once
	pub ctx: GLContext,						// used once
	pub window: Window,						// used in loop
	pub gl: Arc<Context>,					// used in loop
	pub event_pump: EventPump,				// used in loop

	pub painter: Painter,					// used in loop
	pub egui_state: EguiStateHandler,		// used in loop
	pub egui_ctx: egui::Context,			// used in loop
	pub start_time: Instant,				// used in loop
}

fn main() {
	let win_data = set_up_window("Ecosim | Temporary game of life", 800, 600);

	let (painter, egui_state) =
		egui_backend::with_sdl2(
			&win_data.window,
			ShaderVersion::Default,
			DpiScaling::Default
		);
	let egui_ctx = egui::Context::default();
	let start_time = Instant::now();

	let world = World::new(win_data.gl.clone(), (768, 786));
	let app = App::new(&egui_ctx, (world.size().0 as f32 / 2.0, world.size().1 as f32 / 2.0));

	let data = TediousDataBundle {
		sdl_context: 		win_data.sdl_context,
		video_subsystem: 	win_data.video_subsystem,
		ctx: 				win_data.ctx,
		window: 			win_data.window,
		gl: 				win_data.gl,
		event_pump: 		win_data.event_pump,
		painter,
		egui_state,
		egui_ctx,
		start_time
	};

	run_loop(data, world, app);
}

pub fn set_up_window(title: &str, width: u32, height: u32) -> WindowData {
	let sdl_context = sdl2::init().unwrap();
	let video_subsystem = sdl_context.video().unwrap();
	let gl_attr = video_subsystem.gl_attr();
	gl_attr.set_context_profile(GLProfile::Core);

	gl_attr.set_double_buffer(true);
	gl_attr.set_multisample_samples(1);

	let window = video_subsystem
		.window(title, width, height)
		.opengl()
		.resizable()
		.build()
		.unwrap();

	// Create a window context
	let ctx = window.gl_create_context().unwrap();
	let glow_gl = unsafe { Context::from_loader_function(|name| video_subsystem.gl_get_proc_address(name) as *const _) };
	let glow_gl = Arc::new(glow_gl);
	let event_pump = sdl_context.event_pump().unwrap();

	window
		.subsystem()
		.gl_set_swap_interval(SwapInterval::Immediate)
		.unwrap();

	WindowData {
		sdl_context,
		video_subsystem,
		window,
		ctx,
		gl: glow_gl,
		event_pump
	}
}

impl TediousDataBundle {
	pub fn render_egui<F>(&mut self, run_ui: F)
		where F: FnMut(&egui::Context)
	{
		self.egui_state.input.time = Some(self.start_time.elapsed().as_secs_f64());
		let inputs = self.egui_state.input.take();

		// Render egui
		let outputs = self.egui_ctx.run(inputs, run_ui);
		self.egui_state.process_output(&self.window, &outputs.platform_output);

		let paint_jobs = self.egui_ctx.tessellate(outputs.shapes);
		self.painter.paint(None, paint_jobs, &outputs.textures_delta);
	}

	pub fn render_all(&mut self, app: &mut App, world: &World) {
		let mut rect: Option<Rect> = None;
		self.render_egui(
			|ctx| app.update(ctx, world, &mut rect)
		);

		let rect = rect.unwrap();
		set_viewport_rect(&self.gl, rect);

		let vp_size = rect.max - rect.min;
		let paint_data = PaintData {
			screen_size: (vp_size.x, vp_size.y),
			camera_pos: app.camera.pos(),
			zoom: app.camera.zoom(),
			antialiasing: app.antialiasing,
		};
		world.render(paint_data);

		unsafe {
			self.gl.viewport(0, 0, self.window.size().0 as i32, self.window.size().1 as i32);
		}
		self.window.gl_swap_window();
	}

	pub fn process_input(&mut self, event: Event) {
		self.egui_state.process_input(&self.window, event, &mut self.painter);
	}
}

fn run_loop(mut data: TediousDataBundle, mut world: World, mut app: App) {
	let mut ups_manager = RateManager::new(5, 2);
	let mut fps_manager = RateManager::new(60, 60);
	let mut prev_ups_limit = 0;

	// Contains max theoretical performance
	let mut assumed_ups = 100.0;	// updates per second

	'running: loop {
		let now = Instant::now();
		let next_render_time = fps_manager.next_tick_time();

		if prev_ups_limit != app.ups_limit {
			prev_ups_limit = app.ups_limit;
			ups_manager = RateManager::new(app.ups_limit.min(256),  app.ups_limit);
		}
		if fps_manager.tick_rate() != app.target_fps as u32 {
			fps_manager.set_tick_rate(app.target_fps as u32);
		}

		let time_left = next_render_time - now;
		let max_ticks_to_do = ((time_left.as_secs_f64() * assumed_ups) as u32).max(1);

		let simulation_running = app.run_until > world.cur_tick() || app.run_simulation;
		let ticks_to_do;
		if app.run_until > world.cur_tick() {
			ticks_to_do	= (app.run_until - world.cur_tick()).min(max_ticks_to_do as u64);
		} else if !app.run_simulation {
			ticks_to_do = 0;
		} else  {
			ticks_to_do = max_ticks_to_do as u64;
		}

		let ticks_to_do = if app.is_ups_limited {
			let target_ticks_to_do = ups_manager.ticks_to_do_by_time(next_render_time) as u64;
			target_ticks_to_do.min(ticks_to_do)
		} else {
			ticks_to_do
		};

		// println!("Ticks to do: {} / {}", ticks_to_do, max_ticks_to_do);
		if ticks_to_do > 0 {
			// UPDATE
			let update_start = Instant::now();
			world.use_program();
			for _ in 0..ticks_to_do {
				world.update();
				ups_manager.register_tick();
			}
			unsafe {
				data.gl.finish();
			}
			let current_ups = (ticks_to_do as f64) / update_start.elapsed().as_secs_f64();
			assumed_ups = (assumed_ups * 3.0 + current_ups) / 4.0;
		}

		let now = Instant::now();
		if (app.is_ups_limited || !simulation_running) && now < next_render_time {
			std::thread::sleep(next_render_time - now);
		}

		if now >= next_render_time {
			// RENDER
			world.no_tick(); // Update TPS counter
			data.render_all(&mut app, &world);
			fps_manager.register_tick();
		}

		for event in data.event_pump.poll_iter() {
			match event {
				Event::Quit { .. } => break 'running,
				_ => data.egui_state.process_input(&data.window, event, &mut data.painter),
			}
		}
	}
}

pub fn set_viewport_rect(gl: &Context, rect: Rect) {
	unsafe {
		gl.viewport(rect.min.x as i32, rect.min.y as i32,
					(rect.max.x - rect.min.x) as i32, (rect.max.y - rect.min.y) as i32);
	}
}