//! Takes 2 audio inputs and outputs them to 2 audio outputs.
//! All JACK notifications are also printed out.
extern crate apodize;
extern crate jack;
extern crate goertzel;
use apodize::{hanning_iter};
use jack::prelude as j;
use std::io;

struct Notifications;

impl j::NotificationHandler for Notifications {
    fn thread_init(&self, _: &j::Client) {
        println!("JACK: thread init");
    }

    fn shutdown(&mut self, status: j::ClientStatus, reason: &str) {
        println!(
            "JACK: shutdown with status {:?} because \"{}\"",
            status,
            reason
        );
    }

    fn buffer_size(&mut self, _: &j::Client, sz: j::JackFrames) -> j::JackControl {
        println!("JACK: buffer size changed to {}", sz);
        j::JackControl::Continue
    }

    fn sample_rate(&mut self, _: &j::Client, srate: j::JackFrames) -> j::JackControl {
        println!("JACK: sample rate changed to {}", srate);
        j::JackControl::Continue
    }

    fn xrun(&mut self, _: &j::Client) -> j::JackControl {
        println!("JACK: xrun occurred");
        j::JackControl::Continue
    }
}

fn main() {
    // Create client
    let (client, _status) = j::Client::new("colours", j::client_options::NO_START_SERVER)
        .unwrap();

    // Register ports. They will be used in a callback that will be
    // called when new data is available.
    let in_a = client
        .register_port("in", j::AudioInSpec::default())
        .unwrap();
    let process_callback = move |_: &j::Client, ps: &j::ProcessScope| -> j::JackControl {
        let in_a_p = j::AudioInPort::new(&in_a, ps);
        let mut note_mags: [f32; 12] = [0.; 12];
        let windowed = hanning_iter(256)
            .enumerate()
            .map(|(i, a)| ((*in_a_p)[i] * 1000. * a as f32) as i16)
            .collect::<Vec<i16>>();
        // let windowed = (*in_a_p)
        //     .iter()
        //     .map(|x| (x * 1000.) as i16)
        //     .collect::<Vec<i16>>();
        let f_mags = (1..100)
            .map(|n| 440. * 2.0_f32.powf(1./12.).powf(n as f32 - 48.))
            .filter(|f| *f > 44100. / 256.)
            .map(|f| goertzel::Parameters::new(f, 44100, 256)
                    .start()
                    .add(&windowed)
                    .finish_mag());
        for (i, mag) in f_mags.enumerate() {
            note_mags[(i % 12) as usize] += mag;
        }
        let graph = note_mags
            .iter()
            .map(|m| *m as u32)
            .map(|m| match m {
                0 ... 100 => " ",
                100 ... 1000 => ".",
                1000 ... 10000 => "o",
                10000 ... 100000 => "O",
                _ => "0",
            })
            .collect::<Vec<&str>>()
            .join("");
        println!("{:?}", graph);
        // let mag4k = goertzel::Parameters::new(80., 44100, 256)
        //     .start()
        //     .add(&samples)
        //     .finish_mag();
        // println!("mag {:?}", mag4k);
        j::JackControl::Continue
    };
    let process = j::ClosureProcessHandler::new(process_callback);

    // Activate the client, which starts the processing.
    let active_client = j::AsyncClient::new(client, Notifications, process).unwrap();

    // Wait for user input to quit
    println!("Press enter/return to quit...");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    active_client.deactivate().unwrap();
}
