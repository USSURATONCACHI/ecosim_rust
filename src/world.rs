use rand::Rng;
use crate::util::TickCounter;

#[derive(Clone, Debug)]
pub struct Cell {
	pub state: bool,
}

#[derive(Clone, Debug)]
pub struct World {
	cells_data: Box<[Cell]>,
	tmp_cells: Box<[Cell]>,

	tps: TickCounter,
	size_x: usize,
	size_y: usize,
	tick: u64,
}

impl World {
	pub fn new(size: (usize, usize)) -> Self {
		let arr_size = size.0 * size.1;
		let mut rng = rand::thread_rng();
		World {
			size_x: size.0,
			size_y: size.1,
			tick: 0,
			cells_data: (0..arr_size)
				.map(|i| {
					// let x = i % size.0;
					// let y = i / size.0;
					Cell { state: rng.gen_bool(0.1) }
				})
				.collect(),

			tmp_cells: [Cell { state: false }].into_iter()
				.cycle().take(arr_size).collect(),
			tps: TickCounter::new(30),
		}
	}

	pub fn cells_data(&self) -> &[Cell] {
		&self.cells_data
	}

	pub fn size(&self) -> (usize, usize) {
		(self.size_x, self.size_y)
	}

	pub fn cur_tick(&self) -> u64 {
		self.tick
	}

	pub fn tps(&self) -> &TickCounter {
		&self.tps
	}

	pub fn update(&mut self) {
		let sizex = self.size_x as i64;
		let sizey = self.size_y as i64;
		for x in 0..sizex {
			for y in 0..sizey {
				let mut neighbours = 0;

				for dx in -1..=1 {
					for dy in -1..=1 {
						let x = x + dx;
						let y = y + dy;
						if x >= 0 && y >= 0 && x < sizex && y < sizey &&
							!(dx == 0 && dy == 0) {
							if self.cell(x as usize, y as usize).state {
								neighbours += 1;
							}
						}
					}
				}

				let cell_state = self.cell(x as usize, y as usize).state;

				let new_state = match cell_state {
					true => neighbours == 2 || neighbours == 3,
					false => neighbours == 3,
				};

				*self.tmp_cell(x as usize, y as usize) = Cell { state: new_state };
			}
		}

		std::mem::swap(&mut self.tmp_cells, &mut self.cells_data);
		self.tps.tick();
		self.tick += 1;
	}

	pub fn cell(&self, x: usize, y: usize) -> &Cell {
		&self.cells_data[y * self.size_x + x]
	}

	pub fn tmp_cell(&mut self, x: usize, y: usize) -> &mut Cell {
		&mut self.tmp_cells[y * self.size_x + x]
	}

	pub fn no_tick(&mut self) {
		self.tps.no_tick();
	}
}