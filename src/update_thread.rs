
use std::sync::mpsc::Receiver;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use crate::util::current_time_nanos;
use crate::World;

pub const CHANNEL_CHECK_DELAY: u128 = 1_000_000_000 / 60;	// ns
pub const IDLE_CHECKS_DELAY: u64 = 1_000_000_000 / 10; // ns
pub const UPS_LIM_RESET_FREQ: u64 = 60; // each X ticks

#[derive(Debug, Clone)]
pub enum Message {
	RunSimulation(bool),
	LimitUPS(Option<u32>),
	Stop,
}

pub struct UpdThread {
	world: Box<World>,
	run_simulation: bool,
	ups_limit: Option<u32>,

	recv: Receiver<Message>,
}

impl UpdThread {
	pub fn new(recv: Receiver<Message>, world: Box<World>) -> Self {
		UpdThread {
			world,
			run_simulation: false,
			ups_limit: None,
			recv
		}
	}

	pub fn run(mut self) -> JoinHandle<()> {
		thread::spawn(move || {
			let mut last_channel_check = current_time_nanos();
			let mut cycle: u64 = 0;


			let mut cycles_pack_start_time = last_channel_check;
			let mut cycles_pack_start_cycle = 0_u64;

			'run: loop {
				let now = current_time_nanos();

				if now - last_channel_check >= CHANNEL_CHECK_DELAY {
					last_channel_check = now;

					for msg in self.recv.try_iter() {
						match msg {
							Message::RunSimulation(flag) => {
								if flag {
									cycles_pack_start_time = now;
									cycles_pack_start_cycle = cycle;
								}
								self.run_simulation = flag;
							},
							Message::LimitUPS(limit) => {
								if limit.is_some() {
									cycles_pack_start_time = now;
									cycles_pack_start_cycle = cycle;
								}
								self.ups_limit = limit;
							},
							Message::Stop => break 'run,
						}
					}
				}
				if !self.run_simulation {
					self.world.no_tick();
					thread::sleep(Duration::from_nanos(IDLE_CHECKS_DELAY));
				} else  {
					self.world.update();
					cycle += 1;

					if self.ups_limit.is_some() {
						if cycle % UPS_LIM_RESET_FREQ == 0 {
							cycles_pack_start_time = now;
							cycles_pack_start_cycle = cycle;
						}
						let next_cycle_start = cycles_pack_start_time + ((cycle - cycles_pack_start_cycle) as u128) * 1_000_000_000 / (self.ups_limit.unwrap() as u128);
						let now = current_time_nanos();
						if now < next_cycle_start {
							let sleep_ns = next_cycle_start - now;
							thread::sleep(Duration::from_nanos(sleep_ns as u64))
						}
					}
 				}
			}
		})
	}
}