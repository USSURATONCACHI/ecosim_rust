use std::sync::Arc;
use glow::{Context, HasContext, NativeBuffer, NativeProgram, NativeTexture, PixelPackData};
use noise::NoiseFn;
use rand::{Rng, SeedableRng};
use rand::rngs::{StdRng, ThreadRng};
use crate::glsl_expand::ShaderContext;
use crate::util::compile_program;

const CELL_EMPTY: u8 = 0;
const CELL_FILLED: u8 = 1;
const CELL_CHECKED: u8 = 2;

fn _into_cells(size: (usize, usize), data: &[bool]) -> (Box<[u8]>, u32) {
	let mut map: Box<[u8]> = data.into_iter()
		.map(|x| *x as u8)
		.collect();
	let id = |x: usize, y: usize| y * size.0 + x;

	let mut cells_taken = 0_u32;
	for x in 0..size.0 {
		if map[id(x, 0)] == CELL_EMPTY {
			map[id(x, 0)] = CELL_CHECKED;
			cells_taken += 1;
		}

		if map[id(x, size.1 - 1)] == CELL_EMPTY {
			map[id(x, size.1 - 1)] = CELL_CHECKED;
			cells_taken += 1;
		}
	}
	for y in 1..(size.1 - 1) {
		if map[id(0, y)] == CELL_EMPTY {
			map[id(0, y)] = CELL_CHECKED;
			cells_taken += 1;
		}

		if map[id(size.0 - 1, y)] == CELL_EMPTY {
			map[id(size.0 - 1, y)] = CELL_CHECKED;
			cells_taken += 1;
		}
	}

	(map, cells_taken)
}

pub struct ShapeSmoother {
	gl: Arc<Context>,
	program: NativeProgram,

	texture_1: NativeTexture,
	texture_2: NativeTexture,
	cells_taken_buf: NativeBuffer,
}

impl ShapeSmoother {
	pub fn new(gl: Arc<Context>, shader_context: &mut ShaderContext) -> Self {
		let shader = shader_context
			.get_file_processed("assets/shape/smoother.glsl").unwrap()
			.current_text().clone();
		let sources = [
			(glow::COMPUTE_SHADER, shader.as_str())
		];
		let program = compile_program(&gl, sources).expect(format!("Failed to compile: \n{}", shader).as_str());

		let mut texture_1;
		let mut texture_2;
		let cells_taken_buf;
		unsafe {
			let create = || {
				let tex = gl.create_texture().unwrap();
				gl.bind_texture(glow::TEXTURE_2D, Some(tex));
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
				gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
				tex
			};

			// Create textures with data for shader
			texture_1 = create();
			texture_2 = create();
			// Create buffer to store taken cells count
			cells_taken_buf = gl.create_buffer().unwrap();
		}

		ShapeSmoother {
			gl,
			program,
			texture_1,
			texture_2,
			cells_taken_buf,
		}
	}

	pub fn smooth_out(&self, size: (usize, usize), data: &mut [bool]) {
		let gl = self.gl.clone();
		let (mut map, mut cells_taken) = _into_cells(size, data);
		// let id = |x: usize, y: usize| y * size.0 + x;

		let tex_image_2d = unsafe {
			|data: Option<&[u8]>|
				gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::R8UI as i32,
								size.0 as i32, size.1 as i32, 0,
								glow::RED_INTEGER, glow::UNSIGNED_BYTE, data)
		};

		// Send data to GPU
		unsafe {
			gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.cells_taken_buf));
			let pointer = std::slice::from_raw_parts(&cells_taken as *const u32 as *const u8, std::mem::size_of::<u32>());
			gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, pointer, glow::DYNAMIC_DRAW);

			gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_1));
			tex_image_2d(Some(&map));
			gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_2));
			tex_image_2d(Some(&map));
		}

		const WORK_GROUP_SIZE: u32 = 32;
		let call_size_x = (size.0 as u32 + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE;
		let call_size_y = (size.1 as u32 + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE;

		let calls_per_cycle = (size.0.min(size.1) + 1) / 10;

		let mut curr_image = self.texture_1;
		let mut next_image = self.texture_2;

		unsafe {
			// Call shader enough times to compute everything.
			gl.use_program(Some(self.program));
			gl.uniform_2_i32(gl.get_uniform_location(self.program, "u_size").as_ref(), size.0 as i32, size.1 as i32);
			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 2, Some(self.cells_taken_buf));

			loop {
				for _ in 0..calls_per_cycle {
					gl.bind_image_texture(0, curr_image, 0, false, 0, glow::READ_ONLY, glow::R8UI);
					gl.bind_image_texture(1, next_image, 0, false, 0, glow::READ_WRITE, glow::R8UI);
					gl.dispatch_compute(call_size_x, call_size_y, 1);
					gl.memory_barrier(glow::ALL_BARRIER_BITS);

					std::mem::swap(&mut curr_image, &mut next_image);
				}

				gl.finish();
				let start_cells_taken = cells_taken;

				// Get amount of taken cells
				let pointer = &mut cells_taken as *mut u32 as *mut u8;
				let pointer = std::slice::from_raw_parts_mut(pointer, std::mem::size_of::<u32>());
				gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.cells_taken_buf));
				gl.get_buffer_sub_data(glow::SHADER_STORAGE_BUFFER, 0, pointer);

				// No new cells is taken
				if cells_taken == start_cells_taken {
					break;
				}
			}

			// Copy data from GPU
			gl.bind_texture(glow::TEXTURE_2D, Some(curr_image));
			gl.get_tex_image(glow::TEXTURE_2D, 0, glow::RED_INTEGER, glow::UNSIGNED_BYTE, PixelPackData::Slice(&mut map));
		}

		println!("Copied");
		for (value, write_to) in map.into_iter().zip(data.iter_mut()) {
			*write_to = *value != CELL_CHECKED;
		}
	}
}

impl Drop for ShapeSmoother {
	fn drop(&mut self) {
		unsafe {
			self.gl.delete_program(self.program);
			self.gl.delete_texture(self.texture_1);
			self.gl.delete_texture(self.texture_2);
			self.gl.delete_buffer(self.cells_taken_buf);
		}
	}
}

pub fn generate_map(size: (u64, u64), motion_length: u32, continents_count: u32, smoother: &ShapeSmoother, noise: impl NoiseFn<f64, 2>) -> Box<[i32]> {
	let map_area = (size.0 * size.1) as usize;
	let mut map: Box<[i32]> = vec![0; map_area].into_boxed_slice();
	let mut shape_buffer: Box<[bool]> = vec![false; map_area].into_boxed_slice();

	let cell_weight = |dx: i32, dy: i32| {
		let x = (dx * dx + dy * dy) as f64;
		(1.0 / (1.0 + x*x)) / (std::f64::consts::PI)
	};

	let mut rng = StdRng::seed_from_u64(44);
	for i in 0..continents_count {
		let width = 5;
		let x = ((i % width) + 1) * (size.0 as u32) / (width + 1); // rng.gen_range(0..size.0);
		let y = ((i / width) + 1) * (size.1 as u32) / (width + 1); // rng.gen_range(0..size.1);

		let x = x.clamp(0, size.0 as u32 - 1) as u64;
		let y = y.clamp(0, size.1 as u32 - 1) as u64;

		shape_buffer.fill(false);
		generate_shape(size, &mut shape_buffer, rng.gen_range(1..motion_length), (x, y), &mut rng);

		for i in 0..map_area {
			if shape_buffer[i] {
				shape_buffer[i] = false;
				let x = (i % (size.0 as usize)) as i32;
				let y = (i / (size.0 as usize)) as i32;
				let id = y * (size.0 as i32) + x;
				map[id as usize] += 1;
			}
		}
	}

	let mut map_bool: Box<[bool]> = map.iter().map(|x| *x > 0).collect();
	smoother.smooth_out((size.0 as usize, size.1 as usize), &mut map_bool);

	for i in 0..map_area {
		let mut continents_count = map[i];
		// TODO: Make this calculate from local avg
		if map_bool[i] && continents_count == 0 {
			continents_count = 1;
		}
		let x = (i as u64) % (size.0);
		let y = (i as u64) / (size.0);

		let height = (continents_count as f32 + 1.0).ln() / 2.0_f32.ln() / 1.6;
		let noise_component = (noise.get([x as f64, y as f64]).powf(3.0) / 10.0) as f32;
		map[i] = (1_000_000.0 * (height + noise_component)) as i32;

		/*
		for dx in -0..=1 {
			for dy in -0..=1 {
				let x = (x as i32 + dx).clamp(0, size.0 as i32 - 1);
				let y = (y as i32 + dy).clamp(0, size.1 as i32 - 1);
				let id = y * (size.0 as i32) + x;
				let weight = cell_weight(dx, dy);
				map[id as usize] += (1_000_000.0 * (height as f64) / 64.0) as i32;
				if map[id as usize] > 1_000_000 {
					println!("x {} y {} val {}", x, y, map[id as usize]);
				}
			}
		}*/
	}

	map
}

pub fn generate_shape(size: (u64, u64), buffer: &mut [bool], motion_length: u32, pos: (u64, u64), rng: &mut StdRng) {
	let mut x = pos.0 as i32;
	let mut y = pos.1 as i32;

	for _ in 0..motion_length {
		let id = y * (size.0 as i32) + x;
		buffer[id as usize] = true;

		let dx = rng.gen_range(-1..=1);
		let dy = rng.gen_range(-1..=1);

		x = (x + dx).clamp(0, size.0 as i32 - 1);
		y = (y + dy).clamp(0, size.1 as i32 - 1);
	}
}


pub fn to_data(noise: &impl NoiseFn<f64, 2>, size: (u64, u64), func: impl Fn(f32) -> f32) -> Box<[i32]> {
	let arr_size = size.0 * size.1;
	let mut map: Box<[i32]> = vec![0; arr_size as usize].into_boxed_slice();

	for x in 0..size.0 {
		for y in 0..size.1 {
			let id = y * size.0 + x;
			let val = noise.get([x as f64, y as f64]) as f32;
			let x = val / 2.0 + 0.5;
			map[id as usize] = (func(x) * 1_000_000.0) as i32; // range [0; 1kk]
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