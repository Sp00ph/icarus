use icarus_common::piece::Color;

use crate::search::{
    history::{CORR_SIZE, MAX_CORR_VALUE, apply_gravity},
    params::{corr_bonus_div, corr_bonus_scale},
};

pub struct CorrHist {
    /// [stm][hash % CORR_SIZE]
    data: [[i16; CORR_SIZE]; 2],
}

impl CorrHist {
    pub fn amount(delta: i32, depth: i16) -> i32 {
        (delta * (depth as i32) * corr_bonus_scale()) / corr_bonus_div()
    }

    pub fn get(&self, stm: Color, hash: u64) -> i16 {
        self.data[stm][(hash % (CORR_SIZE as u64)) as usize]
    }

    fn get_mut(&mut self, stm: Color, hash: u64) -> &mut i16 {
        &mut self.data[stm][(hash % (CORR_SIZE as u64)) as usize]
    }

    pub fn update(&mut self, stm: Color, hash: u64, amount: i32) {
        apply_gravity::<{ MAX_CORR_VALUE / 4 }, MAX_CORR_VALUE>(self.get_mut(stm, hash), None, amount);
    }
}
