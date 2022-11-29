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
use egui_sdl2_gl::gl;
use glow::{HasContext, NativeFence};
use sdl2::video::{GLContext, SwapInterval};

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

	let mut world = World::new(glow_gl.clone(), (4096, 4096));

	let mut app = App::new(&egui_ctx, (world.size().0 as f32 / 2.0, world.size().1 as f32 / 2.0));
	let mut frame = 0_u64;

	let mut update_fence: Option<NativeFence> = None;
	let mut update_start = Instant::now();
	'running: loop {
		let frames_required = start_time.elapsed().as_secs_f64() * 60.0;
		let should_render = frames_required >= frame as f64;
		let fence_open = match &update_fence {
			None => true,
			Some(fence) => unsafe { glow_gl.get_sync_status(fence.clone()) == glow::SIGNALED },
		};

		if (app.run_simulation || app.run_until > world.cur_tick()) &&
			(fence_open)	// || (!should_render && frames_required.fract() <= 0.5)
		{
			if let Some(fence) = update_fence {
				println!("Tick {} - {} ms", world.cur_tick(), update_start.elapsed().as_secs_f64() * 1000.0);
				update_start = Instant::now();
				unsafe {
					glow_gl.delete_sync(fence);
				}
			}
			update_fence = Some(world.update());
			/*let upd_start = Instant::now();
			let upd_end = Instant::now();
			print!("Tick {} - {:.04} ms | \n", world.cur_tick(), (upd_end - upd_start).as_secs_f64() * 1000.0);*/
		}

		if !should_render {
			continue;
		}
		frame += 1;

		/* ---- */ let render_start = Instant::now();

		egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
		let inputs = egui_state.input.take();

		/* ---- */ let inp = Instant::now();

		let mut rect: Option<Rect> = None;
		let outputs = egui_ctx.run(inputs, |egui_ctx| app.update(egui_ctx, &world, &mut rect));
		egui_state.process_output(&window, &outputs.platform_output);
		let paint_jobs = egui_ctx.tessellate(outputs.shapes);
		painter.paint(None, paint_jobs, &outputs.textures_delta);

		/* ---- */ let egui_rend = Instant::now();

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

		/* ---- */ let world_rend = Instant::now();

		window.gl_swap_window();

		/* ---- */ let swap_time = Instant::now();

		let events_start = Instant::now();
		for event in event_pump.poll_iter() {
			match event {
				Event::Quit { .. } => break 'running,
				_ => {
					// Process input event
					egui_state.process_input(&window, event, &mut painter);
				}
			}
		}
		/* ---- */ let events = Instant::now();

		let mut prev_ins = render_start;

		let timings = [
			(inp, 			"Input"),
			(egui_rend, 	"Egui"),
			(world_rend, 	"World"),
			(swap_time, 	"Swap"),
			(events, 		"Events"),
		];

		let total_ms = (prev_ins - render_start).as_secs_f64() * 1000.0;

		if total_ms >= 50.0 {
			for (instant, name) in timings {
				print!("{} {:.04} ms | ", name, (instant - prev_ins).as_secs_f64() * 1000.0);
				prev_ins = instant;
			}
			print!("Total: {:.04} ms\n", total_ms);
		}


		// println!("Render time: {:.03} ms | Events: {:.03} ms", frame_start.elapsed().as_secs_f64() * 1000.0, events_start.elapsed().as_secs_f64() * 1000.0);
	}
}