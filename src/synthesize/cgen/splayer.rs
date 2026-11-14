//! Port of the NV Speech Player generator (`speechPlayer/src`) that `klatt 6`
//! voices use, with the glibc `rand()` stream it draws its noise from.

use std::collections::VecDeque;
use std::f64::consts::PI;

const SAMPLE_RATE: f64 = 22050.0;
const PITWO: f64 = PI * 2.0;
const RAND_MAX: f64 = 2_147_483_647.0;

pub const N_PARAMS: usize = 47;
pub const VOICE_PITCH: usize = 0;
pub const VIBRATO_PITCH_OFFSET: usize = 1;
pub const VIBRATO_SPEED: usize = 2;
pub const VOICE_TURBULENCE: usize = 3;
pub const GLOTTAL_OPEN: usize = 4;
pub const VOICE_AMPLITUDE: usize = 5;
pub const ASPIRATION_AMPLITUDE: usize = 6;
pub const CF1: usize = 7;
pub const CF_N0: usize = 13;
pub const CF_NP: usize = 14;
pub const CB1: usize = 15;
pub const CB_N0: usize = 21;
pub const CB_NP: usize = 22;
pub const CA_NP: usize = 23;
pub const FRICATION_AMPLITUDE: usize = 24;
pub const PF1: usize = 25;
pub const PB1: usize = 31;
pub const PA1: usize = 37;
pub const PARALLEL_BYPASS: usize = 43;
pub const PRE_FORMANT_GAIN: usize = 44;
pub const OUTPUT_GAIN: usize = 45;
pub const END_VOICE_PITCH: usize = 46;

/// `speechPlayer_frame_t`: 47 doubles, addressed by the index constants.
#[derive(Clone, Copy)]
pub struct Frame(pub [f64; N_PARAMS]);

impl Default for Frame {
    fn default() -> Self {
        Frame([0.0; N_PARAMS])
    }
}

/// glibc's default `rand()` (TYPE_3 additive feedback, seed 1).
pub struct LibcRand {
    ring: [u32; 34],
    pos: usize,
}

impl LibcRand {
    pub fn new() -> Self {
        let mut r = [0u32; 344];
        r[0] = 1;
        for i in 1..31 {
            let prev = i64::from(r[i - 1] as i32);
            let word = 16807 * (prev % 127_773) - 2836 * (prev / 127_773);
            r[i] = (if word < 0 { word + 2_147_483_647 } else { word }) as u32;
        }
        for i in 31..34 {
            r[i] = r[i - 31];
        }
        for i in 34..344 {
            r[i] = r[i - 31].wrapping_add(r[i - 3]);
        }
        let mut ring = [0u32; 34];
        ring.copy_from_slice(&r[310..344]);
        LibcRand { ring, pos: 0 }
    }

    pub fn next(&mut self) -> i32 {
        let v = self.ring[(self.pos + 3) % 34].wrapping_add(self.ring[(self.pos + 31) % 34]);
        self.ring[self.pos] = v;
        self.pos = (self.pos + 1) % 34;
        (v >> 1) as i32
    }
}

impl Default for LibcRand {
    fn default() -> Self {
        Self::new()
    }
}

fn fade(old: f64, new: f64, ratio: f64) -> f64 {
    if new.is_nan() {
        old
    } else {
        old + (new - old) * ratio
    }
}

#[derive(Default)]
struct Noise {
    last: f64,
}

impl Noise {
    fn next(&mut self, rand: &mut LibcRand) -> f64 {
        self.last = f64::from(rand.next()) / RAND_MAX + 0.75 * self.last;
        self.last
    }
}

#[derive(Default)]
struct Cycle {
    last: f64,
}

impl Cycle {
    fn next(&mut self, frequency: f64) -> f64 {
        self.last = (frequency / SAMPLE_RATE + self.last) % 1.0;
        self.last
    }
}

#[derive(Default)]
struct Voice {
    pitch: Cycle,
    vibrato: Cycle,
    aspiration: Noise,
}

impl Voice {
    fn next(&mut self, f: &Frame, rand: &mut LibcRand) -> f64 {
        let p = &f.0;
        let vibrato = (self.vibrato.next(p[VIBRATO_SPEED]) * PITWO).sin() * 0.06 * p[VIBRATO_PITCH_OFFSET] + 1.0;
        let voice = self.pitch.next(p[VOICE_PITCH] * vibrato);
        let aspiration = self.aspiration.next(rand) * 0.2;
        let mut turbulence = aspiration * p[VOICE_TURBULENCE];
        if voice < p[GLOTTAL_OPEN] {
            turbulence *= 0.01;
        }
        let voice = ((voice * 2.0 - 1.0) + turbulence) * p[VOICE_AMPLITUDE];
        aspiration * p[ASPIRATION_AMPLITUDE] + voice
    }
}

#[derive(Default)]
struct Resonator {
    anti: bool,
    set_once: bool,
    frequency: f64,
    bandwidth: f64,
    a: f64,
    b: f64,
    c: f64,
    p1: f64,
    p2: f64,
}

impl Resonator {
    fn new(anti: bool) -> Self {
        Resonator { anti, ..Default::default() }
    }

    fn set_params(&mut self, frequency: f64, bandwidth: f64) {
        if !self.set_once || frequency != self.frequency || bandwidth != self.bandwidth {
            self.frequency = frequency;
            self.bandwidth = bandwidth;
            let r = (-PI / SAMPLE_RATE * bandwidth).exp();
            self.c = -(r * r);
            self.b = r * (PITWO / SAMPLE_RATE * -frequency).cos() * 2.0;
            self.a = 1.0 - self.b - self.c;
            if self.anti && frequency != 0.0 {
                self.a = 1.0 / self.a;
                self.c *= -self.a;
                self.b *= -self.a;
            }
        }
        self.set_once = true;
    }

    fn resonate(&mut self, input: f64, frequency: f64, bandwidth: f64) -> f64 {
        self.set_params(frequency, bandwidth);
        let out = self.a * input + self.b * self.p1 + self.c * self.p2;
        self.p2 = self.p1;
        self.p1 = if self.anti { input } else { out };
        out
    }
}

struct Cascade {
    r: [Resonator; 6],
    n0: Resonator,
    np: Resonator,
}

impl Cascade {
    fn new() -> Self {
        Cascade {
            r: std::array::from_fn(|_| Resonator::new(false)),
            n0: Resonator::new(true),
            np: Resonator::new(false),
        }
    }

    fn next(&mut self, f: &Frame, input: f64) -> f64 {
        let p = &f.0;
        let input = input / 2.0;
        let n0 = self.n0.resonate(input, p[CF_N0], p[CB_N0]);
        let mut out = fade(input, self.np.resonate(n0, p[CF_NP], p[CB_NP]), p[CA_NP]);
        for k in (0..6).rev() {
            out = self.r[k].resonate(out, p[CF1 + k], p[CB1 + k]);
        }
        out
    }
}

struct Parallel {
    r: [Resonator; 6],
}

impl Parallel {
    fn new() -> Self {
        Parallel { r: std::array::from_fn(|_| Resonator::new(false)) }
    }

    fn next(&mut self, f: &Frame, input: f64) -> f64 {
        let p = &f.0;
        let input = input / 2.0;
        let mut out = 0.0;
        for k in 0..6 {
            out += (self.r[k].resonate(input, p[PF1 + k], p[PB1 + k]) - input) * p[PA1 + k];
        }
        fade(out, input, p[PARALLEL_BYPASS])
    }
}

struct Request {
    min_samples: u32,
    fade_samples: u32,
    frame: Frame,
    pitch_inc: f64,
}

/// `FrameManagerImpl`: fades from the frame in force to each queued one.
struct Frames {
    queue: VecDeque<Request>,
    old: Request,
    new: Option<Request>,
    cur: Frame,
    cur_null: bool,
    counter: u32,
    old_is_null: bool,
}

impl Frames {
    fn new() -> Self {
        let old = Request { min_samples: 0, fade_samples: 0, frame: Frame::default(), pitch_inc: 0.0 };
        Frames { queue: VecDeque::new(), old, new: None, cur: Frame::default(), cur_null: true, counter: 0, old_is_null: true }
    }

    fn queue_frame(&mut self, frame: &Frame, min_samples: u32, fade_samples: u32) {
        let pitch_inc = (frame.0[END_VOICE_PITCH] - frame.0[VOICE_PITCH]) / f64::from(min_samples);
        self.queue.push_back(Request { min_samples, fade_samples: fade_samples.max(1), frame: *frame, pitch_inc });
    }

    fn current(&mut self) -> Option<Frame> {
        self.update();
        (!self.cur_null).then_some(self.cur)
    }

    fn update(&mut self) {
        self.counter = self.counter.wrapping_add(1);
        if self.new.is_some() {
            self.fade_in();
        } else if self.counter > self.old.min_samples {
            self.start_next();
        } else {
            self.cur.0[VOICE_PITCH] += self.old.pitch_inc;
            self.old.frame.0[VOICE_PITCH] = self.cur.0[VOICE_PITCH];
        }
    }

    fn fade_in(&mut self) {
        let Some(new) = self.new.take() else { return };
        if self.counter > new.fade_samples {
            self.old = new;
            self.old_is_null = false;
            return;
        }
        let ratio = f64::from(self.counter) / f64::from(new.fade_samples);
        for i in 0..N_PARAMS {
            self.cur.0[i] = fade(self.old.frame.0[i], new.frame.0[i], ratio);
        }
        self.new = Some(new);
    }

    fn start_next(&mut self) {
        let Some(mut req) = self.queue.pop_front() else {
            self.cur_null = true;
            return;
        };
        self.cur_null = false;
        if self.old_is_null {
            self.old.frame = req.frame;
            self.old.frame.0[PRE_FORMANT_GAIN] = 0.0;
        }
        self.counter = 0;
        req.frame.0[VOICE_PITCH] += req.pitch_inc * f64::from(req.fade_samples);
        self.new = Some(req);
    }
}

/// `speechPlayer_handle_t`: the frame queue and wave generator.
pub struct SpeechPlayer {
    frames: Frames,
    voice: Voice,
    fric: Noise,
    cascade: Cascade,
    parallel: Parallel,
}

impl SpeechPlayer {
    pub fn new() -> Self {
        SpeechPlayer {
            frames: Frames::new(),
            voice: Voice::default(),
            fric: Noise::default(),
            cascade: Cascade::new(),
            parallel: Parallel::new(),
        }
    }

    pub fn queue_frame(&mut self, frame: &Frame, min_samples: u32, fade_samples: u32) {
        self.frames.queue_frame(frame, min_samples, fade_samples);
    }

    /// `generate`: up to `count` samples; fewer once the queued frames run out.
    pub fn synthesize(&mut self, count: usize, rand: &mut LibcRand) -> Vec<i16> {
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let Some(f) = self.frames.current() else { break };
            let gain = f.0[PRE_FORMANT_GAIN];
            let voice = self.voice.next(&f, rand);
            let cascade = self.cascade.next(&f, voice * gain);
            let fric = self.fric.next(rand) * 0.3 * f.0[FRICATION_AMPLITUDE];
            let parallel = self.parallel.next(&f, fric * gain);
            let level = (cascade + parallel) * f.0[OUTPUT_GAIN] * 4000.0;
            out.push((level as i32).clamp(-32000, 32000) as i16);
        }
        out
    }
}

impl Default for SpeechPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libc_rand_matches_glibc_seed_one() {
        let mut r = LibcRand::new();
        let first: Vec<i32> = (0..4).map(|_| r.next()).collect();
        assert_eq!(first, [1_804_289_383, 846_930_886, 1_681_692_777, 1_714_636_915]);
    }
}
