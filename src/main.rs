#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod util;
mod world;

use std::sync::Arc;
use egui_backend::sdl2::video::GLProfile;
use egui_backend::{egui, sdl2};
use egui_backend::{sdl2::event::Event, DpiScaling, ShaderVersion};
use std::time::Instant;
// Alias the backend to something less mouthful
use egui_sdl2_gl as egui_backend;
use egui_sdl2_gl::egui::Rect;
use glow::{HasContext};
use sdl2::video::{SwapInterval};

use crate::app::App;
use crate::world::{PaintData, World};

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

fn main() {
	let sdl_context = sdl2::init().unwrap();
	let video_subsystem = sdl_context.video().unwrap();
	let gl_attr = video_subsystem.gl_attr();
	gl_attr.set_context_profile(GLProfile::Core);

	gl_attr.set_double_buffer(true);
	gl_attr.set_multisample_samples(1);

	let window = video_subsystem
		.window(
			"Ecosim | Temporary game of life",
			SCREEN_WIDTH,
			SCREEN_HEIGHT,
		)
		.opengl()
		.resizable()
		.build()
		.unwrap();

	// Create a window context
	let _ctx = window.gl_create_context().unwrap();
	let glow_gl = unsafe { glow::Context::from_loader_function(|name| video_subsystem.gl_get_proc_address(name) as *const _) };
	let glow_gl = Arc::new(glow_gl);
	// Init egui stuff
	let (mut painter, mut egui_state) = egui_backend::with_sdl2(&window, ShaderVersion::Default, DpiScaling::Default);
	let mut event_pump = sdl_context.event_pump().unwrap();

	window
		.subsystem()
		.gl_set_swap_interval(SwapInterval::Immediate)
		.unwrap();

	let start_time = Instant::now();
	let egui_ctx = egui::Context::default();

	let mut world = World::new(glow_gl.clone(), (768, 786));

	let mut app = App::new(&egui_ctx, (world.size().0 as f32 / 2.0, world.size().1 as f32 / 2.0));

	// Each `PACK_SIZE` frames start time is being reset
	const PACK_SIZE: u64 = 120;
	let mut prev_fps = app.target_fps;
	let mut frames_pack_start = Instant::now();
	let mut cur_frame = 0_u64;

	let mut assumed_ups = 100.0;	// updates per second

	'running: loop {
		if prev_fps != app.target_fps {
			cur_frame = 0;
			frames_pack_start = Instant::now();
			prev_fps = app.target_fps;
		}

		let next_frame_start = ((cur_frame + 1) as f64) / (app.target_fps as f64);

		if app.run_simulation || app.run_until > world.cur_tick()
		{
			let time_left = next_frame_start - frames_pack_start.elapsed().as_secs_f64();
			let updates_to_do = (time_left * assumed_ups).max(1.0) as u64;
			let updates_to_do = if app.run_until > world.cur_tick() {
				updates_to_do.min(app.run_until - world.cur_tick())
			} else {
				updates_to_do
			};

			if updates_to_do > 0 {
				let update_start = Instant::now();
				for _ in 0..updates_to_do {
					world.update();
				}
				unsafe {
					glow_gl.finish();
				}
				let current_ups = (updates_to_do as f64) / update_start.elapsed().as_secs_f64();
				assumed_ups = (assumed_ups * 3.0 + current_ups) / 4.0;
			}
		}

		if frames_pack_start.elapsed().as_secs_f64() < next_frame_start {
			continue; // Waiting for frame
		}

		world.no_tick();
		egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
		let inputs = egui_state.input.take();

		// Render egui
		let mut rect: Option<Rect> = None;
		let outputs = egui_ctx.run(inputs, |egui_ctx| app.update(egui_ctx, &world, &mut rect));
		egui_state.process_output(&window, &outputs.platform_output);
		let paint_jobs = egui_ctx.tessellate(outputs.shapes);
		painter.paint(None, paint_jobs, &outputs.textures_delta);

		// Render world on top
		if let Some(rect) = rect {
			unsafe {
				glow_gl.viewport(rect.min.x as i32, rect.min.y as i32,
								 (rect.max.x - rect.min.x) as i32, (rect.max.y - rect.min.y) as i32);
				let vp_size = rect.max - rect.min;
				let paint_data = PaintData {
					screen_size: (vp_size.x, vp_size.y),
					camera_pos: app.camera.pos(),
					zoom: app.camera.zoom(),
					antialiasing: app.antialiasing,
				};
				world.render(paint_data);
				glow_gl.viewport(0, 0, window.size().0 as i32, window.size().1 as i32);
			}
		}

		window.gl_swap_window();

		for event in event_pump.poll_iter() {
			match event {
				Event::Quit { .. } => break 'running,
				_ => {
					// Process input event
					egui_state.process_input(&window, event, &mut painter);
				}
			}
		}

		cur_frame = (cur_frame + 1) % PACK_SIZE;
		if cur_frame == 0 {
			frames_pack_start = Instant::now();
		}
	}
}
