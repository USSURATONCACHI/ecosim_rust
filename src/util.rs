use std::time::{Duration, Instant};
use glow::{Context, Program};

#[derive(Clone, Debug)]
pub struct TickCounter {
    pub counters: Vec<(usize, f64)>,
    tps: usize,
    tps_corrected: f64,
}
impl TickCounter {
    pub fn new(count: usize) -> TickCounter {
        let mut res = TickCounter{ counters: Vec::new(), tps: 0, tps_corrected: 0.0 };
        let cur_time = current_time();
        for i in 0..count { res.counters.push( (0, cur_time + (i as f64) / (count as f64)) ); }
        res
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        let cur_time = current_time();
        let count = self.counters.len();
        for (i, (ticks, time)) in self.counters.iter_mut().enumerate() {
            *ticks = 0;
            *time = cur_time + (i as f64) / (count as f64);
        }
    }
    pub fn tick(&mut self) {
        let cur_time = current_time();
        for (f, t) in self.counters.iter_mut() {
            *f += 1;    //Просчет текущего значения
            if cur_time > *t {  //Обновление счетчиков
                self.tps = *f;
                self.tps_corrected = (*f as f64) / (cur_time - *t + 1.0);
                *f = 0;
                *t += 1.0;
            }
        }
    }

    pub fn no_tick(&mut self) {
        let cur_time = current_time();
        for (f, t) in self.counters.iter_mut() {
            if cur_time > *t {
                self.tps = *f;
                self.tps_corrected = (*f as f64) / (cur_time - *t + 1.0);
                *f = 0;
                *t += 1.0;
            }
        }
    }

    #[allow(dead_code)]
    pub fn tps(&self) -> usize { self.tps }
    pub fn tps_corrected(&self) -> f64 { self.tps_corrected }
}

pub fn current_time() -> f64 {
    (
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as f64
    ) / 1000000.0
}

pub const CAMERA_ZOOM_EXP: f64 = 1000.0;
pub const CAMERA_VEL_EXP: f64 = 128.0;

#[derive(Debug, Clone)]
pub struct Camera {
    pos: (f32, f32),
    vel: (f32, f32),
    vel_start_time: f64,

    next_vel: (f32, f32),
    last_drag_time: f64,
    pub vel_exp: f64,
    pub vel_inertia: f32,

    zoom: f32,
    zoom_vel: f32,
    zoom_vel_start_time: f64,
    pub zoom_exp: f64,
}

#[allow(dead_code)]
impl Camera {
    pub fn new(start_x: f32, start_y: f32) -> Self {
        let now = current_time();
        Camera {
            pos: (start_x, start_y),
            vel: (0.0, 0.0),
            vel_start_time: now,
            next_vel: (0.0, 0.0),
            last_drag_time: now,
            vel_exp: CAMERA_VEL_EXP,
            vel_inertia: 0.3,
            zoom: 0.0,
            zoom_vel: 0.0,
            zoom_vel_start_time: now,
            zoom_exp: CAMERA_ZOOM_EXP,
        }
    }

    pub fn pos(&self) -> (f32, f32) {
        let q: f64 = 1.0 / self.vel_exp;

        let (x, y) = self.pos;
        let (vx, vy) = self.vel;
        let t = current_time() - self.vel_start_time;

        let coef = ((1.0 - q.powf(t)) / (1.0 - q)) as f32;
        let new_x = x + self.vel_inertia * vx * coef;
        let new_y = y + self.vel_inertia * vy * coef;
        (new_x, new_y)
    }

    pub fn vel(&self) -> (f32, f32) {
        let q: f64 = 1.0 / self.vel_exp;
        let t = current_time() - self.zoom_vel_start_time;
        let coef = q.powf(t) as f32;
        (self.vel_inertia * self.vel.0 * coef, self.vel_inertia * self.vel.1 * coef)
    }

    pub fn set_pos(&mut self, new_pos: (f32, f32)) {
        let new_vel = self.vel();
        self.pos = new_pos;
        self.vel = new_vel;
        self.vel_start_time = current_time();
    }

    pub fn wrap_x(&mut self, max: f32) {
        let (x, _y) = self.pos();
        let new_x = ((x % max) + max) % max;
        let dx = new_x - x;
        self.pos.0 += dx;
    }

    pub fn wrap_y(&mut self, max: f32) {
        let (_x, y) = self.pos();
        let new_y = ((y % max) + max) % max;
        let dy = new_y - y;
        self.pos.1 += dy;
    }

    pub fn update_anim(&mut self) {
        let new_pos = self.pos();
        let new_vel = self.vel();
        let new_zoom_vel = self.zoom_vel();
        let now = current_time();

        self.pos = new_pos;
        self.vel = new_vel;
        self.zoom_vel = new_zoom_vel;
        self.vel_start_time = now;
        self.zoom_vel_start_time = now;
    }

    pub fn zoom(&self) -> f32 {
        let q = 1.0 / self.zoom_exp;
        let t = current_time() - self.zoom_vel_start_time;
        self.zoom + self.zoom_vel * (((1.0 - q.powf(t)) / (1.0 - q)) as f32)
    }

    pub fn zoom_vel(&self) -> f32 {
        let q = 1.0 / self.zoom_exp;
        let t = current_time() - self.zoom_vel_start_time;
        self.zoom_vel * (q.powf(t) as f32)
    }

    pub fn set_zoom(&mut self, new_zoom: f32) {
        self.zoom = new_zoom;
        self.zoom_vel = 0.0;
        self.zoom_vel_start_time = current_time();
    }

    pub fn on_zoom(&mut self, delta: f32) {
        self.zoom = self.zoom();
        self.zoom_vel = self.zoom_vel() + delta;
        self.zoom_vel_start_time = current_time();
    }

    pub fn on_drag(&mut self, drag: (f32, f32)) {
        self.pos.0 += drag.0;
        self.pos.1 += drag.1;

        let now = current_time();
        let dt = (now - self.last_drag_time) as f32;
        self.last_drag_time = now;
        let this_drag_vel = (drag.0 / dt, drag.1 / dt);
        self.next_vel.0 = self.next_vel.0 * 0.2 + this_drag_vel.0 * 0.8;
        self.next_vel.1 = self.next_vel.1 * 0.2 + this_drag_vel.1 * 0.8;
    }

    pub fn on_drag_start(&mut self) {
        self.pos = self.pos();
        self.vel = (0.0, 0.0);
        self.vel_start_time = current_time();
    }

    pub fn on_drag_end(&mut self) {
        self.pos = self.pos();
        self.vel = self.next_vel;
        self.next_vel = (0.0, 0.0);
        self.vel_start_time = current_time();
    }
}


pub struct RateManager {
    pack_size: u32,
    target_tick_rate: u32,
    current_tick: u32,
    pack_start: Instant,
}

impl RateManager {
    pub fn new(pack_size: u32, target_tick_rate: u32) -> Self {
        RateManager {
            pack_size,
            target_tick_rate,
            current_tick: 0,
            pack_start: Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.current_tick = 0;
        self.pack_start = Instant::now();
    }

    pub fn set_tick_rate(&mut self, rate: u32) {
        self.target_tick_rate = rate;
        self.reset();
    }

    pub fn tick_rate(&self) -> u32 {
        self.target_tick_rate
    }

    pub fn register_tick(&mut self) {
        self.current_tick += 1;

        if self.current_tick >= self.pack_size {
            self.current_tick -= self.pack_size;
            self.pack_start = Instant::now();
        }
    }

    pub fn next_tick_time(&mut self) -> Instant {
        self.pack_start + Duration::from_secs_f64(((self.current_tick + 1) as f64) / (self.target_tick_rate as f64))
    }

    pub fn ticks_to_do_by_time(&mut self, time: Instant) -> u32 {
        let total = (time - self.pack_start).as_secs_f64() * (self.target_tick_rate as f64);
        let total = total as u32;

        if total >= self.current_tick {
            total - self.current_tick
        } else {
            0
        }
    }
}


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
                    Err(format!("Cannot compile shader: {}", gl.get_shader_info_log(shader)))
                } else {
                    gl.attach_shader(program, shader);
                    Ok(shader)
                }
            })
            .collect();

        let mut checked_shaders = vec![];

        for result in shaders {
            checked_shaders.push(result?);
        }

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!("Cannot link program: {}", gl.get_program_info_log(program)));
        }

        for shader in checked_shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        Ok(program)
    }
}