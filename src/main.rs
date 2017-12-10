extern crate goertzel;
extern crate jack;
extern crate lossyq;
extern crate sdl2;

use jack::prelude as j;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::collections::VecDeque;
use std::{env, thread, time};
// use std::io;

// const LOWEST_NOTE_FREQ: f32 = 27.5;  // A0
const HIGHEST_SAMPLE_RATE: u32 = 96000;  // I guess?
const WINDOW_SIZE: usize = HIGHEST_SAMPLE_RATE as usize;

const BUFFSIZE: usize = 1024;

const DECAY_SAMPLES: usize = 32;

#[derive(Debug)]
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
        let f = |l: f32| { (l as u32 / 2000).min(255) as u8 };
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
    RGB { r: 1.0, g: 0.0, b: 0.5 },  // E  purpley-red
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
    canvas.set_draw_color(Color::RGB(0, 0, 0));
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

    fn update(&mut self) {
        for sample in self.rx.iter() {
            self.samples_window.push_back(sample);
            if self.samples_window.len() > WINDOW_SIZE {
                self.samples_window.pop_front();
            }
        }
        let freq_samples = self.samples_window
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
        self.decay_window.pop_back();
        self.decay_window.push_front(rgb);
    }

    fn get_colour(&self) -> RGB {
        let mut decayed_rgb = RGB::new(0., 0., 0.);
        let mut total_weight = 1.;
        for i in 0..DECAY_SAMPLES {
            let weight = (1. - (i as f32 / DECAY_SAMPLES as f32)).powf(2.);
            total_weight += weight;
            let old_rgb = &self.decay_window[i];
            decayed_rgb.r += old_rgb.r * weight;
            decayed_rgb.g += old_rgb.g * weight;
            decayed_rgb.b += old_rgb.b * weight;
        }
        decayed_rgb.r /= total_weight;
        decayed_rgb.g /= total_weight;
        decayed_rgb.b /= total_weight;
        decayed_rgb
    }
}

fn main() {
    let channels = env::args()
        .nth(1)
        .unwrap()
        .parse::<u8>()
        .unwrap();

    let (client, _status) = j::Client::new("colours", j::client_options::NO_START_SERVER)
        .unwrap();

    let mut receivers: Vec<JackReceiver> = Vec::new();
    let mut computers: Vec<Computer> = Vec::new();

    for i in 0..channels {
        let (receiver, computer) = get_channel(&client, &format!("in_{}", i+1));
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
        for computer in &mut computers {
            computer.update();
        }
        let colours = computers
            .iter()
            .map(Computer::get_colour)
            .map(Color::from);

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        let (w, h) = canvas.window().size();
        let rects = computers.len() as u32;
        let rect_width = w / rects;

        for (i, colour) in colours.enumerate() {
            canvas.set_draw_color(colour);
            canvas
                .fill_rect(Rect::new(i as i32 * rect_width as i32, 0, rect_width, h))
                .unwrap();
        }
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
