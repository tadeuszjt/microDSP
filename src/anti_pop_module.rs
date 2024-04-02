use crate::sample::{Sample};

pub struct AntiPop<const NUM_CHANNELS : usize> {
    buffer : [Sample<NUM_CHANNELS>; 10],
}


impl <const NUM_CHANNELS: usize> AntiPop<NUM_CHANNELS> {
    pub fn new() -> Self {
        AntiPop {
            buffer : [Sample::new(); 10],
        }
    }
}

