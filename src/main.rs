extern crate goertzel;
extern crate jack;
extern crate lossyq;
extern crate sdl2;

mod hi;

use jack::prelude as j;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::collections::VecDeque;
use std::{env, thread, time};
use hi::Glt;

// const LOWEST_NOTE_FREQ: f32 = 27.5;  // A0
const HIGHEST_SAMPLE_RATE: u32 = 96000;  // I guess?
const WINDOW_SIZE: usize = HIGHEST_SAMPLE_RATE as usize;

const BUFFSIZE: usize = 1024;

const DECAY_SAMPLES: usize = 32;

#[derive(Debug)]
struct RGB {
    r: f64,
    g: f64,
    b: f64,
}

impl RGB {
    fn new(r: f64, g: f64, b: f64) -> Self {
        RGB { r, g, b }
    }
}

impl From<RGB> for Color {
    fn from(rgb: RGB) -> Self {
        let f = |l: f64| l.min(255.) as u8;
        Color::RGB(f(rgb.r), f(rgb.g), f(rgb.b))
    }
}

// // simple, rooted at red for key of F
// const NOTE_COLOURS: [RGB; 12] = [
//     RGB { r: 1.0, g: 0.0, b: 0.0 },  // F  red
//     RGB { r: 1.0, g: 0.5, b: 0.0 },  // F# orange
//     RGB { r: 1.0, g: 1.0, b: 0.0 },  // G  yellow
//     RGB { r: 0.5, g: 1.0, b: 0.0 },  // Ab lime
//     RGB { r: 0.0, g: 1.0, b: 0.0 },  // A  green
//     RGB { r: 0.0, g: 1.0, b: 0.5 },  // Bb bluish green
//     RGB { r: 0.0, g: 1.0, b: 1.0 },  // B  cyan
//     RGB { r: 0.0, g: 0.5, b: 1.0 },  // C  boring blue
//     RGB { r: 0.0, g: 0.0, b: 1.0 },  // C# blue
//     RGB { r: 0.5, g: 0.0, b: 1.0 },  // D  purple
//     RGB { r: 1.0, g: 0.0, b: 1.0 },  // Eb magenta
//     RGB { r: 1.0, g: 0.0, b: 0.5 },  // E  purpley-red
// ];

// 16
const NOTE_COLOURS: [RGB; 16] = [
    RGB { r: 1.0,   g: 0.0,   b: 0.0   },
    RGB { r: 1.0,   g: 0.376, b: 0.0   },
    RGB { r: 1.0,   g: 0.749, b: 0.0   },
    RGB { r: 0.875, g: 1.0,   b: 0.0   },
    RGB { r: 0.502, g: 1.0,   b: 0.0   },
    RGB { r: 0.125, g: 1.0,   b: 0.0   },
    RGB { r: 0.0,   g: 1.0,   b: 0.251 },
    RGB { r: 0.0,   g: 1.0,   b: 0.624 },
    RGB { r: 0.0,   g: 1.0,   b: 1.0   },
    RGB { r: 0.0,   g: 0.624, b: 1.0   },
    RGB { r: 0.0,   g: 0.251, b: 1.0   },
    RGB { r: 0.125, g: 0.0,   b: 1.0   },
    RGB { r: 0.502, g: 0.0,   b: 1.0   },
    RGB { r: 0.875, g: 0.0,   b: 1.0   },
    RGB { r: 1.0,   g: 0.0,   b: 0.749 },
    RGB { r: 1.0,   g: 0.0,   b: 0.376 },
];

// // circle of fifths for key of C
// const NOTE_COLOURS: [RGB; 12] = [
//     RGB { r: 1.0, g: 0.0, b: 0.0 },  // F  red
//     RGB { r: 0.0, g: 0.5, b: 1.0 },  // F# boring blue
//     RGB { r: 1.0, g: 1.0, b: 0.0 },  // G  yellow
//     RGB { r: 0.5, g: 0.0, b: 1.0 },  // Ab purple
//     RGB { r: 0.0, g: 1.0, b: 0.0 },  // A  green
//     RGB { r: 1.0, g: 0.0, b: 0.5 },  // Bb purpley-red
//     RGB { r: 0.0, g: 1.0, b: 1.0 },  // B  cyan
//     RGB { r: 1.0, g: 0.5, b: 0.0 },  // C  orange
//     RGB { r: 0.0, g: 0.0, b: 1.0 },  // C# blue
//     RGB { r: 0.5, g: 1.0, b: 0.0 },  // D  lime
//     RGB { r: 1.0, g: 0.0, b: 1.0 },  // Eb magenta
//     RGB { r: 0.0, g: 1.0, b: 0.5 },  // E  bluish green
// ];

// // circle of fifths for key of C
// const NOTE_COLOURS: [RGB; 12] = [
//     RGB { r: 1.0, g: 0.0, b: 0.0 },  // F  red
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // F# (off) boring blue
//     RGB { r: 1.0, g: 1.0, b: 0.0 },  // G  yellow
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // Ab (off) purple
//     RGB { r: 0.0, g: 1.0, b: 0.0 },  // A  green
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // Bb (off) purpley-red
//     RGB { r: 0.0, g: 1.0, b: 1.0 },  // B  cyan
//     RGB { r: 1.0, g: 0.5, b: 0.0 },  // C  orange
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // C# (off) blue
//     RGB { r: 0.5, g: 1.0, b: 0.0 },  // D  lime
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // Eb (off) magenta
//     RGB { r: 0.0, g: 1.0, b: 0.5 },  // E  bluish green
// ];

// // whole tone near-complements
// const NOTE_COLOURS: [RGB; 12] = [
//     RGB { r: 1.0, g: 0.0, b: 0.0 },  // F  red
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // F# (off)
//     RGB { r: 0.0, g: 0.5, b: 1.0 },  // G  boring blue
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // Ab (off)
//     RGB { r: 0.0, g: 1.0, b: 0.0 },  // A  green
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // Bb (off)
//     RGB { r: 1.0, g: 0.0, b: 0.5 },  // B  purpley red
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // C  (off)
//     RGB { r: 1.0, g: 0.5, b: 0.0 },  // C# orange
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // D  (off)
//     RGB { r: 0.0, g: 0.0, b: 1.0 },  // Eb blue
//     RGB { r: 0.0, g: 0.0, b: 0.0 },  // E  (off)
// ];

// // two cycles per octave
// const NOTE_COLOURS: [RGB; 12] = [
//     RGB { r: 1.0, g: 0.0, b: 0.0 },  // F  red
//     RGB { r: 1.0, g: 1.0, b: 0.0 },  // F# yellow
//     RGB { r: 0.0, g: 1.0, b: 0.0 },  // G  green
//     RGB { r: 0.0, g: 1.0, b: 1.0 },  // Ab cyan
//     RGB { r: 0.0, g: 0.0, b: 1.0 },  // A  blue
//     RGB { r: 1.0, g: 0.0, b: 1.0 },  // Bb magenta
//     RGB { r: 1.0, g: 0.0, b: 0.0 },  // B  red
//     RGB { r: 1.0, g: 1.0, b: 0.0 },  // C  yellow
//     RGB { r: 0.0, g: 1.0, b: 0.0 },  // C# green
//     RGB { r: 0.0, g: 1.0, b: 1.0 },  // D  cyan
//     RGB { r: 0.0, g: 0.0, b: 1.0 },  // Eb blue
//     RGB { r: 1.0, g: 0.0, b: 1.5 },  // E  magenta
// ];


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
    fn new(client: &j::Client, name: &str, tx: lossyq::spsc::Sender<f32>) -> Self {
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
    glt: Glt,
    decay_window: VecDeque<RGB>,
    samples_window: VecDeque<f32>,
}

impl Computer {
    fn new(rx: lossyq::spsc::Receiver<f32>) -> Self {
        let mut samples_window: VecDeque<f32> = VecDeque::with_capacity(WINDOW_SIZE);
        let mut decay_window: VecDeque<RGB> = VecDeque::with_capacity(DECAY_SAMPLES);
        for _ in 0..DECAY_SAMPLES {
            decay_window.push_back(RGB::new(0., 0., 0.));
        }
        for _ in 0..WINDOW_SIZE {
            samples_window.push_back(0.0);
        }
        let glt = Glt::new();
        Computer {
            rx,
            glt,
            decay_window,
            samples_window,
        }
    }

    fn update(&mut self) {
        let mut min_samples = 0;
        for sample in self.rx.iter() {
            self.samples_window.push_back(sample);
            if self.samples_window.len() > WINDOW_SIZE {
                self.samples_window.pop_front();
            }
            min_samples += 1;
        }
        // bleh
        let contig_samples: Vec<f32> = self.samples_window.iter().map(|s| *s).collect();
        let mags: [(f64, f64); 144] = self.glt.process(&*contig_samples, min_samples);
        // let freq_samples = self.samples_window
        //     .iter()
        //     .rev()
        //     .take(BUFFSIZE)
        //     .map(|s| *s)
        //     .collect::<Vec<_>>();
        // let freq_mags = (1..100)
        //     .map(|n| 440. * 2.0_f32.powf(1./12.).powf(n as f32 - 48.))
        //     .filter(|f| *f > 44100. / BUFFSIZE as f32)
        //     .map(|f| goertzel::Parameters::new(f, 44100, BUFFSIZE as usize)
        //             .mag(&freq_samples));

        let mut rgb = RGB { r: 0., g: 0., b: 0. };
        for (i, (_f, mag)) in mags.iter().enumerate() {
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
            let weight = (1. - (i as f64 / DECAY_SAMPLES as f64)).powf(2.);
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
