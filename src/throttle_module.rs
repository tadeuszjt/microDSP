use crate::counting_mpsc;
use std::sync::mpsc;
use thread_priority;

pub fn new<const NUM_CHANNELS: usize>() -> (
    mpsc::Sender<[f32; NUM_CHANNELS]>,
    counting_mpsc::Receiver<[f32; NUM_CHANNELS]>,
) {
    let (input_mpsc_tx, input_mpsc_rx) = mpsc::channel();
    let (mut output_mpsc_tx, output_mpsc_rx) = counting_mpsc::channel();

    let mut min_output_len = 0;
    let output_len_target = 500;

    thread_priority::spawn(thread_priority::ThreadPriority::Max, move |_| {
        for iter_count in 0.. {
            let sample = match input_mpsc_rx.recv() {
                Err(_) => {
                    break;
                }
                Ok(sample) => sample,
            };

            // control min_output_len
            let output_len = output_mpsc_tx.get_count();
            if output_len < min_output_len {
                min_output_len = output_len;
            }
            if iter_count % 1000 == 0 {
                min_output_len += 1;
            }

            // drop sample
            let mut keep_sample = true;
            if (min_output_len > output_len_target) && (iter_count % 5000 == 0) {
                keep_sample = false;
            }

            if keep_sample {
                output_mpsc_tx.send(sample).unwrap();
            }
        }
    });

    (input_mpsc_tx, output_mpsc_rx)
}
