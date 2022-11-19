use glow::{Context, HasContext, NativeTexture, Program};
use rand::Rng;
use crate::util::TickCounter;

pub struct World {
	program: Program,
	current_state: NativeTexture,
	next_state: NativeTexture,

	size: (u64, u64),
	tick: u64,
	tps: TickCounter,

	is_destroyed: bool,
}

fn tmp_gen_image(size: (u64, u64)) -> Box<[u8]> {
	let byte_size = size.0 * size.1 * 3; // 3 channels - rgb
	let mut data = vec![0_u8;byte_size as usize].into_boxed_slice();
	let mut rng = rand::thread_rng();

	for x in 0..size.0 {
		for y in 0..size.1 {
			let pos = ((y * size.0 + x) * 3) as usize;

			if x <= 200 && rng.gen_bool(0.4) {
				data[pos + 0] = 255;
				data[pos + 1] = 255;
				data[pos + 1] = 255;
			}
		}
	}

	data
}

impl World {
	pub fn new(gl: &Context, size: (u64, u64)) -> Self {
		unsafe {
			let program = gl.create_program().expect("Cannot create program");
			let shader_source = String::from_utf8_lossy(include_bytes!("../assets/game_of_life.glsl"));

			let shader = gl.create_shader(glow::COMPUTE_SHADER).expect("Cannot create shader");
			gl.shader_source(shader, shader_source.as_ref());
			gl.compile_shader(shader);
			if !gl.get_shader_compile_status(shader) {
				panic!("{}", gl.get_shader_info_log(shader));
			}
			gl.attach_shader(program, shader);

			gl.link_program(program);
			if !gl.get_program_link_status(program) {
				panic!("{}", gl.get_program_info_log(program));
			}

			gl.detach_shader(program, shader);
			gl.delete_shader(shader);

			let texture_1 = gl.create_texture().unwrap();
			let texture_2 = gl.create_texture().unwrap();
			let state = tmp_gen_image(size);

			for tex in [texture_1, texture_2] {
				gl.bind_texture(glow::TEXTURE_2D, Some(tex));
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
				gl.tex_image_2d(glow::TEXTURE_2D, 0,
								glow::RGB8UI as i32, size.0 as i32, size.1 as i32,
								0, glow::RGB, glow::UNSIGNED_BYTE,
								Some(&state));
			}

			World {
				program,
				current_state: texture_1,
				next_state: texture_2,
				size,
				tick: 0,
				tps: TickCounter::new(30),
				is_destroyed: false
			}
		}
	}

	pub fn size(&self) -> (u64, u64) {
		self.size
	}

	pub fn cur_tick(&self) -> u64 {
		self.tick
	}

	pub fn tps(&self) -> &TickCounter {
		&self.tps
	}

	pub fn no_tick(&mut self) {
		self.tps.no_tick();
	}

	pub fn update(&mut self, gl: &Context) {
		unsafe {
			gl.use_program(Some(self.program));

			gl.active_texture(glow::TEXTURE0);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.current_state));

			gl.active_texture(glow::TEXTURE1);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.next_state));

			gl.uniform_1_i32(gl.get_uniform_location(self.program, "current_state").as_ref(), 0);
			gl.uniform_1_i32(gl.get_uniform_location(self.program, "next_state").as_ref(), 1);

			gl.dispatch_compute(self.size.0 as u32, self.size.1 as u32, 1);
			gl.memory_barrier(glow::ALL_BARRIER_BITS);
		}

		std::mem::swap(&mut self.current_state, &mut self.next_state);
		self.tps.tick();
		self.tick += 1;
	}
}