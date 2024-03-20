use std::collections::VecDeque;

pub struct FirFilter<const NUM_CHANNELS: usize> {
    impulse: Vec<[f32; NUM_CHANNELS]>,
    buffer: VecDeque<[f32; NUM_CHANNELS]>,
}

impl<const NUM_CHANNELS: usize> FirFilter<NUM_CHANNELS> {
    pub fn new(impulses: Vec<Vec<f32>>) -> FirFilter<NUM_CHANNELS> {
        // make sure it is a block of vectors which all have the same length
        assert!(impulses.len() < NUM_CHANNELS, "too many impulse vectors");
        for i in 0..impulses.len() {
            assert!(
                impulses[i].len() == impulses[0].len() || impulses[i].len() == 0,
                "impulse length mismatch"
            );
        }

        let mut filter = FirFilter {
            impulse: Vec::new(),
            buffer: VecDeque::new(),
        };

        if impulses.len() > 0 {
            let impulse_len = impulses[0].len();

            filter.impulse = vec![[0.0; NUM_CHANNELS]; impulse_len];
            filter.buffer = vec![[0.0; NUM_CHANNELS]; impulse_len].into();

            for i in 0..impulses.len() {
                if impulses[i].len() > 0 {
                    for j in 0..impulse_len {
                        filter.impulse[j][i] = impulses[i][j];
                    }
                }
            }
        }

        return filter;
    }

    pub fn pop_sample(&mut self) -> Option<[f32; NUM_CHANNELS]> {
        if self.buffer.len() < self.impulse.len() {
            return None;
        }

        let mut arr = [0.0; NUM_CHANNELS];
        for i in 0..self.impulse.len() {
            for j in 0..NUM_CHANNELS {
                arr[j] += self.impulse[i][j] * self.buffer[i][j];
            }
        }

        self.buffer.pop_front();
        return Some(arr);
    }

    pub fn push_sample(&mut self, sample: [f32; NUM_CHANNELS]) {
        self.buffer.push_back(sample);
    }

    pub fn buffer_len(&self) -> usize {
        return self.buffer.len();
    }
}
