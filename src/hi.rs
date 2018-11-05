// https://www.embedded.com/design/configurable-systems/4024443/The-Goertzel-Algorithm

struct Goertz16 {
    pub n: usize,
    cosine: f64,
    sine: f64,
    coeff: f64,
}

impl Goertz16 {
    pub fn new(n: usize) -> Goertz16 {
        let k = 23.0;  // we always want the 23rd bin
        let w = 2.0 * std::f64::consts::PI / (n as f64) * k;
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

const RATE: u32 = 44100;
const BASE_N: usize = 36221;
const BASE_F: f64 = RATE as f64 / BASE_N as f64 * 23.0;
const OCTAVE_BASE: u32 = 16;
const NOTES: usize = (OCTAVE_BASE * 9) as usize;

struct Glt {
    samples: [f32; BASE_N],
    goertzes: [(f64, Goertz16); NOTES],
}

impl Glt {
    pub fn new() -> Glt {
        let mut goertzes: [(f64, Goertz16); NOTES] = unsafe {
            std::mem::uninitialized()
        };
        for g in 0..NOTES {
            let k = 2_f64.powf(g as f64 / 16.0);
            let n = (BASE_N as f64 / k) as usize;
            let target = k * BASE_F;
            goertzes[g] = (target, Goertz16::new(n));
        }
        Glt {
            samples: [0.0; BASE_N],
            goertzes,
        }
    }

    pub fn process(&self) -> [(f64, f64); NOTES] {
        let mut mags: [(f64, f64); NOTES] = unsafe { std::mem::uninitialized() };
        for (i, (f, goertz)) in self.goertzes.iter().enumerate() {
            let mag = goertz.magnitude(&self.samples[0..goertz.n]);
            mags[i] = (*f, mag);
        }
        mags
    }
}
