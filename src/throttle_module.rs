use crate::counting_mpsc;
use std::sync::mpsc;
pub struct ThrottleSender<T: std::marker::Send + 'static> {
    iter_count: u64,
    min_output_len: usize,
    output_mpsc_tx: counting_mpsc::Sender<T>,
}

impl<T: std::marker::Send + 'static> ThrottleSender<T> {
    pub fn send(&mut self, item: T) -> Result<(), mpsc::SendError<T>> {
        let output_len_target = 500;

        // control min_output_len
        let output_len = self.output_mpsc_tx.get_count();
        if output_len < self.min_output_len {
            self.min_output_len = output_len;
        }
        if self.iter_count % 1000 == 0 {
            self.min_output_len += 1;
        }

        // drop sample
        let mut keep_sample = true;
        if (self.min_output_len > output_len_target) && (self.iter_count % 5000 == 0) {
            keep_sample = false;
        }

        let mut result = Ok(());
        if keep_sample {
            result = self.output_mpsc_tx.send(item);
        }

        self.iter_count += 1;

        return result;
    }
}

pub fn channel<T: std::marker::Send + 'static>() -> (ThrottleSender<T>, counting_mpsc::Receiver<T>)
{
    let (output_mpsc_tx, output_mpsc_rx) = counting_mpsc::channel();

    (
        ThrottleSender {
            iter_count: 0,
            min_output_len: 0,
            output_mpsc_tx: output_mpsc_tx,
        },
        output_mpsc_rx,
    )
}
