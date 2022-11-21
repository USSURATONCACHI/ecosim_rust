use std::sync::Arc;
use glow::{Context, HasContext, NativeTexture, NativeVertexArray, Program};
use rand::Rng;
use crate::util::TickCounter;

#[derive(Clone, Debug)]
pub struct World {
	gl: Arc<Context>,

	program: Program,
	vertex_array: NativeVertexArray,
	current_buf: NativeTexture,
	next_buf: NativeTexture,

	size: (u64, u64),

	tps: TickCounter,
	tick: u64,
}

impl World {
	pub fn new(gl: Arc<Context>, size: (u64, u64)) -> Self {
		let arr_size = size.0 * size.1 * 3;
		let mut rng = rand::thread_rng();

		let mut initial_state: Box<[u8]> = vec![255_u8;arr_size as usize].into_boxed_slice();
		for x in 0..size.0 {
			for y in 0..size.1 {
				let val = if x <= 200 && rng.gen_bool(0.4) {
					255_u8
				} else {
					0_u8
				};

				let id = ((y * size.0 + x) * 3) as usize;
				initial_state[id + 0] = val;
				initial_state[id + 1] = val;
				initial_state[id + 2] = val;
			}
		}

		let create_texture = || {
			let texture;
			unsafe {
				texture = gl.create_texture().unwrap();
				gl.bind_texture(glow::TEXTURE_2D, Some(texture));
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
				gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGB8UI as i32,
								size.0 as i32, size.1 as i32, 0,
								glow::RGB, glow::UNSIGNED_BYTE, Some(&initial_state))
			}
			texture
		};

		let program;
		let vertex_array;
		unsafe {
			program = gl.create_program().expect("Cannot create program");
			let compute_shader_source = include_str!("game_of_life.glsl");

			let shader = gl.create_shader(glow::COMPUTE_SHADER).expect("Cannot create shader");
			gl.shader_source(shader, compute_shader_source);
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

			vertex_array = gl.create_vertex_array().expect("Cannot create vertex array");
		}
		let current_buf = create_texture();
		let next_buf = create_texture();

		World {
			gl,
			program,
			vertex_array,
			current_buf,
			next_buf,
			size,
			tps: TickCounter::new(30),
			tick: 0
		}
	}

	pub fn current_state(&self) -> NativeTexture {
		self.current_buf
	}

	pub fn size(&self) -> (u64, u64) {
		self.size.clone()
	}

	pub fn cur_tick(&self) -> u64 {
		self.tick
	}

	pub fn tps(&self) -> &TickCounter {
		&self.tps
	}

	pub fn update(&mut self) {
		unsafe {
			self.gl.use_program(Some(self.program));

			self.gl.active_texture(glow::TEXTURE0);
			self.gl.bind_texture(glow::TEXTURE_2D, Some(self.current_buf));
			self.gl.uniform_1_i32(self.gl.get_uniform_location(self.program, "current_state").as_ref(), 0);

			self.gl.active_texture(glow::TEXTURE1);
			self.gl.bind_texture(glow::TEXTURE_2D, Some(self.next_buf));
			self.gl.uniform_1_i32(self.gl.get_uniform_location(self.program, "next_state").as_ref(), 1);

			self.gl.dispatch_compute(self.size.0 as u32, self.size.1 as u32, 1);
			self.gl.memory_barrier(glow::ALL_BARRIER_BITS)
		}

		std::mem::swap(&mut self.current_buf, &mut self.next_buf);
		self.tps.tick();
		self.tick += 1;
	}

	pub fn no_tick(&mut self) {
		self.tps.no_tick();
	}
}