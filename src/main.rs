extern crate goertzel;
extern crate jack;
extern crate lossyq;
extern crate sdl2;

mod goertz;

use jack::prelude as j;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::collections::VecDeque;
use std::{env, thread, time};
use goertz::Glt;

// const LOWEST_NOTE_FREQ: f32 = 27.5;  // A0
const HIGHEST_SAMPLE_RATE: u32 = 96000;  // I guess?
const WINDOW_SIZE: usize = HIGHEST_SAMPLE_RATE as usize;

const BUFFSIZE: usize = 1024;

const AMP_SAMPLES: usize = 16;
const COLOUR_SAMPLES: usize = 3;

#[derive(Clone, Debug)]
struct RGB {
    r: f64,
    g: f64,
    b: f64,
}

impl RGB {
    fn new(r: f64, g: f64, b: f64) -> Self {
        RGB { r, g, b }
    }
    fn scale(&self, x: f64) -> RGB {
        RGB {
            r: self.r * x,
            g: self.g * x,
            b: self.b * x,
        }
    }
}

impl From<RGB> for Color {
    fn from(rgb: RGB) -> Self {
        let f = |l: f64| l.min(255.) as u8;
        Color::RGB(f(rgb.r), f(rgb.g), f(rgb.b))
    }
}


  // modified munsell // original munsell hex
  // [59, 100, 47],  // 0 #f0ea00
  // [70, 100, 42],  // 1 #b1d700
  // [115, 100, 40],  // 2 #00ca24
  // [150, 100, 36],  // 3 #00a877
  // [172, 100, 33],  // 4 #00a78a
  // [188, 100, 35],  // 5 #00a59c
  // [201, 100, 39],  // 6 #00a3ac
  // [218, 100, 43],  // 7 #0093af
  // [250, 100, 51],  // 8 #0082b2
  // [260, 100, 49],  // 9 #006ebf
  // [269, 100, 47],  // 10 #7d00f8
  // [285, 100, 39],  // 11 #9f00c5
  // [303, 100, 36],  // 12 #b900a6
  // [321, 100, 41],  // 13 #d00081
  // [332, 100, 44],  // 14 #e20064
  // [345, 100, 47],  // 15 #f2003c
  // [22, 100, 49],  // 16 #f85900
  // [34, 100, 47],  // 17 #f28800
  // [42, 100, 47],  // 18 #f2ab00
  // [51, 100, 47],  // 19 #efcc00


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
    amp_window: VecDeque<f64>,
    colour_window: VecDeque<RGB>,
    samples_window: VecDeque<f32>,  // amplitudes
}

impl Computer {
    fn new(rx: lossyq::spsc::Receiver<f32>) -> Self {
        let mut amp_window: VecDeque<f64> = VecDeque::with_capacity(AMP_SAMPLES);
        let mut colour_window: VecDeque<RGB> = VecDeque::with_capacity(COLOUR_SAMPLES);
        let mut samples_window: VecDeque<f32> = VecDeque::with_capacity(WINDOW_SIZE);
        for _ in 0..AMP_SAMPLES {
            amp_window.push_back(0.0);
        }
        for _ in 0..COLOUR_SAMPLES {
            colour_window.push_back(RGB::new(0.0, 0.0, 0.0));
        }
        for _ in 0..WINDOW_SIZE {
            samples_window.push_back(0.0);
        }
        let glt = Glt::new();
        let colour = RGB::new(0.0, 0.0, 0.0);
        Computer {
            rx,
            glt,
            amp_window,
            colour_window,
            samples_window,
        }
    }

    fn process(&mut self) -> [(f64, f64); 144] {
        let mut min_samples = 0;
        for sample in self.rx.iter() {
            self.samples_window.push_back(sample);
            if self.samples_window.len() > WINDOW_SIZE {
                self.samples_window.pop_front();
            }
            min_samples += 1;
        }
        // bleh
        let mut config_samples: Vec<f32> = self.samples_window
            .iter()
            .map(|s| *s)
            .collect();
        config_samples.reverse();
        self.glt.process(&*config_samples, min_samples)
    }

    fn update_colour(&mut self) -> [(f64, f64); 144] {
        let mags = self.process();

        let mut amplitude = 0.0;

        let mut rgb = RGB { r: 0., g: 0., b: 0. };
        for (i, (_f, mag)) in mags.iter().enumerate() {
            rgb.r += mag * NOTE_COLOURS[i % 16].r;
            rgb.g += mag * NOTE_COLOURS[i % 16].g;
            rgb.b += mag * NOTE_COLOURS[i % 16].b;
            amplitude += mag;
        }
        let highest = 0.0_f64.max(rgb.r).max(rgb.g).max(rgb.b);
        if highest > 255.0 {
            rgb.scale(255.0 / highest);
        }
        self.amp_window.pop_back();
        self.amp_window.push_front(amplitude);
        self.colour_window.pop_back();
        self.colour_window.push_front(rgb);
        mags
    }

    fn get_colour(&self) -> RGB {
        let mut decayed_colour = RGB::new(0., 0., 0.);
        let mut total_weight = 1.;
        for i in 0..COLOUR_SAMPLES {
            let weight = (1. - (i as f64 / COLOUR_SAMPLES as f64)).powf(2.);
            total_weight += weight;
            let old_colour = &self.colour_window[i];
            decayed_colour.r += old_colour.r * weight;
            decayed_colour.g += old_colour.g * weight;
            decayed_colour.b += old_colour.b * weight;
        }
        decayed_colour.r /= total_weight;
        decayed_colour.g /= total_weight;
        decayed_colour.b /= total_weight;
        decayed_colour
    }
}

fn main() {
    let channels = env::args()
        .nth(1)
        .unwrap_or(2.to_string())
        .parse::<u8>()
        .unwrap();

    let (client, _status) = j::Client::new("colours", j::client_options::NO_START_SERVER)
        .expect("\n\nHEY! you might need to `jackdmp -d coreaudio` in another terminal :)\n\n");

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

    for i in 0..channels {
        let source = &format!("system:capture_{}", i+1);
        let destination = &format!("colours:in_{}", i+1);
        match active_client.connect_ports_by_name(source, destination) {
            Ok(_) => println!("{} → {} ✓", source, destination),
            Err(e) => println!("{} → {} ✗\n{:?}", source, destination, e),
        }
    }

    let (mut canvas, mut events) = get_window_canvas();

    'main: loop {
        let t0 = time::Instant::now();
        for computer in &mut computers {
            computer.update_colour();
        }
        // println!("dt {:?}", t0.elapsed());
        let colours = computers
            .iter()
            .map(Computer::get_colour)
            .map(Color::from);

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        let (w, h) = canvas.window().size();
        let rects = computers.len() as u32;
        let rect_width = w as f64 / rects as f64;

        for (i, colour) in colours.enumerate() {
            canvas.set_draw_color(colour);
            canvas
                .fill_rect(Rect::new(i as i32 * rect_width as i32, 0, rect_width as u32, h))
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
        let elapsed = t0.elapsed();
        let target_time = time::Duration::new(0, 1_000_000_000 / 60);
        if elapsed < target_time {
            thread::sleep(target_time - elapsed);
        }
    }
    println!("bye");

    active_client.deactivate().unwrap();
}
