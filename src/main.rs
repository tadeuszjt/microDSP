mod counting_mpsc;
mod fir_module;
mod throttle_module;
use fir_module::FirFilter;

use thread_priority::*;

use std::fs;
use std::io;
use std::str::FromStr;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::{cursor, execute, terminal};

const SAMPLE_RATE: u32 = 48000;
const NUM_INPUT_CHANNELS: usize = 2;
const NUM_OUTPUT_CHANNELS: usize = 6;
const NUM_FIR_CHANNELS: usize = 8;
const FIR_IMPULSE_LEN: usize = 2046;
const VOLUME_SCALAR: f32 = 0.5;

const INPUT_CHANNEL_MAPPING: [usize; NUM_FIR_CHANNELS] = [1, 1, 1, 0, 0, 0, 0, 0];

type FirSample = [f32; NUM_FIR_CHANNELS];

enum RunResult {
    Quit,
    Reload,
    StreamsStopped,
}

fn main() {
    execute!(io::stdout(), terminal::EnterAlternateScreen).unwrap();

    // setup stdin scanner thread
    let (stdin_sender, stdin_receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in std::io::stdin().lines() {
            stdin_sender.send(line.unwrap()).unwrap();
        }
    });

    loop {
        match run(&stdin_receiver) {
            RunResult::Quit => {
                break;
            }
            RunResult::Reload => {}
            RunResult::StreamsStopped => {
                println!("restarting...");
            }
        }
    }

    execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
    println!("exit");
}

fn run_fir_thread() -> (
    counting_mpsc::Sender<FirSample>,
    counting_mpsc::Receiver<FirSample>,
) {
    let impulses: Vec<Vec<f32>> = vec![
        load_fir_impulse("impulse_tweeter_6_3_24.txt"),
        load_fir_impulse("impulse_woofer_6_3_24.txt"),
        Vec::new(),
        Vec::new(),
        load_fir_impulse("impulse_tweeter_6_3_24.txt"),
        load_fir_impulse("impulse_woofer_6_3_24.txt"),
    ];

    let (fir_input_tx, mut fir_input_rx) = counting_mpsc::channel::<FirSample>();
    let (throttle_input_tx, throttle_output_rx) = throttle_module::new::<NUM_FIR_CHANNELS>();

    thread_priority::spawn(ThreadPriority::Max, move |_| {
        let mut fir = FirFilter::<NUM_FIR_CHANNELS>::new(impulses);
        loop {
            let sample = match fir_input_rx.recv() {
                Err(_) => {
                    break;
                } // channel closed
                Ok(sample) => sample,
            };

            fir.push_sample(sample);
            match fir.pop_sample() {
                None => {}
                Some(sample) => {
                    throttle_input_tx.send(sample).unwrap();
                }
            }
        }
    });

    (fir_input_tx, throttle_output_rx)
}

fn run(stdin_receiver: &mpsc::Receiver<String>) -> RunResult {
    let input_config = cpal::StreamConfig {
        channels: NUM_INPUT_CHANNELS.try_into().unwrap(),
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let output_config = cpal::StreamConfig {
        channels: NUM_OUTPUT_CHANNELS.try_into().unwrap(),
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let host = cpal::default_host();

    let input_device_name = "Hi-Fi Cable Output (VB-Audio Hi-Fi Cable)";
    let output_device_name = "Speakers (Sound Blaster Audigy Fx V2)";

    let input_device = host
        .input_devices()
        .unwrap()
        .find(|dev| dev.name().unwrap() == input_device_name)
        .expect("Input device not found");

    let output_device = host
        .output_devices()
        .unwrap()
        .find(|dev| dev.name().unwrap() == output_device_name)
        .expect("Output device not found");

    let shared_data = Arc::new(Mutex::new(SharedData::new()));
    let shared_data_input = Arc::clone(&shared_data);
    let shared_data_output = Arc::clone(&shared_data);

    let (mut fir_input_tx, mut fir_output_rx) = run_fir_thread();
    let fir_output_count = fir_output_rx.clone_count();

    let input_stream = input_device
        .build_input_stream(
            &input_config,
            move |data, _: &_| process_input_data(&data, &shared_data_input, &mut fir_input_tx),
            |err| eprintln!("Error in input stream: {}", err),
            None,
        )
        .expect("Couldn't build input stream.");

    let output_stream = output_device
        .build_output_stream(
            &output_config,
            move |data: &mut _, _: &_| {
                process_output_data(data, &shared_data_output, &mut fir_output_rx)
            },
            |err| eprintln!("Error in output stream: {}", err),
            None,
        )
        .expect("Couldn't build output stream.");

    output_stream.play().unwrap();
    input_stream.play().unwrap();

    let mut result = RunResult::Quit;
    let mut running = true;
    while running {
        {
            let shared_data = shared_data.lock().expect("Failed to lock shared data");
            let time = Instant::now();

            let input_delta = time
                .duration_since(shared_data.input_buffer_timestamp)
                .as_secs_f64();

            let output_delta = time
                .duration_since(shared_data.output_buffer_timestamp)
                .as_secs_f64();

            if output_delta > 1.0 || input_delta > 1.0 {
                println!("streams stopped");
                return RunResult::StreamsStopped;
            }

            print_interface(&shared_data, fir_output_count.get_count());
        }

        loop {
            // read std::in channel
            match stdin_receiver.try_recv() {
                Err(mpsc::TryRecvError::Empty) => {
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    assert!(false, "stdio disconnected");
                }
                Ok(s) if s == "q" => {
                    running = false;
                    result = RunResult::Quit;
                    break;
                }
                Ok(s) if s == "r" => {
                    running = false;
                    result = RunResult::Reload;
                    break;
                }
                Ok(_) => {
                    println!("invalid cmd");
                }
            }
        }

        if running {
            thread::sleep(Duration::from_millis(500));
        }
    }

    return result;
}

struct SharedData {
    input_buffer_timestamp: Instant,
    output_buffer_timestamp: Instant,
    output_buffer_volumes: [f32; NUM_OUTPUT_CHANNELS],
    missed_sample_count: usize,
}

impl SharedData {
    fn new() -> Self {
        SharedData {
            input_buffer_timestamp: Instant::now(),
            output_buffer_timestamp: Instant::now(),
            output_buffer_volumes: [0.0; NUM_OUTPUT_CHANNELS],
            missed_sample_count: 0,
        }
    }
}

fn process_input_data(
    data: &[f32],
    shared_data: &Arc<Mutex<SharedData>>,
    sender: &mut counting_mpsc::Sender<FirSample>,
) {
    {
        (*shared_data.lock().expect("Failed to lock shared data")).input_buffer_timestamp =
            Instant::now();
    }

    let num_input_frames = data.len() / NUM_INPUT_CHANNELS;

    for frame in 0..num_input_frames {
        let mut arr: FirSample = [0.0; NUM_FIR_CHANNELS];
        for channel in 0..NUM_FIR_CHANNELS {
            let input_channel = INPUT_CHANNEL_MAPPING[channel];
            arr[channel] = data[frame * NUM_INPUT_CHANNELS + input_channel];
        }

        sender.send(arr).unwrap();
    }
}

fn process_output_data(
    data: &mut [f32],
    shared_data: &Arc<Mutex<SharedData>>,
    receiver: &mut counting_mpsc::Receiver<FirSample>,
) {
    let mut shared_data = shared_data.lock().expect("Failed to lock shared data");
    let num_output_frames = data.len() / NUM_OUTPUT_CHANNELS;
    shared_data.output_buffer_timestamp = Instant::now();

    for frame in 0..num_output_frames {
        let sample = match receiver.try_recv() {
            Ok(sample) => sample,
            Err(mpsc::TryRecvError::Empty) => {
                shared_data.missed_sample_count += 1;
                [0.0; NUM_FIR_CHANNELS]
            }
            Err(_) => {
                assert!(false);
                [0.0; NUM_FIR_CHANNELS]
            }
        };

        for channel in 0..NUM_OUTPUT_CHANNELS {
            data[frame * NUM_OUTPUT_CHANNELS + channel] = sample[channel] * VOLUME_SCALAR;
        }
    }

    // record channel volumes
    for channel in 0..NUM_OUTPUT_CHANNELS {
        let mut sum = 0.0;
        for frame in 0..num_output_frames {
            let sample = data[frame * NUM_OUTPUT_CHANNELS + channel];
            sum += sample * sample;
        }

        shared_data.output_buffer_volumes[channel] = (sum / num_output_frames as f32).sqrt();
    }
}

fn load_fir_impulse(filename: &str) -> Vec<f32> {
    let impulse_str = fs::read_to_string(filename).unwrap();

    let impulse: Vec<f32> = impulse_str
        .lines()
        .filter_map(|line| f32::from_str(line).ok())
        .collect();

    //println!("Impulse: {:?}", impulse);

    assert!(
        impulse.len() == FIR_IMPULSE_LEN,
        "invalid impulse length ({}) for: {}",
        impulse.len(),
        filename
    );
    return impulse;
}

fn print_interface(shared_data: &SharedData, buffer_count: usize) {
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
    execute!(io::stdout(), cursor::MoveTo(0, 0)).unwrap();
    println!("===== Tadeusz's FIR =====");
    println!("q - quit, r - reload");

    for channel in 0..NUM_OUTPUT_CHANNELS {
        println!(
            "channel {} volume: {:.4}",
            channel, shared_data.output_buffer_volumes[channel],
        );
    }

    println!(
        "buffer frames: {}\nmissed samples: {}\nload: {:.1}%",
        buffer_count,
        shared_data.missed_sample_count,
        0.0, // shared_data.fir.load_percentage(),
    );
}
