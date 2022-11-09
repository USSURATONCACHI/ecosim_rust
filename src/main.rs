extern crate gl;
extern crate winit;
extern crate winapi;
extern crate raw_gl_context;
extern crate sciter;

mod util;

use std::time::{Duration, Instant};
use raw_gl_context::{GlConfig, GlContext};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use sciter::Host;
use sciter::windowless::RenderEvent;
use winit::{
	event::{Event, WindowEvent},
	event_loop::EventLoop,
	window::WindowBuilder,
};
use winit::event::StartCause;
use crate::util::{current_time_nanos, TickCounter};


fn main() {
	let event_loop = EventLoop::new();

	let window = WindowBuilder::new()
		.with_title("A fantastic window!")
		.with_inner_size(winit::dpi::LogicalSize::new(800, 600))
		.build(&event_loop)
		.unwrap();

	let window_handle = window.raw_window_handle();
	let context = GlContext::create(&window, GlConfig::default()).unwrap();
	context.make_current();
	gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

	// let window_handle = window.raw_window_handle();
	// create an engine instance with an opaque pointer as an identifier
	use sciter::windowless::{Message, handle_message};
	let sciter_window = { &window as *const _ as sciter::types::HWINDOW };
	handle_message(sciter_window, Message::Create { backend: sciter::types::GFX_LAYER::SKIA_OPENGL, transparent: true });
	let sciter_host = Host::attach(sciter_window);
	let path = std::env::current_dir().unwrap().join("minimal.htm");
	sciter_host.load_file(path.to_str().unwrap());


	let mut pack_start = current_time_nanos();
	let pack_size = 64;
	let target_fps = 4_000_u128; // millihertz

	let mut fps_counter = TickCounter::new(30);
	let mut frame: u128 = 0;

	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent {
				event: WindowEvent::CloseRequested,
				..
			} => {
				control_flow.set_exit();
			}
			Event::RedrawRequested(_) => {
				frame += 1;
				fps_counter.tick();
				context.make_current();

				let on_render = move |bitmap_area: &sciter::types::RECT, bitmap_data: &[u8]|
					on_sciter_render(window_handle, bitmap_area, bitmap_data);
				let cb = RenderEvent {
					layer: None,
					callback: Box::new(on_render),
				};

				handle_message(sciter_window, Message::RenderTo(cb));

				window.set_title(&format!("Ecosim | FPS: {:.2}", fps_counter.tps_corrected()));

				unsafe {
					let i = ((frame % 256) as f32) / 255.0;
					gl::ClearColor(1.0, i, 1.0 - i / 4.0, 1.0);
					gl::Clear(gl::COLOR_BUFFER_BIT);
				}

				context.swap_buffers();
				context.make_not_current();
			}
			Event::NewEvents(StartCause::Init) => {
				control_flow.set_wait_until(Instant::now());
			}
			Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
				// VSync parody
				let target_fps = match window.current_monitor() {
					None => target_fps,
					Some(monitor) => match monitor.refresh_rate_millihertz() {
						None => target_fps,
						Some(rate) => rate as u128,
					}
				};

				let next_frame_time = pack_start + (frame % pack_size) * 1_000_000_000_u128 * 1000_u128 / target_fps;
				let now = current_time_nanos();
				if frame % pack_size == 0 {
					pack_start = now;
				}

				if now < next_frame_time {
					control_flow.set_wait_until(Instant::now() + Duration::from_nanos((next_frame_time - now) as u64));
				}
				window.request_redraw();
			}

			_other => {
				//println!("Event {:?}", other);
			}
		}
	});

	/*let event_loop = EventLoop::new();
	let window_builder = WindowBuilder::new().with_title("a window title");

	// Set this to OpenGL 3.3
	let context = ContextBuilder::new()
		.with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
		.with_vsync(true)
		.build_windowed(window_builder, &event_loop)
		.unwrap();

	let context = unsafe { context.make_current().unwrap() };*/
}

fn on_sciter_render(window_handle: RawWindowHandle, bitmap_area: &sciter::types::RECT, bitmap_data: &[u8]) {
	#[cfg(unix)]
	{
		let _ = bitmap_area;
		let _ = bitmap_data;
		let _ = window_handle;
	}

	// Windows-specific bitmap rendering on the window
	#[cfg(windows)]
	{
		use winapi::um::winuser::*;
		use winapi::um::wingdi::*;
		use winapi::shared::minwindef::LPVOID;

		let hwnd = match window_handle {
			RawWindowHandle::Win32(data) => data.hwnd as winapi::shared::windef::HWND,
			_ => unreachable!(),
		};

		unsafe {
			// NOTE: we use `GetDC` here instead of `BeginPaint`, because the way
			// winit 0.19 processed the `WM_PAINT` message (it always calls `DefWindowProcW`).

			// let mut ps = PAINTSTRUCT::default();
			// let hdc = BeginPaint(hwnd, &mut ps as *mut _);

			let hdc = GetDC(hwnd);

			let (w, h) = (bitmap_area.width(), bitmap_area.height());

			let mem_dc = CreateCompatibleDC(hdc);
			let mem_bm = CreateCompatibleBitmap(hdc, w, h);

			let mut bmi = BITMAPINFO::default();
			{
				let mut info = &mut bmi.bmiHeader;
				info.biSize = std::mem::size_of::<BITMAPINFO>() as u32;
				info.biWidth = w;
				info.biHeight = -h;
				info.biPlanes = 1;
				info.biBitCount = 32;
			}

			let old_bm = SelectObject(mem_dc, mem_bm as LPVOID);

			let _copied = StretchDIBits(mem_dc, 0, 0, w, h, 0, 0, w, h, bitmap_data.as_ptr() as *const _, &bmi as *const _, 0, SRCCOPY);
			let _ok = BitBlt(hdc, 0, 0, w, h, mem_dc, 0, 0, SRCCOPY);

			SelectObject(mem_dc, old_bm);

			// EndPaint(hwnd, &ps as *const _);
			ReleaseDC(hwnd, hdc);

			// println!("+ {} {}", w, h);
		}
	}
}