use std::sync::Arc;
use glow::{Context, HasContext, NativeTexture, Program, VertexArray};
use rand::Rng;
use crate::app::AntiAliasing;
use crate::util::TickCounter;

const RENDER_VERT_SOURCE: &str =
r#"
	#version 330
	const vec2 verts[6] = vec2[6](
		vec2(-1.0, -1.0),
		vec2(-1.0, 1.0),
		vec2(1.0, 1.0),
		vec2(1.0, 1.0),
		vec2(1.0, -1.0),
		vec2(-1.0, -1.0)
	);

	out vec2 f_tex_coords;
	uniform float u_angle;

	void main() {
		f_tex_coords = verts[gl_VertexID] / 2.0;
		gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
	}
"#;
const RENDER_FRAG_SOURCE: &str = include_str!("../assets/main.glsl");

pub fn compile_program<'a>(gl: &Context, shader_sources: impl IntoIterator<Item = (u32, &'a str)>) -> Result<Program, String> {
	use glow::HasContext as _;
	unsafe {
		let program = gl.create_program()
			.map_err(|err| format!("Cannot create program: {}", err))?;

		let shaders: Vec<_> = shader_sources
			.into_iter()
			.map(|(shader_type, shader_source)| {
				let shader = gl
					.create_shader(shader_type)
					.map_err(|err| format!("Cannot create shader: {}", err))
					.unwrap();	// TODO

				gl.shader_source(shader, shader_source);
				gl.compile_shader(shader);
				if !gl.get_shader_compile_status(shader) {
					panic!("Cannot compile shader: {}", gl.get_shader_info_log(shader));
				}
				gl.attach_shader(program, shader);
				shader
			})
			.collect();

		gl.link_program(program);
		if !gl.get_program_link_status(program) {
			return Err(format!("Cannot link program: {}", gl.get_program_info_log(program)));
		}

		for shader in shaders {
			gl.detach_shader(program, shader);
			gl.delete_shader(shader);
		}

		Ok(program)
	}
}

#[derive(Clone, Debug)]
pub struct PaintData {
	pub screen_size: (f32, f32),
	pub camera_pos: (f32, f32),
	pub zoom: f32,
	pub antialiasing: AntiAliasing,
}

#[derive(Clone, Debug)]
pub struct World {
	gl: Arc<Context>,

	program: Program,
	current_buf: NativeTexture,
	next_buf: NativeTexture,

	size: (u64, u64),

	tps: TickCounter,
	tick: u64,

	render_program: Program,
	vertex_array: VertexArray,
	
	max_work_group_count: (usize, usize),
}

impl World {
	pub fn new(gl: Arc<Context>, size: (u64, u64)) -> Self {
		let arr_size = size.0 * size.1;
		let mut rng = rand::thread_rng();

		let mut initial_state: Box<[u8]> = vec![0_u8;arr_size as usize].into_boxed_slice();
		let empty_state: Box<[u8]> = vec![0_u8;arr_size as usize].into_boxed_slice();
		for x in 0..size.0 {
			for y in 0..size.1 {
				let val = if rng.gen_bool(0.55) {
					1_u8
				} else {
					0_u8
				};

				let id = (y * size.0 + x) as usize;
				initial_state[id + 0] = val;
			}
		}

		let create_texture = |state: &[u8]| {
			let texture;
			unsafe {
				texture = gl.create_texture().unwrap();
				gl.bind_texture(glow::TEXTURE_2D, Some(texture));
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
				gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::R8UI as i32,
								size.0 as i32, size.1 as i32, 0,
								glow::RED_INTEGER, glow::UNSIGNED_BYTE, Some(state))
			}
			texture
		};
		let current_buf = create_texture(&initial_state);
		let next_buf = create_texture(&empty_state);

		let sources = [
			(glow::COMPUTE_SHADER, include_str!("game_of_life.glsl"))
		];
		let program = compile_program(&gl, sources).unwrap();

		let render_sources = [
			(glow::VERTEX_SHADER, RENDER_VERT_SOURCE),
			(glow::FRAGMENT_SHADER, RENDER_FRAG_SOURCE),
		];
		let render_program = compile_program(&gl, render_sources).unwrap();
		let vertex_array = unsafe { gl.create_vertex_array().unwrap() };

		let max_wg_x;
		let max_wg_y;
		unsafe {
			gl.use_program(Some(program));
			gl.uniform_2_i32( gl.get_uniform_location(program, "world_size").as_ref(), size.0 as i32, size.1 as i32);
			max_wg_x = gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_COUNT, 0) as usize;
			max_wg_y = gl.get_parameter_indexed_i32(glow::MAX_COMPUTE_WORK_GROUP_COUNT, 1) as usize;
		}

		World {
			gl,
			program,
			current_buf,
			next_buf,
			size,
			tps: TickCounter::new(30),
			tick: 0,
			render_program,
			vertex_array,
			max_work_group_count: (max_wg_x, max_wg_y),
		}
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

	pub fn use_program(&self) {
		unsafe {
			self.gl.use_program(Some(self.program));
		}
	}

	pub fn update(&mut self) {
		const WORK_GROUP_SIZE: u64 = 32;
		unsafe {
			// self.gl.use_program(Some(self.program));
			self.gl.bind_image_texture(0, self.current_buf, 0, false, 0, glow::READ_WRITE, glow::R8UI);
			self.gl.bind_image_texture(1, self.next_buf, 0, false, 0, glow::READ_WRITE, glow::R8UI);
			let calls_x = (self.size().0 + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE;
			let calls_y = (self.size().1 + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE;

			if calls_x <= self.max_work_group_count.0 as u64 && calls_y <= self.max_work_group_count.1 as u64 {
				if self.tick == 0 {
					self.gl.uniform_2_u32( self.gl.get_uniform_location(self.program, "tile_offset").as_ref(), 0, 0);
				}
				self.gl.dispatch_compute(calls_x as u32, calls_y as u32, 1);
			} else {
				// Tiling
				for tile_x in (0..calls_x).step_by(self.max_work_group_count.0) {
					let tile_x_size = (calls_x - tile_x).min(self.max_work_group_count.0 as u64);

					for tile_y in (0..calls_y).step_by(self.max_work_group_count.1) {
						let tile_y_size = (calls_y - tile_y).min(self.max_work_group_count.1 as u64);
						self.gl.uniform_2_u32( self.gl.get_uniform_location(self.program, "tile_offset").as_ref(), tile_x as u32, tile_y as u32);
						self.gl.dispatch_compute(tile_x_size as u32, tile_y_size as u32, 1);
					}
				}
			}

			self.gl.memory_barrier(glow::ALL_BARRIER_BITS);
			self.tps.tick();
			self.tick += 1;
		}
		std::mem::swap(&mut self.current_buf, &mut self.next_buf);
	}

	pub fn render(&self, data: PaintData) {
		let gl = &self.gl;
		unsafe {
			gl.use_program(Some(self.render_program));

			let loc = |name: &str| gl.get_uniform_location(self.render_program, name);
			gl.active_texture(glow::TEXTURE2);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.current_buf));
			gl.uniform_1_i32(loc("u_world_texture").as_ref(), 2);
			gl.uniform_1_i32(loc("u_antialiasing").as_ref(), data.antialiasing as i32);

			gl.uniform_2_f32(loc("u_world_size").as_ref(), self.size.0 as f32, self.size.1 as f32);
			gl.uniform_2_f32(loc("u_screen_size").as_ref(), data.screen_size.0, data.screen_size.1);
			gl.uniform_2_f32(loc("u_camera_pos").as_ref(), data.camera_pos.0, data.camera_pos.1);
			gl.uniform_1_f32(loc("u_camera_zoom").as_ref(), data.zoom);

			gl.bind_vertex_array(Some(self.vertex_array));
			gl.draw_arrays(glow::TRIANGLES, 0, 6);
		}
	}

	pub fn no_tick(&mut self) {
		self.tps.no_tick();
	}
}
