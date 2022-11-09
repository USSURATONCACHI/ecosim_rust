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

pub fn current_time_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}