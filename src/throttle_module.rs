use crate::counting_mpsc;
use std::sync::mpsc;
pub struct ThrottleSender<T: std::marker::Send + 'static> {
    iter_count: u64,
    output_mpsc_tx: counting_mpsc::Sender<T>,
    pid_i : f64,
    pid_prev_error : f64,
}

const ONE_MILLION : u64 = 1000000;
const BUFFER_LEN_TARGET : usize = 600;

const PID_KP : f64 = 0.001;
const PID_KI : f64 = 0.0001;
const PID_KD : f64 = 0.0001;
const PID_TD : f64 = 1.0 / 48000.0;


impl<T: std::marker::Send + 'static> ThrottleSender<T> {
    pub fn send(&mut self, item: T) -> Result<(), mpsc::SendError<T>> {
        let output_len = self.output_mpsc_tx.get_count();


        // PID loop
        let error = ((output_len as i64) - (BUFFER_LEN_TARGET as i64)) as f64;
        self.pid_i += error * PID_TD;
        let pid_d = error - self.pid_prev_error;
        self.pid_prev_error = error;
        let pid_output = PID_KP * error + PID_KI * self.pid_i + PID_KD * pid_d;

        let mut keep_sample = true;
        if pid_output > 0.0 {
            if self.iter_count % (ONE_MILLION / ((pid_output * 50.0) as u64 + 1)) == 0 {
                keep_sample = false;
            }
        }

        let mut result = Ok(());
        if keep_sample {
            result = self.output_mpsc_tx.send(item);
        }

        if self.iter_count % 100000 == 0 {
            println!("pid:{}", pid_output);
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
            output_mpsc_tx: output_mpsc_tx,
            pid_i : 0.0,
            pid_prev_error: 0.0,
        },
        output_mpsc_rx,
    )
}
