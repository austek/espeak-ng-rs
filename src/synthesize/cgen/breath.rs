//! Breath noise from `wavegen.c` (`InitBreath`, `SetBreath`, `ApplyBreath`)
//! and the `espeak_rand` generator it draws from.

use super::voice::WVoice;

const N_PEAKS: usize = 9;
const SAMPLERATE: f64 = 22050.0;

/// C's `RESONATOR`.
#[derive(Clone, Copy, Default)]
struct Resonator {
    a: f64,
    b: f64,
    c: f64,
    x1: f64,
    x2: f64,
}

impl Resonator {
    /// `setresonator`.
    fn set(&mut self, freq: i32, bwidth: i32, init: bool) {
        if init {
            self.x1 = 0.0;
            self.x2 = 0.0;
        }
        let minus_pi_t = -std::f64::consts::PI / SAMPLERATE;
        let two_pi_t = -2.0 * minus_pi_t;
        let x = (minus_pi_t * f64::from(bwidth)).exp();
        self.c = -(x * x);
        self.b = x * (two_pi_t * f64::from(freq)).cos() * 2.0;
        self.a = 1.0 - self.b - self.c;
    }

    /// `resonator`.
    fn step(&mut self, input: f64) -> f64 {
        let x = self.a * input + self.b * self.x1 + self.c * self.x2;
        self.x2 = self.x1;
        self.x1 = x;
        x
    }
}

/// `espeak_rand`, with its state: glibc's `random_r` step.
pub struct Rand(u32);

impl Rand {
    /// `espeak_srand`.
    pub fn seeded(seed: u32) -> Self {
        let mut r = Rand(seed);
        r.next(0, 1);
        r
    }

    pub fn next(&mut self, min: i64, max: i64) -> i64 {
        self.0 = ((u64::from(self.0) * 1_103_515_245 + 12345) % 0x7fff_ffff) as u32;
        (i64::from(self.0) % (max - min + 1)) - min
    }
}

/// The breath resonators and the noise source that drives them.
pub struct Breath {
    res: [Resonator; N_PEAKS],
    rand: Rand,
}

impl Breath {
    /// `InitBreath`, with the generator seeded as `espeak_srand(seed)`.
    pub fn new(seed: u32) -> Self {
        let mut res = [Resonator::default(); N_PEAKS];
        res.iter_mut().for_each(|r| r.set(2000, 200, true));
        Breath { res, rand: Rand::seeded(seed) }
    }

    /// `SetBreath`: retune each breath formant to its peak's frequency.
    pub fn set(&mut self, voice: &WVoice, peak_freq: impl Fn(usize) -> i32) {
        if voice.breath[0] == 0 {
            return;
        }
        for pk in 1..N_PEAKS {
            if voice.breath[pk] != 0 {
                self.res[pk].set(peak_freq(pk) >> 16, voice.breathw[pk], false);
            }
        }
    }

    /// `ApplyBreath`: one sample of filtered noise.
    pub fn apply(&mut self, voice: &WVoice, peak_height: impl Fn(usize) -> i32) -> i32 {
        let noise = self.rand.next(-0x2000, 0x1fff) as f64;
        let mut value: i32 = 0;
        for ix in 1..N_PEAKS {
            if voice.breath[ix] != 0 {
                let amp = voice.breath[ix].wrapping_mul(peak_height(ix) >> 14);
                value = value.wrapping_add((self.res[ix].step(noise) as i32).wrapping_mul(amp));
            }
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rand_follows_the_glibc_step_and_c_range_quirk() {
        let mut r = Rand::seeded(1);
        let state = 1_103_515_245u32 + 12345;
        let next = ((u64::from(state) * 1_103_515_245 + 12345) % 0x7fff_ffff) as i64;
        assert_eq!(r.next(-0x2000, 0x1fff), next % 0x4000 + 0x2000);
    }
}
