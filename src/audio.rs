use jack::prelude as j;
use BUFFSIZE;


pub fn get_channel(client: &j::Client, name: &str) -> (JackReceiver, lossyq::spsc::Receiver<f32>) {
    // TODO: accept buffsize as a param?
    let (tx, rx) = lossyq::spsc::channel::<f32>(BUFFSIZE * 4);
    let receiver = JackReceiver::new(client, name, tx);
    (receiver, rx)
}

pub struct JackReceiver {
    pub tx: lossyq::spsc::Sender<f32>,
    pub jack_in: jack::port::Port<j::AudioInSpec>,
}

impl JackReceiver {
    pub fn new(client: &j::Client, name: &str, tx: lossyq::spsc::Sender<f32>) -> Self {
        let jack_in = client
            .register_port(name, j::AudioInSpec::default())
            .unwrap();
        JackReceiver {
            tx,
            jack_in,
        }
    }
}
