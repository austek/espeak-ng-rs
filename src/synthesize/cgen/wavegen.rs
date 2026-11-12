//! Port of `wavegen.c`: plays the command queue `generate` builds.

use super::voice::WVoice;
use super::{Env, Wcmd};
use crate::synthesize::envelopes::ENVELOPE_DATA;
use crate::synthesize::phondata::SpectFrame;

include!("../sintab_data.rs");

const N_PEAKS: usize = 9;
const N_LOWHARM: usize = 30;
const MAX_HARMONIC: usize = 400;
const N_ECHO_BUF: usize = 5500;
const STEPSIZE: i32 = 64;
const ENV_LEN: i32 = 128;
const N_TONE_ADJUST: i32 = 1000;
const MAX_PITCH_VALUE: i32 = 101;
const N_FLUTTER: i32 = 0x170;
const WAVEMULT_MAX: i32 = 103;
const WAVEMULT_OFFSET: i32 = 51;
const SAMPLERATE: i32 = 22050;
const PHASE_INC_FACTOR: i32 = 0x800_0000 / SAMPLERATE;

pub const EMBED_P: usize = 1;
pub const EMBED_S: usize = 2;
pub const EMBED_A: usize = 3;
pub const EMBED_R: usize = 4;
pub const EMBED_H: usize = 5;
pub const EMBED_T: usize = 6;
pub const EMBED_S2: usize = 8;
pub const EMBED_F: usize = 13;
pub const N_EMBEDDED_VALUES: usize = 15;

pub const EMBEDDED_DEFAULT: [i32; N_EMBEDDED_VALUES] =
    [0, 50, 175, 100, 50, 0, 0, 0, 175, 0, 0, 0, 0, 0, 0];
const EMBEDDED_MAX: [i32; N_EMBEDDED_VALUES] =
    [0, 0x7fff, 2000, 300, 99, 99, 99, 0, 2000, 0, 0, 0, 0, 4, 0];

static MODULATION_TAB: [[u8; 8]; 8] = [
    [0, 0x00, 0x00, 0x00, 0, 0x46, 0xf2, 0x29],
    [0, 0x2f, 0x00, 0x2f, 0, 0x45, 0xf2, 0x29],
    [0, 0x2f, 0x00, 0x2e, 0, 0x45, 0xf2, 0x28],
    [0, 0x2e, 0x00, 0x2d, 0, 0x34, 0xf2, 0x28],
    [0, 0x2d, 0x2d, 0x2c, 0, 0x34, 0xf2, 0x28],
    [0, 0x2b, 0x2b, 0x2b, 0, 0x34, 0xf2, 0x28],
    [0, 0x2a, 0x2a, 0x2a, 0, 0x34, 0xf2, 0x28],
    [0, 0x29, 0x29, 0x29, 0, 0x34, 0xf2, 0x28],
];

static FLUTTER_TAB: [u8; N_FLUTTER as usize] = [
    0x80, 0x9b, 0xb5, 0xcb, 0xdc, 0xe8, 0xed, 0xec, 0xe6, 0xdc, 0xce, 0xbf, 0xb0, 0xa3, 0x98, 0x90,
    0x8c, 0x8b, 0x8c, 0x8f, 0x92, 0x94, 0x95, 0x92, 0x8c, 0x83, 0x78, 0x69, 0x59, 0x49, 0x3c, 0x31,
    0x2a, 0x29, 0x2d, 0x36, 0x44, 0x56, 0x69, 0x7d, 0x8f, 0x9f, 0xaa, 0xb1, 0xb2, 0xad, 0xa4, 0x96,
    0x87, 0x78, 0x69, 0x5c, 0x53, 0x4f, 0x4f, 0x55, 0x5e, 0x6b, 0x7a, 0x88, 0x96, 0xa2, 0xab, 0xb0,
    0xb1, 0xae, 0xa8, 0xa0, 0x98, 0x91, 0x8b, 0x88, 0x89, 0x8d, 0x94, 0x9d, 0xa8, 0xb2, 0xbb, 0xc0,
    0xc1, 0xbd, 0xb4, 0xa5, 0x92, 0x7c, 0x63, 0x4a, 0x32, 0x1e, 0x0e, 0x05, 0x02, 0x05, 0x0f, 0x1e,
    0x30, 0x44, 0x59, 0x6d, 0x7f, 0x8c, 0x96, 0x9c, 0x9f, 0x9f, 0x9d, 0x9b, 0x99, 0x99, 0x9c, 0xa1,
    0xa9, 0xb3, 0xbf, 0xca, 0xd5, 0xdc, 0xe0, 0xde, 0xd8, 0xcc, 0xbb, 0xa6, 0x8f, 0x77, 0x60, 0x4b,
    0x3a, 0x2e, 0x28, 0x29, 0x2f, 0x3a, 0x48, 0x59, 0x6a, 0x7a, 0x86, 0x90, 0x94, 0x95, 0x91, 0x89,
    0x80, 0x75, 0x6b, 0x62, 0x5c, 0x5a, 0x5c, 0x61, 0x69, 0x74, 0x80, 0x8a, 0x94, 0x9a, 0x9e, 0x9d,
    0x98, 0x90, 0x86, 0x7c, 0x71, 0x68, 0x62, 0x60, 0x63, 0x6b, 0x78, 0x88, 0x9b, 0xaf, 0xc2, 0xd2,
    0xdf, 0xe6, 0xe7, 0xe2, 0xd7, 0xc6, 0xb2, 0x9c, 0x84, 0x6f, 0x5b, 0x4b, 0x40, 0x39, 0x37, 0x38,
    0x3d, 0x43, 0x4a, 0x50, 0x54, 0x56, 0x55, 0x52, 0x4d, 0x48, 0x42, 0x3f, 0x3e, 0x41, 0x49, 0x56,
    0x67, 0x7c, 0x93, 0xab, 0xc3, 0xd9, 0xea, 0xf6, 0xfc, 0xfb, 0xf4, 0xe7, 0xd5, 0xc0, 0xaa, 0x94,
    0x80, 0x71, 0x64, 0x5d, 0x5a, 0x5c, 0x61, 0x68, 0x70, 0x77, 0x7d, 0x7f, 0x7f, 0x7b, 0x74, 0x6b,
    0x61, 0x57, 0x4e, 0x48, 0x46, 0x48, 0x4e, 0x59, 0x66, 0x75, 0x84, 0x93, 0x9f, 0xa7, 0xab, 0xaa,
    0xa4, 0x99, 0x8b, 0x7b, 0x6a, 0x5b, 0x4e, 0x46, 0x43, 0x45, 0x4d, 0x5a, 0x6b, 0x7f, 0x92, 0xa6,
    0xb8, 0xc5, 0xcf, 0xd3, 0xd2, 0xcd, 0xc4, 0xb9, 0xad, 0xa1, 0x96, 0x8e, 0x89, 0x87, 0x87, 0x8a,
    0x8d, 0x91, 0x92, 0x91, 0x8c, 0x84, 0x78, 0x68, 0x55, 0x41, 0x2e, 0x1c, 0x0e, 0x05, 0x01, 0x05,
    0x0f, 0x1f, 0x34, 0x4d, 0x68, 0x81, 0x9a, 0xb0, 0xc1, 0xcd, 0xd3, 0xd3, 0xd0, 0xc8, 0xbf, 0xb5,
    0xab, 0xa4, 0x9f, 0x9c, 0x9d, 0xa0, 0xa5, 0xaa, 0xae, 0xb1, 0xb0, 0xab, 0xa3, 0x96, 0x87, 0x76,
    0x63, 0x51, 0x42, 0x36, 0x2f, 0x2d, 0x31, 0x3a, 0x48, 0x59, 0x6b, 0x7e, 0x8e, 0x9c, 0xa6, 0xaa,
    0xa9, 0xa3, 0x98, 0x8a, 0x7b, 0x6c, 0x5d, 0x52, 0x4a, 0x48, 0x4a, 0x50, 0x5a, 0x67, 0x75, 0x82,
];

use crate::synthesize::wavegen::{PK_SHAPE2, WAVEMULT};

pub static PITCH_ADJUST_TAB: [u8; 102] = [
    64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 86, 87, 88,
    89, 91, 92, 93, 94, 96, 97, 98, 100, 101, 103, 104, 105, 107, 108, 110, 111, 113, 115, 116,
    118, 119, 121, 123, 124, 126, 128, 130, 132, 133, 135, 137, 139, 141, 143, 145, 147, 149, 151,
    153, 155, 158, 160, 162, 164, 167, 169, 171, 174, 176, 179, 181, 184, 186, 189, 191, 194, 197,
    199, 202, 205, 208, 211, 214, 217, 220, 223, 226, 229, 232, 236, 239, 242, 246, 249, 252, 254,
    255,
];

static PK_SHAPE1: [u8; 257] = [
    255, 254, 254, 254, 254, 254, 253, 253, 252, 251, 251, 250, 249, 248, 247, 246, 245, 244, 242,
    241, 239, 238, 236, 234, 233, 231, 229, 227, 225, 223, 220, 218, 216, 213, 211, 209, 207, 205,
    203, 201, 199, 197, 195, 193, 191, 189, 187, 185, 183, 180, 178, 176, 173, 171, 169, 166, 164,
    161, 159, 156, 154, 151, 148, 146, 143, 140, 138, 135, 132, 129, 126, 123, 120, 118, 115, 112,
    108, 105, 102, 99, 96, 95, 93, 91, 90, 88, 86, 85, 83, 82, 80, 79, 77, 76, 74, 73, 72, 70, 69,
    68, 67, 66, 64, 63, 62, 61, 60, 59, 58, 57, 56, 55, 55, 54, 53, 52, 52, 51, 50, 50, 49, 48, 48,
    47, 47, 46, 46, 46, 45, 45, 45, 44, 44, 44, 44, 44, 44, 44, 43, 43, 43, 43, 44, 43, 42, 42, 41,
    40, 40, 39, 38, 38, 37, 36, 36, 35, 35, 34, 33, 33, 32, 32, 31, 30, 30, 29, 29, 28, 28, 27, 26,
    26, 25, 25, 24, 24, 23, 23, 22, 22, 21, 21, 20, 20, 19, 19, 18, 18, 18, 17, 17, 16, 16, 15, 15,
    15, 14, 14, 13, 13, 13, 12, 12, 11, 11, 11, 10, 10, 10, 9, 9, 9, 8, 8, 8, 7, 7, 7, 7, 6, 6, 6,
    5, 5, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];


#[derive(Clone, Copy, Default)]
struct Peak {
    freq: i32,
    height: i32,
    left: i32,
    right: i32,
    freq1: f64,
    height1: f64,
    left1: f64,
    right1: f64,
    freq_inc: f64,
    height_inc: f64,
    left_inc: f64,
    right_inc: f64,
}

#[derive(Clone, Copy, Default)]
struct Mix {
    addr: usize,
    n: i32,
    scale: i32,
    amp: i32,
    ix: i32,
    max: i32,
    offset: i32,
}

#[derive(Clone)]
struct WData {
    pitch_env: Env,
    pitch: i32,
    pitch_ix: i32,
    pitch_inc: i32,
    pitch_base: i32,
    pitch_range: i32,
    mix: Mix,
    amplitude: i32,
    amplitude_v: i32,
    amplitude_fmt: i32,
}

/// The generator state that C keeps in `wavegen.c` statics.
pub struct Wavegen {
    voice: WVoice,
    pub embedded_value: [i32; N_EMBEDDED_VALUES],
    general_amplitude: i32,
    consonant_amp: i32,
    option_harmonic1: i32,
    flutter_amp: i32,
    flutter_ix: i32,
    peaks: [Peak; N_PEAKS],
    peak_harmonic: [i32; N_PEAKS],
    peak_height: [i32; N_PEAKS],
    harm_inc: [i32; N_LOWHARM],
    hspect: [[i32; MAX_HARMONIC]; 2],
    hswitch: usize,
    harmspect: usize,
    nsamples: i32,
    samplecount: i32,
    samplecount_start: i32,
    end_wave: i32,
    wavephase: i32,
    phaseinc: i32,
    cycle_samples: i32,
    cbytes: i32,
    hf_factor: i32,
    modulation_type: i32,
    glottal_flag: i32,
    glottal_reduce: i32,
    amp_ix: i32,
    amp_inc: i32,
    amplitude_env: Option<Env>,
    wdata: WData,
    maxh: i32,
    maxh2: i32,
    agc: i32,
    h_switch_sign: i32,
    cycle_count: i32,
    amplitude2: i32,
    echo_buf: Vec<i16>,
    echo_head: usize,
    echo_tail: usize,
    echo_amp: i32,
    echo_length: i32,
    echo_complete: i32,
    voicing: i32,
    pub out: Vec<i16>,
}

fn env_table(env: Env, phondata: &[u8]) -> &[u8] {
    match env {
        Env::Builtin(ix) => &ENVELOPE_DATA[ix.min(ENVELOPE_DATA.len() - 1)][..],
        Env::Data(off) => phondata.get(off..off + 128).unwrap_or(&ENVELOPE_DATA[0][..]),
    }
}

impl Wavegen {
    pub fn new(voice: WVoice, embedded_value: [i32; N_EMBEDDED_VALUES]) -> Self {
        let wdata = WData {
            pitch_env: Env::Builtin(0),
            pitch: 0,
            pitch_ix: 0,
            pitch_inc: 0,
            pitch_base: 0,
            pitch_range: 0,
            mix: Mix::default(),
            amplitude: 32,
            amplitude_v: 0,
            amplitude_fmt: 100,
        };
        let mut w = Wavegen {
            voice,
            embedded_value,
            general_amplitude: 60,
            consonant_amp: 26,
            option_harmonic1: 10,
            flutter_amp: 64,
            flutter_ix: 0,
            peaks: [Peak::default(); N_PEAKS],
            peak_harmonic: [0; N_PEAKS],
            peak_height: [0; N_PEAKS],
            harm_inc: [0; N_LOWHARM],
            hspect: [[0; MAX_HARMONIC]; 2],
            hswitch: 0,
            harmspect: 0,
            nsamples: 0,
            samplecount: 0,
            samplecount_start: 0,
            end_wave: 0,
            wavephase: 0x7fff_ffff,
            phaseinc: 0,
            cycle_samples: 0,
            cbytes: 0,
            hf_factor: 0,
            modulation_type: 0,
            glottal_flag: 0,
            glottal_reduce: 0,
            amp_ix: 0,
            amp_inc: 0,
            amplitude_env: None,
            wdata,
            maxh: 0,
            maxh2: 0,
            agc: 256,
            h_switch_sign: 0,
            cycle_count: 0,
            amplitude2: 0,
            echo_buf: vec![0; N_ECHO_BUF],
            echo_head: 0,
            echo_tail: 0,
            echo_amp: 0,
            echo_length: 0,
            echo_complete: 0,
            voicing: 64,
            out: Vec::new(),
        };
        w.general_amplitude = w.get_amplitude();
        w.set_voice();
        w
    }

    fn get_amplitude(&mut self) -> i32 {
        const AMP_EMPHASIS: [i32; 5] = [16, 16, 10, 16, 22];
        let amp = self.embedded_value[EMBED_A] * 55 / 100;
        let f = self.embedded_value[EMBED_F].clamp(0, 4) as usize;
        self.general_amplitude = amp * AMP_EMPHASIS[f] / 16;
        self.general_amplitude
    }

    fn set_voice(&mut self) {
        self.consonant_amp = self.voice.consonant_amp * 26 / 100;
        self.set_echo();
        self.set_pitch_formants();
    }

    fn set_echo(&mut self) {
        self.voicing = self.voice.voicing;
        let mut delay = self.voice.echo_delay.min(N_ECHO_BUF as i32 - 1);
        let mut amp = self.voice.echo_amp.min(100);
        self.echo_buf.iter_mut().for_each(|s| *s = 0);
        self.echo_tail = 0;
        if self.embedded_value[EMBED_H] > 0 {
            amp = self.embedded_value[EMBED_H];
            delay = 130;
        }
        if delay == 0 {
            amp = 0;
        }
        self.echo_head = (delay * SAMPLERATE / 1000) as usize;
        self.echo_length = self.echo_head as i32;
        if amp == 0 {
            self.echo_length = 0;
        }
        if amp > 20 {
            self.echo_length = self.echo_head as i32 * 2;
        }
        self.echo_amp = amp;
        self.general_amplitude = self.get_amplitude();
        self.general_amplitude = self.general_amplitude * (500 - amp) / 500;
    }

    fn set_pitch_formants(&mut self) {
        let pitch_value = self.embedded_value[EMBED_P].min(MAX_PITCH_VALUE);
        let factor = if pitch_value > 50 { 256 + (25 * (pitch_value - 50)) / 50 } else { 256 };
        for ix in 0..=5 {
            self.voice.freq[ix] = self.voice.freq2[ix] * factor / 256;
        }
        let factor = self.embedded_value[EMBED_T] * 3;
        self.voice.height[0] = self.voice.height2[0] * (256 - factor * 2) / 256;
        self.voice.height[1] = self.voice.height2[1] * (256 - factor) / 256;
    }

    /// `SetEmbedded`: an embedded command reached the generator.
    pub fn set_embedded(&mut self, control: i32, value: i32) {
        let command = (control & 0x1f) as usize;
        let sign = match control & 0x60 {
            0x60 => -1,
            0x40 => 1,
            _ => 0,
        };
        if command < N_EMBEDDED_VALUES {
            let v = if sign == 0 { value } else { self.embedded_value[command] + value * sign };
            self.embedded_value[command] = v.clamp(0, EMBEDDED_MAX[command].max(0));
        }
        match command {
            EMBED_T => {
                self.set_echo();
                self.set_pitch_formants();
            }
            EMBED_P => self.set_pitch_formants(),
            EMBED_A | EMBED_F => {
                self.get_amplitude();
            }
            EMBED_H => self.set_echo(),
            _ => {}
        }
    }

    fn pk_shape(&self) -> &'static [u8; 257] {
        if self.voice.peak_shape == 0 { &PK_SHAPE1 } else { &PK_SHAPE2 }
    }

    fn peaks_to_harmspect(&mut self, pitch: i32, which: usize, control: i32) -> i32 {
        let nhp = self.voice.n_harmonic_peaks as usize;
        let mut hmax = (self.peaks[nhp].freq.wrapping_add(self.peaks[nhp].right)) / pitch;
        if hmax >= MAX_HARMONIC as i32 {
            hmax = MAX_HARMONIC as i32 - 1;
        }
        let hmax_samplerate = (((SAMPLERATE * 19) / 40) << 16) / pitch;
        hmax = hmax.min(hmax_samplerate);
        let shape = self.pk_shape();
        let mut htab = [0i32; MAX_HARMONIC];
        self.add_peak_shapes(&mut htab, pitch, nhp, shape);
        add_bass(&mut htab, self.peaks[1].height, pitch);
        for pk in nhp + 1..N_PEAKS {
            let x = self.peaks[pk].height >> 14;
            self.peak_height[pk] = x.wrapping_mul(x).wrapping_mul(5) / 2;
            if control == 0 {
                self.peak_harmonic[pk] = self.peaks[pk].freq / pitch;
            }
            if self.peak_harmonic[pk] >= hmax_samplerate {
                self.peak_height[pk] = 0;
            }
        }
        let mut f: i32 = 0;
        for h in 0..=hmax.max(0) as usize {
            let x = htab[h] >> 15;
            htab[h] = x.wrapping_mul(x) >> 8;
            let ix = f >> 19;
            if ix < N_TONE_ADJUST {
                htab[h] = htab[h].wrapping_mul(self.voice.tone_adjust[ix as usize] as i32) >> 13;
            }
            f = f.wrapping_add(pitch);
        }
        htab[1] = htab[1].wrapping_mul(self.option_harmonic1) / 8;
        if control & 1 != 0 {
            for h in 1..N_LOWHARM {
                self.harm_inc[h] = (htab[h] - self.hspect[self.harmspect][h]) >> 3;
            }
        }
        let n = (hmax.max(0) as usize + 1).min(MAX_HARMONIC);
        self.hspect[which][..n].copy_from_slice(&htab[..n]);
        hmax
    }

    fn add_peak_shapes(&self, htab: &mut [i32; MAX_HARMONIC], pitch: i32, nhp: usize, shape: &[u8; 257]) {
        for p in self.peaks.iter().take(nhp + 1) {
            if p.height == 0 || p.freq == 0 {
                continue;
            }
            let fp = p.freq;
            let fhi = p.freq.wrapping_add(p.right);
            let mut h = ((p.freq - p.left) / pitch + 1).max(1);
            let mut f = pitch.wrapping_mul(h);
            while f < fp && (h as usize) < MAX_HARMONIC {
                let ix = ((fp - f) / (p.left >> 8).max(1)).clamp(0, 256) as usize;
                htab[h as usize] = htab[h as usize].wrapping_add((shape[ix] as i32).wrapping_mul(p.height));
                h += 1;
                f += pitch;
            }
            while f < fhi && (h as usize) < MAX_HARMONIC {
                let ix = ((f - fp) / (p.right >> 8).max(1)).clamp(0, 256) as usize;
                htab[h as usize] = htab[h as usize].wrapping_add((shape[ix] as i32).wrapping_mul(p.height));
                h += 1;
                f += pitch;
            }
        }
    }

    fn advance_parameters(&mut self, phondata: &[u8]) {
        self.wdata.pitch_ix += self.wdata.pitch_inc;
        let ix = (self.wdata.pitch_ix >> 8).min(127) as usize;
        let env = env_table(self.wdata.pitch_env, phondata);
        let x = env[ix] as i32 * self.wdata.pitch_range;
        self.wdata.pitch = (x >> 8) + self.wdata.pitch_base;
        self.amp_ix += self.amp_inc;
        if self.flutter_ix >= N_FLUTTER * 64 {
            self.flutter_ix = 0;
        }
        let x = (FLUTTER_TAB[(self.flutter_ix >> 6) as usize] as i32 - 0x80) * self.flutter_amp;
        self.flutter_ix += 64;
        self.wdata.pitch += x;
        if self.wdata.pitch < 102400 {
            self.wdata.pitch = 102400;
        }
        if self.samplecount == self.samplecount_start {
            return;
        }
        self.advance_peaks();
    }

    fn advance_peaks(&mut self) {
        let nhp = self.voice.n_harmonic_peaks as usize;
        for (ix, p) in self.peaks.iter_mut().enumerate().take(8) {
            if ix < 7 {
                p.freq1 += p.freq_inc;
                p.freq = p.freq1 as i32;
            }
            p.height1 += p.height_inc;
            p.height = (p.height1 as i32).max(0);
            if ix <= nhp {
                p.left1 += p.left_inc;
                p.left = p.left1 as i32;
                if ix < 3 {
                    p.right1 += p.right_inc;
                    p.right = p.right1 as i32;
                } else {
                    p.right = p.left;
                }
            }
        }
    }

    fn set_synth(&mut self, length: i32, modn: i32, fr1: &SpectFrame, fr2: &SpectFrame, next_is_spect: bool) {
        const GLOTTAL_REDUCE_TAB1: [i32; 4] = [0x30, 0x30, 0x40, 0x50];
        const GLOTTAL_REDUCE_TAB2: [i32; 4] = [0x90, 0xa0, 0xb0, 0xc0];
        self.end_wave = if next_is_spect { 0 } else { 1 };
        self.modulation_type = modn & 0xff;
        self.glottal_flag = 0;
        if modn & 0x400 != 0 {
            self.glottal_flag = 3;
            self.glottal_reduce = GLOTTAL_REDUCE_TAB1[((modn >> 8) & 3) as usize];
        }
        if modn & 0x800 != 0 {
            self.glottal_flag = 4;
            self.glottal_reduce = GLOTTAL_REDUCE_TAB2[((modn >> 8) & 3) as usize];
        }
        let mut length2 = (length + STEPSIZE / 2) & !0x3f;
        if length2 == 0 {
            length2 = STEPSIZE;
        }
        self.samplecount_start = self.samplecount;
        self.nsamples += length2;
        self.set_synth_peaks(length2, fr1, fr2);
    }

    fn set_synth_peaks(&mut self, length2: i32, fr1: &SpectFrame, fr2: &SpectFrame) {
        let v = &self.voice;
        let length4 = (length2 / 4) as f64;
        let length2f = length2 as f64;
        let step = STEPSIZE as f64;
        self.peaks[7].freq = (7800 * v.freq[7] + v.freqadd[7] * 256) << 8;
        self.peaks[8].freq = (9000 * v.freq[8] + v.freqadd[8] * 256) << 8;
        for ix in 0..8 {
            let p = &mut self.peaks[ix];
            if ix < 7 {
                p.freq1 = (((fr1.ffreq[ix] as i32) * v.freq[ix] + v.freqadd[ix] * 256) << 8) as f64;
                p.freq = p.freq1 as i32;
                let next = (((fr2.ffreq[ix] as i32) * v.freq[ix] + v.freqadd[ix] * 256) << 8) as f64;
                p.freq_inc = ((next - p.freq1) * (step / 4.0)) / length4;
            }
            p.height1 = (((fr1.fheight[ix] as i32) * v.height[ix]) << 6) as f64;
            p.height = p.height1 as i32;
            let next = (((fr2.fheight[ix] as i32) * v.height[ix]) << 6) as f64;
            p.height_inc = ((next - p.height1) * step) / length2f;
            if ix <= 5 && ix <= v.n_harmonic_peaks as usize {
                p.left1 = (((fr1.fwidth[ix] as i32) * v.width[ix]) << 10) as f64;
                p.left = p.left1 as i32;
                let next = (((fr2.fwidth[ix] as i32) * v.width[ix]) << 10) as f64;
                p.left_inc = ((next - p.left1) * step) / length2f;
                if ix < 3 {
                    p.right1 = (((fr1.fright[ix] as i32) * v.width[ix]) << 10) as f64;
                    p.right = p.right1 as i32;
                    let next = (((fr2.fright[ix] as i32) * v.width[ix]) << 10) as f64;
                    p.right_inc = ((next - p.right1) * step) / length2f;
                } else {
                    p.right = p.left;
                }
            }
        }
    }

    fn step_parameters(&mut self, phondata: &[u8]) {
        if self.samplecount == 0 {
            self.hswitch = 0;
            self.harmspect = 0;
            self.maxh2 = self.peaks_to_harmspect(self.wdata.pitch << 4, 0, 0);
            self.amplitude2 = (self.wdata.amplitude * (self.wdata.pitch >> 8) * self.wdata.amplitude_fmt) / (10000 << 3);
            self.h_switch_sign = 890 / (self.wdata.pitch >> 12);
        } else {
            self.advance_parameters(phondata);
        }
        self.phaseinc = (self.wdata.pitch >> 7) * PHASE_INC_FACTOR;
        self.cycle_samples = SAMPLERATE / (self.wdata.pitch >> 12);
        self.hf_factor = self.wdata.pitch >> 11;
        self.maxh = self.maxh2;
        self.harmspect = self.hswitch;
        self.hswitch ^= 1;
        self.maxh2 = self.peaks_to_harmspect(self.wdata.pitch << 4, self.hswitch, 1);
    }

    fn quiet_point(&mut self, phondata: &[u8]) {
        let nhp = self.voice.n_harmonic_peaks as usize;
        self.cycle_count += 1;
        for pk in nhp + 1..N_PEAKS {
            self.peak_harmonic[pk] = ((self.peaks[pk].freq / (self.wdata.pitch * 8)) + 1) / 2;
        }
        self.amplitude2 = (self.wdata.amplitude * (self.wdata.pitch >> 8) * self.wdata.amplitude_fmt) / (10000 << 3);
        self.apply_glottal();
        if let Some(env) = self.amplitude_env {
            let ix = (self.amp_ix >> 8).min(127) as usize;
            self.amplitude2 = self.amplitude2 * env_table(env, phondata)[ix] as i32 / 128;
        }
        self.apply_roughness();
    }

    fn apply_glottal(&mut self) {
        if self.glottal_flag <= 0 {
            return;
        }
        if self.glottal_flag == 3 {
            if (self.nsamples - self.samplecount) < self.cycle_samples * 2 {
                self.glottal_flag = 2;
                self.amplitude2 = self.amplitude2 * self.glottal_reduce / 256;
            }
        } else if self.glottal_flag == 4 {
            self.glottal_flag = 2;
            self.amplitude2 = self.amplitude2 * self.glottal_reduce / 256;
        } else {
            self.glottal_flag -= 1;
        }
    }

    fn apply_roughness(&mut self) {
        let r = self.voice.roughness;
        if !(0..8).contains(&r) {
            return;
        }
        let m = MODULATION_TAB[r as usize][(self.modulation_type & 7) as usize] as i32;
        let (modn_amp, modn_period) = (m & 0xf, m >> 4);
        if modn_period == 0 {
            return;
        }
        if modn_period == 0xf {
            self.amplitude2 = self.amplitude2 * modn_amp / 16;
            self.modulation_type = 0;
        } else if self.cycle_count % modn_period == 0 {
            self.amplitude2 = self.amplitude2 * modn_amp / 16;
        }
    }

    fn harmonic_sum(&self, waveph: u16) -> i32 {
        let nhp = self.voice.n_harmonic_peaks as usize;
        let mut total: i32 = 0;
        if self.cbytes >= 0 && self.cbytes < WAVEMULT_MAX {
            let mut hf: i64 = 0;
            for pk in nhp + 1..N_PEAKS {
                let theta = (self.peak_harmonic[pk] as u16).wrapping_mul(waveph);
                hf += SIN_TAB[(theta >> 5) as usize] as i64 * self.peak_height[pk] as i64;
            }
            let t = hf as i32;
            total = ((t / self.hf_factor) as i64 * WAVEMULT[self.cbytes as usize] as i64) as i32;
        }
        let hs = &self.hspect[self.harmspect];
        let mut theta = waveph;
        let mut h = 1;
        while h <= self.h_switch_sign {
            total = total.wrapping_add((SIN_TAB[(theta >> 5) as usize] as i32).wrapping_mul(hs[h as usize]));
            theta = theta.wrapping_add(waveph);
            h += 1;
        }
        while h <= self.maxh {
            total = total.wrapping_sub((SIN_TAB[(theta >> 5) as usize] as i32).wrapping_mul(hs[h as usize]));
            theta = theta.wrapping_add(waveph);
            h += 1;
        }
        total
    }

    fn mix_sample(&mut self, phondata: &[u8]) -> i32 {
        let m = &mut self.wdata.mix;
        if m.ix >= m.n {
            return 0;
        }
        let at = |i: i32| phondata.get(m.addr + i.max(0) as usize).copied().unwrap_or(0);
        let sample = if m.scale == 0 {
            let s = at(m.ix + m.offset) as i32 + (at(m.ix + m.offset + 1) as i8 as i32) * 256;
            m.ix += 2;
            s
        } else {
            let s = (at(m.offset + m.ix) as i8 as i32) * m.scale;
            m.ix += 1;
            s
        };
        let z2 = (sample * self.wdata.amplitude_v) >> 10;
        let z2 = z2 * m.amp / 32;
        if m.ix + m.offset >= m.max {
            m.offset -= (m.max * 3) / 4;
        }
        z2
    }

    fn emit(&mut self, z: i32) {
        let z = z as i16;
        self.out.push(z);
        self.echo_buf[self.echo_head] = z;
        self.echo_head = (self.echo_head + 1) % N_ECHO_BUF;
    }

    fn echo(&mut self) -> i32 {
        let e = self.echo_buf[self.echo_tail] as i32 * self.echo_amp;
        self.echo_tail = (self.echo_tail + 1) % N_ECHO_BUF;
        e
    }

    fn wavegen(&mut self, phondata: &[u8]) {
        loop {
            if self.end_wave == 0 && self.samplecount == self.nsamples {
                return;
            }
            if self.samplecount & 0x3f == 0 {
                self.step_parameters(phondata);
            } else if self.samplecount & 0x07 == 0 {
                self.interpolate_lowharm();
            }
            self.samplecount += 1;
            if self.wavephase > 0 {
                self.wavephase = self.wavephase.wrapping_add(self.phaseinc);
                if self.wavephase < 0 {
                    self.cbytes = WAVEMULT_OFFSET - self.cycle_samples / 2;
                    if self.samplecount > self.nsamples {
                        return;
                    }
                    self.quiet_point(phondata);
                }
            } else {
                self.wavephase = self.wavephase.wrapping_add(self.phaseinc);
            }
            self.cbytes += 1;
            self.output_sample(phondata);
        }
    }

    fn interpolate_lowharm(&mut self) {
        let hs = self.harmspect;
        let mut h = 1;
        while h < N_LOWHARM as i32 && h <= self.maxh2 && h <= self.maxh {
            self.hspect[hs][h as usize] += self.harm_inc[h as usize];
            h += 1;
        }
        if self.agc < 256 {
            self.agc += 1;
        }
    }

    fn output_sample(&mut self, phondata: &[u8]) {
        let waveph = (self.wavephase >> 16) as u16;
        let mut total = self.harmonic_sum(waveph);
        if self.voicing != 64 {
            total = (total >> 6) * self.voicing;
        }
        let z2 = self.mix_sample(phondata);
        let mut z1 = z2 + (((total >> 8) * self.amplitude2) >> 13);
        z1 += self.echo() >> 8;
        let mut z = (z1 * self.agc) >> 8;
        if z >= 32768 {
            let ov = 8_388_608 / z1 - 1;
            self.agc = self.agc.min(ov);
            z = (z1 * self.agc) >> 8;
        } else if z <= -32768 {
            let ov = -8_388_608 / z1 - 1;
            self.agc = self.agc.min(ov);
            z = (z1 * self.agc) >> 8;
        }
        self.emit(z);
    }

    fn play_silence(&mut self, length: i32) {
        self.nsamples = 0;
        self.samplecount = 0;
        self.wavephase = 0x7fff_ffff;
        for _ in 0..length.max(0) {
            let value = self.echo() >> 8;
            self.emit(value);
        }
    }

    fn play_wave(&mut self, length: i32, phondata: &[u8], addr: usize, scale: i32, amp: i32) {
        self.nsamples = 0;
        self.samplecount = 0;
        let mut ix = addr;
        for _ in 0..length.max(0) {
            let at = |i: usize| phondata.get(i).copied().unwrap_or(0);
            let mut value = if scale == 0 {
                let v = at(ix) as i32 + (at(ix + 1) as i8 as i32) * 256;
                ix += 2;
                v
            } else {
                let v = (at(ix) as i8 as i32) * scale;
                ix += 1;
                v
            };
            value *= self.consonant_amp * self.general_amplitude;
            value >>= 10;
            value = value * amp / 32;
            value += self.echo() >> 8;
            let value = value.clamp(-32768, 32767);
            self.out.push(value as i16);
            self.echo_buf[self.echo_head] = ((value * 3) / 4) as i16;
            self.echo_head = (self.echo_head + 1) % N_ECHO_BUF;
        }
    }

    fn set_amplitude(&mut self, length: i32, env: Option<Env>, value: i32) {
        self.amp_ix = 0;
        self.amp_inc = if length == 0 { 0 } else { (256 * ENV_LEN * STEPSIZE) / length };
        self.wdata.amplitude = value * self.general_amplitude / 16;
        self.wdata.amplitude_v = self.wdata.amplitude * self.voice.consonant_ampv * 15 / 100;
        self.amplitude_env = env;
    }

    /// `SetPitch2`: the pitch base and range for a pitch command.
    pub fn set_pitch2(&self, pitch1: i32, pitch2: i32) -> (i32, i32) {
        let (pitch1, pitch2) = if pitch1 > pitch2 { (pitch2, pitch1) } else { (pitch1, pitch2) };
        let pitch_value = (self.embedded_value[EMBED_P].min(MAX_PITCH_VALUE) - self.embedded_value[EMBED_T]).max(0);
        let mut base = (self.voice.pitch_base * PITCH_ADJUST_TAB[pitch_value as usize] as i32) / 128;
        let range = (self.voice.pitch_range * self.embedded_value[EMBED_R]) / 50;
        base -= (range - self.voice.pitch_range) * 18;
        let pitch_base = base + (pitch1 * range) / 2;
        (pitch_base, base + (pitch2 * range) / 2 - pitch_base)
    }

    fn set_pitch(&mut self, length: i32, env: Env, pitch1: i32, pitch2: i32, phondata: &[u8]) {
        self.wdata.pitch_env = env;
        self.wdata.pitch_ix = 0;
        self.wdata.pitch_inc = if length == 0 { 0 } else { (256 * ENV_LEN * STEPSIZE) / length };
        let (b, r) = self.set_pitch2(pitch1, pitch2);
        self.wdata.pitch_base = b;
        self.wdata.pitch_range = r;
        let e0 = env_table(env, phondata)[0] as i32;
        self.wdata.pitch = ((e0 * self.wdata.pitch_range) >> 8) + self.wdata.pitch_base;
        self.flutter_amp = self.voice.flutter;
    }

    /// `WavegenFill2` over a whole clause queue; `on_marker` sees each marker
    /// with the number of samples produced before it.
    pub fn run(&mut self, queue: &[Wcmd], frames: &[SpectFrame], phondata: &[u8], on_marker: &mut dyn FnMut(u8, usize)) {
        if self.wdata.pitch < 102400 {
            self.wdata.pitch = 102400;
        }
        for (qi, cmd) in queue.iter().enumerate() {
            self.run_cmd(qi, cmd, queue, frames, phondata, on_marker);
        }
    }

    fn run_cmd(&mut self, qi: usize, cmd: &Wcmd, queue: &[Wcmd], frames: &[SpectFrame], phondata: &[u8], on_marker: &mut dyn FnMut(u8, usize)) {
        match *cmd {
            Wcmd::Pitch { len, env, p1, p2 } => self.set_pitch(len, env, p1, p2, phondata),
            Wcmd::Pause(len) => {
                self.echo_complete -= len;
                self.wdata.mix.n = 0;
                self.wdata.amplitude_fmt = 100;
                self.play_silence(len);
            }
            Wcmd::Wave { len, addr, scale, amp } => {
                self.echo_complete = self.echo_length;
                self.wdata.mix.n = 0;
                self.play_wave(len, phondata, addr, scale, amp);
            }
            Wcmd::Wave2 { len, max, addr, scale, amp } => {
                let k = if scale == 0 { 2 } else { 1 };
                self.wdata.mix = Mix { addr, n: len * k, scale, amp, ix: 0, max: max * k, offset: 0 };
            }
            Wcmd::Spect { spect2, len, modn, fr1, fr2 } => {
                if spect2 {
                    self.wdata.mix.n = 0;
                }
                self.echo_complete = self.echo_length;
                let next_is_spect = next_is_spect(&queue[qi + 1..]);
                self.set_synth(len, modn, &frames[fr1], &frames[fr2], next_is_spect);
                self.wavegen(phondata);
            }
            Wcmd::Amplitude { len, env, amp } => self.set_amplitude(len, env, amp),
            Wcmd::Embedded { cmd, value } => self.set_embedded(cmd, value),
            Wcmd::FmtAmplitude(a) => self.wdata.amplitude_fmt = if a == 0 { 100 } else { a },
            Wcmd::Marker(kind) => on_marker(kind, self.out.len()),
        }
    }

    /// The silence C plays after the queue empties, to let an echo die away.
    pub fn finish(&mut self) {
        if self.echo_complete > 0 {
            let n = self.echo_complete;
            self.play_silence(n);
            self.echo_complete = 0;
        }
    }
}

fn next_is_spect(rest: &[Wcmd]) -> bool {
    for c in rest {
        match c {
            Wcmd::Spect { spect2: false, .. } => return true,
            Wcmd::Wave { .. } | Wcmd::Pause(_) => return false,
            _ => {}
        }
    }
    false
}

fn add_bass(htab: &mut [i32; MAX_HARMONIC], height1: i32, pitch: i32) {
    let mut y = height1.wrapping_mul(10);
    let h2 = (1000 << 16) / pitch;
    if h2 <= 0 {
        return;
    }
    let x = y / h2;
    let mut h = 1usize;
    while y > 0 && h < MAX_HARMONIC {
        htab[h] = htab[h].wrapping_add(y);
        h += 1;
        y -= x;
        if x == 0 {
            break;
        }
    }
}
