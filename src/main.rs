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

const NOTE_COLOURS: [(f32, f32, f32); 12] = [
    (1.0, 0.0, 0.0),  // F  red
    (1.0, 0.5, 0.0),  // F# orange
    (1.0, 1.0, 0.0),  // G  yellow
    (0.5, 1.0, 0.0),  // Ab lime
    (0.0, 1.0, 0.0),  // A  green
    (0.0, 1.0, 0.5),  // Bb bluish green
    (0.0, 1.0, 1.0),  // B  cyan
    (0.0, 0.5, 1.0),  // C  boring blue
    (0.0, 0.0, 1.0),  // C# blue
    (0.5, 0.0, 1.0),  // D  purple
    (1.0, 0.0, 1.0),  // Eb magenta
    (1.0, 0.0, 0.5),  // E purpley-red
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

fn main() {

    // set up a buffer to hold recent audio samples
    let (mut tx, mut rx) = lossyq::spsc::channel::<f32>(BUFFSIZE * 4);

    // Create client
    let (client, _status) = j::Client::new("colours", j::client_options::NO_START_SERVER)
        .unwrap();

    // Register ports. They will be used in a callback that will be
    // called when new data is available.
    let in_a = client
        .register_port("in", j::AudioInSpec::default())
        .unwrap();

    let process_callback = move |_: &j::Client, ps: &j::ProcessScope| -> j::JackControl {
        // just copy stuff to a non-allocating buffer, overwriting old stuff
        let in_a_p = j::AudioInPort::new(&in_a, ps);
        for v in in_a_p.iter() {
            tx.put(|x| *x = Some(*v));
        }
        j::JackControl::Continue
    };
    let process = j::ClosureProcessHandler::new(process_callback);

    // Activate the client, which starts the processing.
    let active_client = j::AsyncClient::new(client, (), process).unwrap();

    let (mut canvas, mut events) = get_window_canvas();

    let mut samples_window: VecDeque<f32> = VecDeque::new();
    let mut decay_window: [(f32, f32, f32); DECAY_SAMPLES] = [(0., 0., 0.,); DECAY_SAMPLES];
    let mut decay_window_idx: usize = 0;

    'main: loop {
        for sample in rx.iter() {
            samples_window.push_back(sample);
            if samples_window.len() > WINDOW_SIZE {
                samples_window.pop_front();
            }
        }

        let freq_samples = samples_window
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
        let mut rgb: (f32, f32, f32) = (0.0, 0.0, 0.0);
        for (i, mag) in freq_mags.enumerate() {
            rgb.0 += mag * NOTE_COLOURS[i % 12].0;
            rgb.1 += mag * NOTE_COLOURS[i % 12].1;
            rgb.2 += mag * NOTE_COLOURS[i % 12].2;
        }

        decay_window[decay_window_idx] = rgb;
        decay_window_idx = (decay_window_idx + 1) % DECAY_SAMPLES;

        let mut decayed_rgb: (f32, f32, f32) = (0., 0., 0.,);
        for i in 0..DECAY_SAMPLES {
            let weight = (1. - (i as f32 / DECAY_SAMPLES as f32)).powf(2.);
            let window_idx = (decay_window_idx + DECAY_SAMPLES - i) % DECAY_SAMPLES;
            let old_rgb = decay_window[window_idx];
            decayed_rgb.0 += old_rgb.0 * weight * weight;
            decayed_rgb.1 += old_rgb.1 * weight * weight;
            decayed_rgb.2 += old_rgb.2 * weight * weight;
            // let old_rgb = decay_window[((decay_window_idx * 2) as i64 - i % DECAY_SAMPLES) as usize];
        }
        decayed_rgb.0 /= DECAY_SAMPLES as f32 / 4.;
        decayed_rgb.1 /= DECAY_SAMPLES as f32 / 4.;
        decayed_rgb.2 /= DECAY_SAMPLES as f32 / 4.;

        let f = |l: f32| { (l as u32 / 1000).min(255) as u8 };
        canvas.set_draw_color(Color::RGB(f(decayed_rgb.0), f(decayed_rgb.1), f(decayed_rgb.2)));
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
