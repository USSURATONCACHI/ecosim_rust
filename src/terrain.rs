use std::sync::Arc;
use glow::{Context, HasContext, NativeBuffer, NativeProgram, NativeTexture};
use noise::NoiseFn;
use crate::glsl_expand::ShaderContext;
use crate::util::compile_program;

pub fn gen_height(noise: &impl NoiseFn<f64, 2>, size: (u64, u64)) -> Box<[i32]> {
	let arr_size = size.0 * size.1;
	let mut map: Box<[i32]> = vec![0; arr_size as usize].into_boxed_slice();

	for x in 0..size.0 {
		for y in 0..size.1 {
			let id = y * size.0 + x;
			let val = noise.get([x as f64, y as f64]) as f32;
			map[id as usize] = ((val / 2.0 + 0.5) * 1_000_000.0) as i32; // range [0; 1kk]
		}
	}

	map
}

pub fn convert_to_texture(gl: &Context, size: (u64, u64), data: &Box<[i32]>) -> NativeTexture {
	let texture;
	unsafe {
		texture = gl.create_texture().unwrap();
		gl.bind_texture(glow::TEXTURE_2D, Some(texture));
		gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
		gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
		let ptr = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * std::mem::size_of::<i32>());
		gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::R32I as i32,
						size.0 as i32, size.1 as i32, 0,
						glow::RED_INTEGER, glow::INT, Some(ptr));
	}
	texture
}


pub fn _gen_map(_seed: u64) {
	/*
	Gen amplitude map (voronoi + quadrants + noise)
		Should be pretty flat (~= 1.0) with few peaks (< or > 1.0) for mountains and depressions

	Gen base height map (noise * amplitude for each point)
	Erode height map

	// Gen temperature and humidity maps
	Set sea level, shallow waters level.
	Erode rivers from random peaks

	Determine base biomes:
		very steep + peak => peak
		very steep + flat => mountain
		height fluctuates => hills
		flat			  => plains
		<= shallow level  => shallow
		<= sea level	  => sea

	Plains near sea => beach

	// High temp + high humidity =>

	*/
}

// Erosion code is inspired from here: https://github.com/SebLague/Hydraulic-Erosion/blob/master/Assets/Scripts/Erosion.cs

/// Creates a map from each droplet position (`data[y * width + x]`) to
/// some amount of cells to erode from.
fn create_brush(map_size: (u64, u64), radius: u32) -> Vec<Vec<(i32, i32, f32)>> {
	let radius = radius as i32;
	let mut data: Vec<Vec<(i32, i32, f32)>> = Vec::with_capacity( (map_size.0 * map_size.1) as usize );

	for center_y in 0..map_size.1 {
		for center_x in 0..map_size.0 {
			let mut indices: Vec<(i32, i32, f32)> = vec![];
			let mut sum_weights = 0.0;

			for dy in (-radius)..=radius {
				for dx in (-radius)..=radius {
					let sqr_dist = dx * dx + dy * dy;

					if sqr_dist > (radius * radius) {
						continue;
					}

					let x = center_x as i32 + dx;
					let y = center_y as i32 + dy;

					if x < 0 || y < 0 || x >= map_size.0 as i32 || y >= map_size.1 as i32 {
						continue;
					}

					let weight = 1.0 - (sqr_dist as f32).sqrt() / (radius as f32);
					sum_weights += weight;

					indices.push((x, y, weight));
				}
			}
			let indices = indices
				.into_iter()
				.map(|(x, y, weight)| (x, y, weight / sum_weights))
				.collect();

			data.push(indices);
		}
	}

	data
}

/// Converts array of arrays into two SSBOs length of the first array:
/// First contains pure data (T objects)
/// Second one contains (start_index, size) pairs of each of inner arrays
fn convert_to_ssbo<T>(gl: &Context, data: Vec<Vec<T>>) -> (NativeBuffer, NativeBuffer, i32) {
	let data_buffer;
	let ptrs_buffer;
	let length = data.len();

	unsafe {
		data_buffer = gl.create_buffer().unwrap();
		ptrs_buffer = gl.create_buffer().unwrap();

		let mut data_array: Vec<T> = vec![];
		let mut pointers_array: Vec<(i32, i32)> = vec![];

		for array in data {
			pointers_array.push((data_array.len() as i32, array.len() as i32));
			data_array.extend(array);
		}

		let data_slice = std::slice::from_raw_parts(
			data_array.as_ptr() as *const u8,
			data_array.len() * std::mem::size_of::<T>()
		);

		let ptrs_slice = std::slice::from_raw_parts(
			pointers_array.as_ptr() as *const u8,
			pointers_array.len() * std::mem::size_of::<(i32, i32)>()
		);

		gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(data_buffer));
		gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, data_slice, glow::STATIC_DRAW);

		gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(ptrs_buffer));
		gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, ptrs_slice, glow::STATIC_DRAW);
	}

	(data_buffer, ptrs_buffer, length as i32)
}

#[derive(Debug, Clone)]
pub struct ErosionGpu {
	gl: Arc<Context>,
	copy_program: NativeProgram,
	erosion_program: NativeProgram,
	brush: (NativeBuffer, NativeBuffer),
	size: (u64, u64),

	tmp_texture: NativeTexture,
}

impl ErosionGpu {
	pub fn new(gl: Arc<Context>, glsl_manager: &mut ShaderContext, map_size: (u64, u64)) -> Self {
		let mut load_program = |path: &str| {
			let shader = glsl_manager
				.get_file_processed(path).unwrap()
				.current_text().clone();
			let sources = [
				(glow::COMPUTE_SHADER, shader.as_str())
			];
			compile_program(&gl, sources).expect(format!("Failed to compile: \n{}", shader).as_str())
		};

		let copy_program = load_program("assets/copy_texture.glsl");
		let erosion_program = load_program("assets/terrain/erosion.glsl");

		let brush = create_brush(map_size, 3);
		let brush = convert_to_ssbo(&gl, brush);
		let brush = (brush.0, brush.1);

		let tmp_texture;
		unsafe {
			tmp_texture = gl.create_texture().unwrap();
			gl.bind_texture(glow::TEXTURE_2D, Some(tmp_texture));
			gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
			gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
			gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::R32I as i32,
							map_size.0 as i32, map_size.1 as i32, 0,
							glow::RED_INTEGER, glow::UNSIGNED_BYTE, None);
		}

		ErosionGpu {
			gl,
			copy_program,
			erosion_program,
			brush,
			size: map_size,
			tmp_texture,
		}
	}

	pub fn erode(&mut self, texture: NativeTexture, iterations: u64, rand_seed: i32) -> NativeTexture {
		const COPY_TEXTURE_WORK_GROUP_SIZE: u32 = 32;
		let gl = self.gl.clone();

		let mut current_texture = texture;
		let mut next_texture = self.tmp_texture;
		unsafe {
			gl.use_program(Some(self.erosion_program));
			gl.uniform_2_i32(gl.get_uniform_location(self.erosion_program, "u_map_size").as_ref(), self.size.0 as i32, self.size.1 as i32);
			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(self.brush.0));
			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 2, Some(self.brush.1));

			let copy_invocs_x =  (self.size.0 as u32 + COPY_TEXTURE_WORK_GROUP_SIZE - 1) / COPY_TEXTURE_WORK_GROUP_SIZE;
			let copy_invocs_y =  (self.size.1 as u32 + COPY_TEXTURE_WORK_GROUP_SIZE - 1) / COPY_TEXTURE_WORK_GROUP_SIZE;

			for i in 0..iterations {
				// 1. Copy image to buffer
				gl.use_program(Some(self.copy_program));
				gl.bind_image_texture(0, current_texture, 0, false, 0, glow::READ_ONLY, glow::R32I);
				gl.bind_image_texture(1, next_texture, 0, false, 0, glow::WRITE_ONLY, glow::R32I);
				gl.dispatch_compute(copy_invocs_x, copy_invocs_y, 1);
				gl.memory_barrier(glow::ALL_BARRIER_BITS);

				// 2. Emulate droplets
				gl.use_program(Some(self.erosion_program));
				gl.uniform_1_i32(gl.get_uniform_location(self.erosion_program, "u_random_seed").as_ref(), i as i32 + rand_seed);
				gl.bind_image_texture(0, current_texture, 0, false, 0, glow::READ_ONLY, glow::R32I);
				gl.bind_image_texture(1, next_texture, 0, false, 0, glow::WRITE_ONLY, glow::R32I);
				gl.dispatch_compute((self.size.0 as u32 + 255) / 256, (self.size.1 as u32 + 255) / 256, 1);
				gl.memory_barrier(glow::ALL_BARRIER_BITS);

				// 3. Swap buffers
				std::mem::swap(&mut current_texture, &mut next_texture);
			}

			gl.finish();
		}

		self.tmp_texture = next_texture;
		current_texture
	}
}

impl Drop for ErosionGpu {
	fn drop(&mut self) {
		let gl = self.gl.clone();
		unsafe {
			gl.delete_program(self.copy_program);
			gl.delete_program(self.erosion_program);
			gl.delete_buffer(self.brush.0);
			gl.delete_buffer(self.brush.1);
			gl.delete_texture(self.tmp_texture);
		}
	}
}