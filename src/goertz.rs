use std::mem;
use std::ptr;

// https://www.embedded.com/design/configurable-systems/4024443/The-Goertzel-Algorithm

// https://plot.ly/~mrlyule/16/equal-loudness-contours-iso-226-2003/#data

const ATTENUATION_FREQS: [f64; 31] = [
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0, 500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0, 8000.0, 10000.0, 12500.0, 16000.0, 20000.0
];
const ATTENUATIONS: [f64; 31] = [
    0.6723254054962602, 0.7003414164405147, 0.729594163246694, 0.7594456047085627, 0.7864726700747149, 0.813338755591704, 0.8406010297362614, 0.8650519031141868, 0.888000888000888, 0.9109542245502165, 0.931098696461825, 0.9488791365199858, 0.9651345156231149, 0.979431929480901, 0.9893643334157803, 0.9978795060496445, 1.004142086105184, 0.999875015623047, 0.9699321047526672, 0.9553379508000955, 0.9926789924308227, 1.0272213662044172, 1.0380173867912288, 1.0215808964372366, 0.98015192354815, 0.9215528164957953, 0.8751777704846297, 0.8720296490080663, 0.9366584709050463, 0.9448446911538916, 0.6725514922236233
];

#[derive(Debug)]
struct Goertz16 {
    pub n: usize,
    cosine: f64,
    sine: f64,
    coeff: f64,
}

impl Goertz16 {
    pub fn new(n: usize) -> Goertz16 {
        use std::f64::consts::PI;
        let k = 23.0;  // we always want the 23rd bin
        let w = 2.0 * PI / (n as f64) * k;
        let cosine = w.cos();
        let sine = w.sin();
        let coeff = 2.0 * cosine;
        Goertz16 {
            n,
            cosine,
            sine,
            coeff,
        }
    }

    fn qs(&self, samples: &[f32]) -> (f64, f64) {
        assert_eq!(samples.len(), self.n);
        let mut q2 = 0.0;
        let mut q1 = 0.0;
        let mut q0;
        for sample in samples {
            q0 = self.coeff * q1 - q2 + (*sample as f64);
            q2 = q1;
            q1 = q0;
        }
        (q1, q2)
    }

    pub fn components(&self, samples: &[f32]) -> (f64, f64) {
        let (q1, q2) = self.qs(samples);
        let real = q1 - q2 * self.cosine;
        let imag = q2 * self.sine;
        (real, imag)
    }

    pub fn magnitude_squared(&self, samples: &[f32]) -> f64 {
        let (q1, q2) = self.qs(samples);
        let mag_squared = q1.powi(2) + q2.powi(2) - q1 * q2 * self.coeff;
        mag_squared
    }

    pub fn magnitude(&self, samples: &[f32]) -> f64 {
        self.magnitude_squared(samples).sqrt()
    }
}

pub fn hann(n: usize) -> Box<[f64]> {
    use std::f64::consts::PI;
    let window: Vec<_> = (0..n)
        .map(|i| (PI * i as f64 / n as f64).sin().powi(2))
        .collect();
    window.into_boxed_slice()
}

const RATE: u32 = 44100;
const BASE_N: usize = 36221;
const BASE_F: f64 = RATE as f64 / BASE_N as f64 * 23.0;
const OCTAVE_BASE: u32 = 16;
const NOTES: usize = (OCTAVE_BASE * 9) as usize;

pub struct Glt {
    //         f    att  window      g
    filters: [(f64, f64, Box<[f64]>, Goertz16); NOTES],
}

impl Glt {
    pub fn new() -> Glt {
        let mut filters: [(f64, f64, Box<[f64]>, Goertz16); NOTES] = unsafe {
            mem::uninitialized()
        };
        for (g, filter) in filters.iter_mut().enumerate() {
            let k = 2_f64.powf(g as f64 / 16.0);
            let n = (BASE_N as f64 / k) as usize;
            let target = k * BASE_F;
            let att = {
                let high_i = {
                    let mut i = 0;
                    while ATTENUATION_FREQS[i] < target {
                        i += 1;
                    }
                    i
                };
                let low_f_lin = ATTENUATION_FREQS[high_i - 1].log2();
                let high_f_lin = ATTENUATION_FREQS[high_i].log2();
                let f_lin = target.log2();
                let highish = (f_lin - low_f_lin) / (high_f_lin - low_f_lin);
                ATTENUATIONS[high_i] * highish + ATTENUATIONS[high_i - 1] * (1.0 - highish)
            };
            let window = hann(n);
            unsafe {
                ptr::write(filter, (target, att, window, Goertz16::new(n)));
            }
        }
        Glt {
            filters,
        }
    }

    pub fn process(&self, samples: &[f32], min_samples: usize) -> [(f64, f64); NOTES] {
        let mut mags: [(f64, f64); NOTES] = unsafe { mem::uninitialized() };
        for (i, (f, att, window, goertz)) in self.filters.iter().enumerate() {
            let mut accumulated_magnitude = 0.0;
            let mut runs = 0;
            for run in 0..=(min_samples / goertz.n * 2) {
                let start = run * goertz.n / 2;
                let end = start + goertz.n;
                assert!(end <= BASE_N);
                let windowed: Vec<f32> = samples[start..end]
                    .iter()
                    .zip(window.iter())
                    .map(|(s, a)| (*s as f64 * a) as f32)
                    .collect();
                let mag = goertz.magnitude(&samples[start..end]);
                accumulated_magnitude += mag;
                runs += 1;
            }
            let magnitude = accumulated_magnitude / runs as f64 * att;
            mags[i] = (*f, magnitude);
        }
        mags
    }
}
