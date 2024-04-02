use std::time::Instant;

pub struct Sample<const NUM_CHANNELS: usize> {
    pub timestamp: Instant,
    pub data: [f32; NUM_CHANNELS],
}


impl <const NUM_CHANNELS: usize> Sample<NUM_CHANNELS> {
    pub fn new() -> Self {
        Sample {
            timestamp: Instant::now(),
            data: [0.0; NUM_CHANNELS],
        }
    }
}


impl <const NUM_CHANNELS: usize> Copy for Sample<NUM_CHANNELS> {}

impl <const NUM_CHANNELS: usize> Clone for Sample<NUM_CHANNELS> {
    fn clone(&self) -> Self {
        Sample {
            timestamp: self.timestamp,
            data: self.data,
        }
    }
}
