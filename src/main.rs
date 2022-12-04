//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod util;
mod world;

use std::io::Write;
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

	let mut world = World::new(glow_gl.clone(), (768, 786));

	let mut app = App::new(&egui_ctx, (world.size().0 as f32 / 2.0, world.size().1 as f32 / 2.0));
	let mut frame = 0_u64;


	// Lite testing
	let linear_sizes = [768];
	let measure_ticks = [10, 100, 1000, 10000];
	let measurements_count = 1;

/*
	// Deep testing
	let linear_sizes = [100, 200, 300, 400, 500, 600, 700, 768, 800, 900, 1000, 1024, 2048];
	let measure_ticks = [10, 100, 1000, 5000, 7500, 8000, 8500, 9000, 9500, 10000];
	let measurements_count = 1;
*/

	// Benchmark for a lot of variants.
	let mut final_table = "World width; World height; Ticks per measurement; Upd/Sec; Upd * Cell / Sec;\n".to_string();

	println!("Starting benchmark...");
	for size_x in &linear_sizes {
		for size_y in &linear_sizes {
			for ticks in &measure_ticks {
				let (size_x, size_y, ticks) = (*size_x, *size_y, *ticks);
				print!("Testing {:?} for {} ticks x {} times...", (size_x, size_y), ticks, measurements_count);
				let mut world = World::new(glow_gl.clone(), (size_x as u64, size_y as u64));

				let mut total_ups = 0.0;
				let mut total_upd_cell_per_sec = 0.0;

				std::io::stdout().flush().unwrap();
				for _ in 0..measurements_count {
					unsafe { glow_gl.finish(); }
					let start = Instant::now();
					for _ in 0..ticks {
						world.update();
					}
					unsafe { glow_gl.finish(); }

					let time_secs = start.elapsed().as_secs_f64();
					let ups = (ticks as f64) / time_secs;
					let upd_cell_per_sec = ups * (world.size().0 as f64) * (world.size().1 as f64);

					total_ups += ups;
					total_upd_cell_per_sec += upd_cell_per_sec;
				}
				let ups = total_ups / (measurements_count as f64);
				let upd_cell = total_upd_cell_per_sec / (measurements_count as f64);
				print!(" Done! ({} | {})\n", ups, upd_cell);

				final_table.push_str(&format!("{}; {}; {}; {}; {};\n", size_x, size_y, ticks, ups, upd_cell));
			}
		}
	}

	println!("Benchmark done!");
	println!("{}", final_table);

	let filename = "benchmark.csv";
	std::fs::write(filename, final_table).unwrap();
	println!("All data is saved to {}", filename);

	'running: loop {
		let frames_required = start_time.elapsed().as_secs_f64() * 60.0;
		let should_render = frames_required >= frame as f64;

		if app.run_simulation || app.run_until > world.cur_tick()
		{
			for _ in 0..100 {
				world.update();
				if !app.run_simulation && app.run_until <= world.cur_tick() {
					break;
				}
			}
			unsafe {
				glow_gl.finish();
			}
		}

		if !should_render {
			continue;
		}
		world.no_tick();
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
