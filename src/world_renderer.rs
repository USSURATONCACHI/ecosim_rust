use glow::NativeTexture;
use crate::World;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AntiAliasing {
	MSAAx1 = 1,
	MSAAx2 = 2,
	MSAAx3 = 3,
	MSAAx4 = 4,
	MSAAx5 = 5,
	SSAAx9 = 9,
	SSAAx16 = 16,
}

impl AntiAliasing {
	pub fn all_values() -> &'static [AntiAliasing] {
		&[
			AntiAliasing::MSAAx1,
			AntiAliasing::MSAAx2,
			AntiAliasing::MSAAx3,
			AntiAliasing::MSAAx4,
			AntiAliasing::MSAAx5,
			AntiAliasing::SSAAx9,
			AntiAliasing::SSAAx16,
		]
	}
}

pub struct WorldRenderer {
	program: glow::Program,
	vertex_array: glow::VertexArray,

	world_texture: NativeTexture,
	last_redraw_tick: Option<u64>,

	is_destroyed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SendPtr<T>(pub *const T);
unsafe impl <T> Send for SendPtr<T> {}
unsafe impl <T> Sync for SendPtr<T> {}

#[derive(Clone, Debug)]
pub struct PaintData {
	pub world: SendPtr<World>,
	pub screen_size: (f32, f32),
	pub camera_pos: (f32, f32),
	pub zoom: f32,
	pub antialiasing: AntiAliasing,
}

impl WorldRenderer {
	pub fn new(gl: &glow::Context) -> Self {
		use glow::HasContext as _;

		unsafe {
			let program = gl.create_program().expect("Cannot create program");

			let vertex_shader_source =
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

			let fragment_shader_source = String::from_utf8_lossy(include_bytes!("../assets/main.glsl"));

			let shader_sources = [
				(glow::VERTEX_SHADER, vertex_shader_source),
				(glow::FRAGMENT_SHADER, &fragment_shader_source),
			];

			let shaders: Vec<_> = shader_sources
				.iter()
				.map(|(shader_type, shader_source)| {
					let shader = gl
						.create_shader(*shader_type)
						.expect("Cannot create shader");
					gl.shader_source(shader, shader_source);
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

			let world_texture = gl.create_texture().unwrap();
			gl.bind_texture(glow::TEXTURE_2D, Some(world_texture));
			gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
			gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

			Self {
				program,
				vertex_array,
				world_texture,
				last_redraw_tick: None,
				is_destroyed: false,
			}
		}
	}

	pub fn destroy(&mut self, gl: &glow::Context) {
		use glow::HasContext as _;
		unsafe {
			gl.delete_texture(self.world_texture);
			gl.delete_program(self.program);
			gl.delete_vertex_array(self.vertex_array);
		}
		self.is_destroyed = true;
	}

	pub fn paint(&mut self, gl: &glow::Context, data: PaintData) {
		use glow::HasContext as _;
		let world = unsafe { data.world.0.as_ref().unwrap() };

		let world_size = world.size();

		unsafe {
			gl.use_program(Some(self.program));
		}

		if self.last_redraw_tick != Some(world.cur_tick()) {
			self.last_redraw_tick = Some(world.cur_tick());
			let mut rgba: Box<[u8]> = vec![128_u8; world_size.0 * world_size.1 * 4].into_boxed_slice();

			for y in 0..world_size.1 {
				for x in 0..world_size.0 {
					let i = y * world_size.0 + x;
					let cell = world.cell(x, y);
					let texel: (u8, u8, u8, u8) = if cell.state { (255, 255, 255, 255) } else { (1, 1, 1, 255) };
					rgba[i * 4 + 0] = texel.0;
					rgba[i * 4 + 1] = texel.1;
					rgba[i * 4 + 2] = texel.2;
					rgba[i * 4 + 3] = texel.3;
				}
			}

			unsafe {
				gl.active_texture(glow::TEXTURE2);
				gl.bind_texture(glow::TEXTURE_2D, Some(self.world_texture));
				gl.tex_image_2d(glow::TEXTURE_2D, 0,
								glow::RGBA as i32, world_size.0 as i32, world_size.1 as i32,
								0, glow::RGBA, glow::UNSIGNED_BYTE,
								Some(&rgba));
			}
		}

		unsafe {
			gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_world_texture").as_ref(), 2);
			gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_antialiasing").as_ref(), data.antialiasing as i32);

			gl.uniform_2_f32(gl.get_uniform_location(self.program, "u_world_size").as_ref(), world_size.0 as f32, world_size.1 as f32);
			gl.uniform_2_f32(gl.get_uniform_location(self.program, "u_screen_size").as_ref(), data.screen_size.0, data.screen_size.1);
			gl.uniform_2_f32(gl.get_uniform_location(self.program, "u_camera_pos").as_ref(), data.camera_pos.0, data.camera_pos.1);
			gl.uniform_1_f32(gl.get_uniform_location(self.program, "u_camera_zoom").as_ref(), data.zoom);

			gl.bind_vertex_array(Some(self.vertex_array));
			gl.draw_arrays(glow::TRIANGLES, 0, 6);
		}
	}
}

impl Drop for WorldRenderer {
	fn drop(&mut self) {
		if !self.is_destroyed {
			println!("ERROR: WorldRenderer was not properly destroyed (`.destroy()`) before dropping");
		}
	}
}