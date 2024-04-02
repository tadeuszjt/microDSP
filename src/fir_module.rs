use crate::sample::Sample;
use std::collections::VecDeque;
use std::time::Instant;

pub struct FirFilter<const NUM_CHANNELS: usize> {
    zero_sample_count : usize,
    impulse: Vec<[f32; NUM_CHANNELS]>,
    buffer: VecDeque<[f32; NUM_CHANNELS]>,
    timestamps: VecDeque<Instant>,
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
            timestamps: VecDeque::new(),
            zero_sample_count: 0,
        };

        if impulses.len() > 0 {
            let impulse_len = impulses[0].len();
            filter.impulse = vec![[0.0; NUM_CHANNELS]; impulse_len];

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

    pub fn pop_sample(&mut self) -> Option<Sample<NUM_CHANNELS>> {
        if self.buffer.len() < self.impulse.len() {
            return None;
        }

        let mut arr = [0.0; NUM_CHANNELS];
        if self.zero_sample_count < self.buffer.len() {
            for i in 0..self.impulse.len() {
                for j in 0..NUM_CHANNELS {
                    arr[j] += self.impulse[i][j] * self.buffer[i][j];
                }
            }
        }

        let timestamp = self.timestamps[0];
        self.buffer.pop_front();
        self.timestamps.pop_front();

        return Some(Sample {
            data: arr,
            timestamp: timestamp,
        });
    }

    pub fn push_sample(&mut self, sample: Sample<NUM_CHANNELS>) {
        if sample.data == [0.0; NUM_CHANNELS] {
            self.zero_sample_count += 1;
        } else {
            self.zero_sample_count = 0;
        }

        self.buffer.push_back(sample.data);
        self.timestamps.push_back(sample.timestamp);
    }

    //    pub fn buffer_len(&self) -> usize {
    //        return self.buffer.len();
    //    }
}
