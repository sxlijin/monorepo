use atomic_float::AtomicF32;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum FilterKind {
    LowPass,
    HighPass,
}

const DEFAULT_Q: f32 = std::f32::consts::FRAC_1_SQRT_2; // Butterworth response

pub fn design_biquad(
    kind: FilterKind,
    sample_rate: f32,
    cutoff_hz: f32,
) -> Option<BiquadCoefficients> {
    if cutoff_hz <= 0.0 || sample_rate <= 0.0 || cutoff_hz >= sample_rate * 0.5 {
        return None;
    }

    let omega = 2.0 * PI * (cutoff_hz / sample_rate);
    let sin_omega = omega.sin();
    let cos_omega = omega.cos();
    let alpha = sin_omega / (2.0 * DEFAULT_Q);

    let (b0, b1, b2, a0, a1, a2) = match kind {
        FilterKind::LowPass => {
            let b0 = (1.0 - cos_omega) * 0.5;
            let b1 = 1.0 - cos_omega;
            let b2 = (1.0 - cos_omega) * 0.5;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterKind::HighPass => {
            let b0 = (1.0 + cos_omega) * 0.5;
            let b1 = -(1.0 + cos_omega);
            let b2 = (1.0 + cos_omega) * 0.5;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
    };

    let inv_a0 = 1.0 / a0;

    Some(BiquadCoefficients {
        b0: b0 * inv_a0,
        b1: b1 * inv_a0,
        b2: b2 * inv_a0,
        a1: a1 * inv_a0,
        a2: a2 * inv_a0,
    })
}

#[derive(Debug)]
pub struct StreamingBiquad {
    coeffs: BiquadCoefficients,
    z1: AtomicF32,
    z2: AtomicF32,
}

impl StreamingBiquad {
    pub fn new(coeffs: BiquadCoefficients) -> Self {
        Self {
            coeffs,
            z1: AtomicF32::new(0.0),
            z2: AtomicF32::new(0.0),
        }
    }

    #[inline]
    pub fn process(&self, input: f32) -> f32 {
        let z1 = self.z1.load(std::sync::atomic::Ordering::SeqCst);
        let z2 = self.z2.load(std::sync::atomic::Ordering::SeqCst);

        let output = self.coeffs.b0 * input + z1;
        let new_z1 = self.coeffs.b1 * input - self.coeffs.a1 * output + z2;
        let new_z2 = self.coeffs.b2 * input - self.coeffs.a2 * output;

        self.z1.store(new_z1, std::sync::atomic::Ordering::SeqCst);
        self.z2.store(new_z2, std::sync::atomic::Ordering::SeqCst);

        output
    }
}

#[derive(Debug, Clone)]
pub struct OfflineBiquad {
    coeffs: BiquadCoefficients,
    z1: f32,
    z2: f32,
}

impl OfflineBiquad {
    pub fn new(coeffs: BiquadCoefficients) -> Self {
        Self {
            coeffs,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let output = self.coeffs.b0 * input + self.z1;
        self.z1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.z2;
        self.z2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
        output
    }

    pub fn process_in_place(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }
}

#[derive(Debug)]
pub struct StreamingFilterBank {
    channels: Vec<StreamingBiquad>,
}

impl StreamingFilterBank {
    pub fn new(coeffs: BiquadCoefficients, channel_count: usize) -> Self {
        let channels = (0..channel_count)
            .map(|_| StreamingBiquad::new(coeffs))
            .collect();
        Self { channels }
    }

    #[inline]
    pub fn process(&self, channel_idx: usize, sample: f32) -> f32 {
        self.channels
            .get(channel_idx)
            .map(|biquad| biquad.process(sample))
            .unwrap_or(sample)
    }
}

#[derive(Debug, Clone)]
pub struct OfflineFilterBank {
    channels: Vec<OfflineBiquad>,
}

impl OfflineFilterBank {
    pub fn new(coeffs: BiquadCoefficients, channel_count: usize) -> Self {
        let channels = (0..channel_count)
            .map(|_| OfflineBiquad::new(coeffs))
            .collect();
        Self { channels }
    }

    pub fn process_frame(&mut self, frame: &mut [f32]) {
        for (idx, sample) in frame.iter_mut().enumerate() {
            if let Some(channel) = self.channels.get_mut(idx) {
                *sample = channel.process_sample(*sample);
            }
        }
    }
}
