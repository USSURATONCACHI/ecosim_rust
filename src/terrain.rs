use std::sync::Arc;
use glow::{Context, HasContext, NativeBuffer, NativeProgram, NativeTexture};
use image::EncodableLayout;
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
		let ptr = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * std::mem::size_of::<i32>()) };
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

// Erosion code is copied (and modified) from here: https://github.com/SebLague/Hydraulic-Erosion/blob/master/Assets/Scripts/Erosion.cs
// Original code licence:

/*
MIT License

Copyright (c) 2019 Sebastian Lague

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

#[derive(Debug, Clone)]
pub struct ErosionGpu {
	gl: Arc<Context>,
	program: NativeProgram,
	eb_indices: (NativeBuffer, NativeBuffer, i32),
	eb_weights: (NativeBuffer, NativeBuffer, i32),
}

fn create_brush_indices(map_size: (u64, u64), radius: u32) -> (Vec<Vec<i32>>, Vec<Vec<f32>>) {
	let radius = radius as i32;
	let arr_len = (map_size.0 * map_size.1) as usize;
	let mut erosion_brush_indices = vec![vec![]; arr_len];
	let mut erosion_brush_weights = vec![vec![]; arr_len];

	let arr_len = (radius * radius * 4) as usize;
	let mut x_offsets: Vec<i32> = vec![0; arr_len];
	let mut y_offsets: Vec<i32> = vec![0; arr_len];
	let mut weights: Vec<f32> = vec![0.0; arr_len];
	let mut weight_sum = 0.0;
	let mut add_index = 0;

	for i in 0..erosion_brush_indices.len() {
		let centre_x = (i as i32) % (map_size.0 as i32);
		let centre_y = (i as i32) / (map_size.0 as i32);

		if centre_y <= radius || centre_y >= (map_size.1 as i32) - radius || centre_x <= radius + 1 || centre_x >= (map_size.0 as i32) - radius {
			weight_sum = 0.0;
			add_index = 0;

			for y in (-radius)..=radius {
				for x in (-radius)..=radius {
					let sqr_dist = x * x + y * y;

					if sqr_dist < (radius * radius) {
						let coord_x = centre_x + x;
						let coord_y = centre_y + y;

						if coord_x >= 0 && coord_x < map_size.0 as i32 && coord_y >= 0 && coord_y < map_size.1 as i32 {
							let weight = 1.0 - (sqr_dist as f32).sqrt() / (radius as f32);
							weight_sum += weight;
							weights[add_index] = weight;
							x_offsets[add_index] = x;
							y_offsets[add_index] = y;
							add_index += 1;
						}
					}
				}
			}
		}

		let num_entries = add_index;
		erosion_brush_indices[i] = vec![0; num_entries];
		erosion_brush_weights[i] = vec![0.0; num_entries];

		for j in 0..num_entries {
			erosion_brush_indices[i][j] = (y_offsets[j] + centre_y) * (map_size.0 as i32) + x_offsets[j] + centre_x;
			erosion_brush_weights[i][j] = weights[j] / weight_sum;
		}
	}

	(erosion_brush_indices, erosion_brush_weights)
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

impl ErosionGpu {
	pub fn new(gl: Arc<Context>, glsl_manager: &mut ShaderContext, map_size: (u64, u64)) -> Self {
		let shader = glsl_manager
			.get_file_processed("assets/erosion.glsl").unwrap()
			.current_text().clone();
		let sources = [
			(glow::COMPUTE_SHADER, shader.as_str())
		];
		let program = compile_program(&gl, sources).unwrap();

		let (erosion_brush_indices, erosion_brush_weights) = create_brush_indices(map_size, 3);


		let eb_indices = convert_to_ssbo(&gl, erosion_brush_indices);
		let eb_weights = convert_to_ssbo(&gl, erosion_brush_weights);
		println!("Length: {} {}", eb_indices.2, eb_weights.2);

		ErosionGpu {
			gl,
			program,
			eb_indices,
			eb_weights,
		}
	}

	pub fn erode(&self, size: (u64, u64), texture: NativeTexture, iterations: u64) {
		const SHADER_LOCAL_SIZE: u32 = 32;
		let gl = self.gl.clone();

		unsafe {
			gl.use_program(Some(self.program));

			gl.uniform_2_i32(gl.get_uniform_location(self.program, "u_map_size").as_ref(), size.0 as i32, size.1 as i32);
			gl.uniform_2_u32(gl.get_uniform_location(self.program, "u_tile_offset").as_ref(), 0, 0);


			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(self.eb_indices.0));
			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 2, Some(self.eb_indices.1));
			gl.uniform_1_i32(gl.get_uniform_location(self.program, "erosion_brush_indices_slices_count").as_ref(), self.eb_indices.2);

			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 3, Some(self.eb_weights.0));
			gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 4, Some(self.eb_weights.1));
			gl.uniform_1_i32(gl.get_uniform_location(self.program, "erosion_brush_weights_slices_count").as_ref(), self.eb_weights.2);

			let size_x = ((size.0 as u32)/8 + SHADER_LOCAL_SIZE - 1) / SHADER_LOCAL_SIZE;
			let size_y = ((size.1 as u32)/8 + SHADER_LOCAL_SIZE - 1) / SHADER_LOCAL_SIZE;

			for i in 0..iterations {
				gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_random_seed").as_ref(), i as i32);
				gl.bind_image_texture(0, texture, 0, false, 0, glow::READ_WRITE, glow::R32I);
				gl.dispatch_compute(size_x, size_y, 1);
			}
			gl.finish();
		}
	}
}

impl Drop for ErosionGpu {
	fn drop(&mut self) {
		let gl = self.gl.clone();
		unsafe {
			gl.delete_program(self.program);
			gl.delete_buffer(self.eb_indices.0);
			gl.delete_buffer(self.eb_indices.1);
			gl.delete_buffer(self.eb_weights.0);
			gl.delete_buffer(self.eb_weights.1);
		}
	}
}