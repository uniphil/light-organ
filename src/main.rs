extern crate goertzel;
extern crate jack;
extern crate lossyq;
extern crate sdl2;

use jack::prelude as j;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::collections::VecDeque;
use std::{thread, time};
// use std::io;

// const LOWEST_NOTE_FREQ: f32 = 27.5;  // A0
const HIGHEST_SAMPLE_RATE: u32 = 96000;  // I guess?
const WINDOW_SIZE: usize = HIGHEST_SAMPLE_RATE as usize;

const BUFFSIZE: usize = 1024;

const DECAY_SAMPLES: usize = 32;

struct RGB {
    r: f32,
    g: f32,
    b: f32,
}

impl RGB {
    fn new(r: f32, g: f32, b: f32) -> RGB {
        RGB { r, g, b }
    }
}

impl From<RGB> for Color {
    fn from(rgb: RGB) -> Self {
        let f = |l: f32| { (l as u32 / 1000).min(255) as u8 };
        Color::RGB(f(rgb.r), f(rgb.g), f(rgb.b))
    }
}

const NOTE_COLOURS: [RGB; 12] = [
    RGB { r: 1.0, g: 0.0, b: 0.0 },  // F  red
    RGB { r: 1.0, g: 0.5, b: 0.0 },  // F# orange
    RGB { r: 1.0, g: 1.0, b: 0.0 },  // G  yellow
    RGB { r: 0.5, g: 1.0, b: 0.0 },  // Ab lime
    RGB { r: 0.0, g: 1.0, b: 0.0 },  // A  green
    RGB { r: 0.0, g: 1.0, b: 0.5 },  // Bb bluish green
    RGB { r: 0.0, g: 1.0, b: 1.0 },  // B  cyan
    RGB { r: 0.0, g: 0.5, b: 1.0 },  // C  boring blue
    RGB { r: 0.0, g: 0.0, b: 1.0 },  // C# blue
    RGB { r: 0.5, g: 0.0, b: 1.0 },  // D  purple
    RGB { r: 1.0, g: 0.0, b: 1.0 },  // Eb magenta
    RGB { r: 1.0, g: 0.0, b: 0.5 },  // E purpley-red
];

fn get_window_canvas() -> (sdl2::render::Canvas<sdl2::video::Window>, sdl2::EventPump) {
    let sdl_context = sdl2::init()
        .unwrap();
    let video_subsystem = sdl_context
        .video()
        .unwrap();
    let window = video_subsystem.window("colours", 800, 600)
        .resizable()
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();
    let events = sdl_context.event_pump().unwrap();
    canvas.set_draw_color(Color::RGB(255, 200, 0));
    canvas.clear();
    canvas.present();
    (canvas, events)
}

fn get_channel(client: &j::Client, name: &str) -> (JackReceiver, Computer) {
    // TODO: accept buffsize as a param?
    let (tx, rx) = lossyq::spsc::channel::<f32>(BUFFSIZE * 4);
    let receiver = JackReceiver::new(client, name, tx);
    let computer = Computer::new(rx);
    (receiver, computer)
}

struct JackReceiver {
    tx: lossyq::spsc::Sender<f32>,
    jack_in: jack::port::Port<j::AudioInSpec>,
}

impl JackReceiver {
    fn new(client: &j::Client, name: &str, tx: lossyq::spsc::Sender<f32>) -> JackReceiver {
        let jack_in = client
            .register_port(name, j::AudioInSpec::default())
            .unwrap();
        JackReceiver {
            tx,
            jack_in,
        }
    }
}

struct Computer {
    rx: lossyq::spsc::Receiver<f32>,
    decay_window: VecDeque<RGB>,
    samples_window: VecDeque<f32>,
}

impl Computer {
    fn new(rx: lossyq::spsc::Receiver<f32>) -> Computer {
        let samples_window: VecDeque<f32> = VecDeque::with_capacity(WINDOW_SIZE);
        let mut decay_window: VecDeque<RGB> = VecDeque::with_capacity(DECAY_SAMPLES);
        for _ in 0..DECAY_SAMPLES {
            decay_window.push_back(RGB::new(0., 0., 0.));
        }
        Computer {
            rx,
            decay_window,
            samples_window,
        }
    }
}

fn main() {
    // Create client
    let (client, _status) = j::Client::new("colours", j::client_options::NO_START_SERVER)
        .unwrap();

    let mut receivers: Vec<JackReceiver> = Vec::new();
    let mut computers: Vec<Computer> = Vec::new();

    for i in 0..1 {
        let (receiver, computer) = get_channel(&client, &format!("in {}", i+1));
        receivers.push(receiver);
        computers.push(computer);
    }

    let process_callback = move |_: &j::Client, ps: &j::ProcessScope| -> j::JackControl {
        // just copy stuff to a non-allocating buffer, overwriting old stuff
        for receiver in &mut receivers {
            let channel_in = j::AudioInPort::new(&receiver.jack_in, ps);
            for v in channel_in.iter() {
                receiver.tx.put(|x| *x = Some(*v));
            }
        }
        j::JackControl::Continue
    };
    let process = j::ClosureProcessHandler::new(process_callback);
    let active_client = j::AsyncClient::new(client, (), process).unwrap();

    let (mut canvas, mut events) = get_window_canvas();

    'main: loop {
        let computer = &mut computers[0];
        for sample in computer.rx.iter() {
            computer.samples_window.push_back(sample);
            if computer.samples_window.len() > WINDOW_SIZE {
                computer.samples_window.pop_front();
            }
        }

        let freq_samples = computer.samples_window
            .iter()
            .rev()
            .take(BUFFSIZE)
            .map(|x| (x * 1000.) as i16)
            .collect::<Vec<i16>>();
        let freq_mags = (1..100)
            .map(|n| 440. * 2.0_f32.powf(1./12.).powf(n as f32 - 48.))
            .filter(|f| *f > 44100. / BUFFSIZE as f32)
            .map(|f| goertzel::Parameters::new(f, 44100, BUFFSIZE as usize)
                    .start()
                    .add(&freq_samples)
                    .finish_mag());
        let mut rgb = RGB { r: 0., g: 0., b: 0. };
        for (i, mag) in freq_mags.enumerate() {
            rgb.r += mag * NOTE_COLOURS[i % 12].r;
            rgb.g += mag * NOTE_COLOURS[i % 12].g;
            rgb.b += mag * NOTE_COLOURS[i % 12].b;
        }

        let decay_window = &mut computer.decay_window;

        decay_window.pop_back();
        decay_window.push_front(rgb);

        let mut decayed_rgb = RGB::new(0., 0., 0.);
        for i in 0..DECAY_SAMPLES {
            let weight = (1. - (i as f32 / DECAY_SAMPLES as f32)).powf(2.);
            let old_rgb = &decay_window[i];
            decayed_rgb.r += old_rgb.r * weight;
            decayed_rgb.g += old_rgb.g * weight;
            decayed_rgb.b += old_rgb.b * weight;
        }

        canvas.set_draw_color(Color::from(decayed_rgb));
        canvas.clear();
        canvas.present();

        for event in events.poll_iter() {
            match event {
                Event::Quit {..} => break 'main,
                Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    break 'main
                },
                _ => {},
            }
        }
        thread::sleep(time::Duration::new(0, 1_000_000_000 / 60));
    }
    println!("bye");

    active_client.deactivate().unwrap();
}
