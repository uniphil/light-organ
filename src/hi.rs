// https://www.embedded.com/design/configurable-systems/4024443/The-Goertzel-Algorithm

struct Goertz {
    pub f: f64,
    pub n: usize,
    coeff: f64,
}

impl Goertz {
    pub fn new(n: usize, target: f64, rate: u32) -> Goertz {
        let k = (0.5 + (n as f64) * target / (rate as f64)).floor();
        let w = 2.0 * std::f64::consts::PI / (n as f64) * k;
        let coeff = 2.0 * w.cos();
        Goertz {
            f: target,
            n,
            coeff,
        }
    }

    pub fn magnitude(&self, samples: &[f32]) -> f64 {
        assert_eq!(samples.len(), self.n);
        let mut q2 = 0.0;
        let mut q1 = 0.0;
        let mut q0;
        for sample in samples {
            q0 = self.coeff * q1 - q2 * (*sample as f64);
            q2 = q1;
            q1 = q0;
        }
        let mag_squared = q1.powi(2) + q2.powi(2) - q1 * q2 * self.coeff;
        mag_squared.sqrt()
    }
}

const RATE: u32 = 44100;
const BASE_N: usize = 36221;
const BASE_F: f64 = RATE as f64 / BASE_N as f64 * 23.0;
const OCTAVE_BASE: u32 = 16;
const NOTES: usize = (OCTAVE_BASE * 9) as usize;

struct Glt {
    samples: [f32; BASE_N],
    goertzes: [Goertz; NOTES],
}

impl Glt {
    pub fn new() -> Glt {
        let mut goertzes: [Goertz; NOTES] = unsafe { std::mem::uninitialized() };
        for g in 0..NOTES {
            let k = 2_f64.powf(g as f64 / 16.0);
            let n = (BASE_N as f64 / k) as usize;
            let target = k * BASE_F;
            goertzes[g] = Goertz::new(n, target, RATE);
        }
        Glt {
            samples: [0.0; BASE_N],
            goertzes,
        }
    }

    pub fn process(&self, min_samples) -> [(f64, f64); NOTES] {
        let mut mags: [(f64, f64); NOTES] = unsafe { std::mem::uninitialized() };
        for (i, goertz) in self.goertzes.iter().enumerate() {
            let mag = goertz.magnitude(&self.samples[0..goertz.n]);
            mags[i] = (goertz.f, mag);
        }
        mags
    }
}
