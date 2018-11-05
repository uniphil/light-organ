// https://www.embedded.com/design/configurable-systems/4024443/The-Goertzel-Algorithm

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
            q0 = self.coeff * q1 - q2 * (*sample as f64);
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

struct Glt {
    samples: [f32; BASE_N],
    filters: [(f64, Box<[f64]>, Goertz16); NOTES],
}

impl Glt {
    pub fn new() -> Glt {
        let mut filters: [(f64, Box<[f64]>, Goertz16); NOTES] = unsafe {
            std::mem::uninitialized()
        };
        for g in 0..NOTES {
            let k = 2_f64.powf(g as f64 / 16.0);
            let n = (BASE_N as f64 / k) as usize;
            let target = k * BASE_F;
            let window = hann(n);
            filters[g] = (target, window, Goertz16::new(n));
        }
        Glt {
            samples: [0.0; BASE_N],
            filters,
        }
    }

    pub fn process(&self, min_samples: usize) -> [(f64, f64); NOTES] {
        let mut mags: [(f64, f64); NOTES] = unsafe { std::mem::uninitialized() };
        for (i, (f, window, goertz)) in self.filters.iter().enumerate() {
            let mut accumulated_magnitude = 0.0;
            let mut runs = 0;
            for i in 0..(min_samples / (goertz.n / 2)) {
                let start = i * goertz.n / 2;
                let end = start + goertz.n;
                if end > BASE_N {
                    continue
                }
                let samples: Vec<f32> = window
                    .iter()
                    .zip(self.samples[start..end].iter())
                    .map(|(a, s)| (a * *s as f64) as f32)
                    .collect();
                let mag = goertz.magnitude(&*samples);
                accumulated_magnitude += mag;
                runs += 1;
            }
            let magnitude = accumulated_magnitude / runs as f64;
            mags[i] = (*f, magnitude);
        }
        mags
    }
}
