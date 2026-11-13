//! The Klatt synthesizer of `klatt.c`: `KlattInit`, `KlattReset`,
//! `SetSynth_Klatt`, `Wavegen_Klatt`, `parwave` and the sources and filters it
//! drives, with the file-level statics held in [`Klatt`].

use super::breath::Rand;
use super::voice::WVoice;
use crate::synthesize::phondata::{SpectFrame, FRFLAG_KLATT};
use std::f64::consts::PI;

pub const STEPSIZE: i32 = 64;
const N_PEAKS: usize = 9;
const N_KLATTP: usize = 10;
const N_RSN: usize = 20;
const RNZ: usize = 0;
const RNPP: usize = 10;
const R2P: usize = 12;
const R6P: usize = 16;
const RGL: usize = 17;
const RLP: usize = 18;
const ROUT: usize = 19;
const F_NP: usize = 9;
const KLATT_AV: usize = 0;
const KLATT_FNZ: usize = 1;
const KLATT_TILT: usize = 2;
const KLATT_ASPR: usize = 3;
const KLATT_SKEW: usize = 4;
const IMPULSIVE: i32 = 1;
const NATURAL: i32 = 2;
const SAMPLED: i32 = 3;
const SAMPLED2: i32 = 4;
const SCALE_WAV_TAB: [i32; 6] = [45, 38, 45, 45, 55, 45];
const SAMRATE: i64 = 22050;

static NATURAL_SAMPLES2: [i16; 256] = [
    2583, 2516, 2450, 2384, 2319, 2254, 2191, 2127, 2067, 2005, 1946, 1890, 1832, 1779, 1726, 1675,
    1626, 1579, 1533, 1491, 1449, 1409, 1372, 1336, 1302, 1271, 1239, 1211, 1184, 1158, 1134, 1111,
    1089, 1069, 1049, 1031, 1013, 996, 980, 965, 950, 936, 921, 909, 895, 881, 869, 855,
    843, 830, 818, 804, 792, 779, 766, 754, 740, 728, 715, 702, 689, 676, 663, 651,
    637, 626, 612, 601, 588, 576, 564, 552, 540, 530, 517, 507, 496, 485, 475, 464,
    454, 443, 434, 424, 414, 404, 394, 385, 375, 366, 355, 347, 336, 328, 317, 308,
    299, 288, 280, 269, 260, 250, 240, 231, 220, 212, 200, 192, 181, 172, 161, 152,
    142, 133, 123, 113, 105, 94, 86, 76, 67, 57, 49, 39, 30, 22, 11, 4,
    -5, -14, -23, -32, -41, -50, -60, -69, -78, -87, -96, -107, -115, -126, -134, -144,
    -154, -164, -174, -183, -193, -203, -213, -222, -233, -242, -252, -262, -271, -281, -291, -301,
    -310, -320, -330, -339, -349, -357, -368, -377, -387, -397, -406, -417, -426, -436, -446, -456,
    -467, -477, -487, -499, -509, -521, -532, -543, -555, -567, -579, -591, -603, -616, -628, -641,
    -653, -666, -679, -692, -705, -717, -732, -743, -758, -769, -783, -795, -808, -820, -834, -845,
    -860, -872, -885, -898, -911, -926, -939, -955, -968, -986, -999, -1018, -1034, -1054, -1072, -1094,
    -1115, -1138, -1162, -1188, -1215, -1244, -1274, -1307, -1340, -1377, -1415, -1453, -1496, -1538, -1584, -1631,
    -1680, -1732, -1783, -1839, -1894, -1952, -2010, -2072, -2133, -2196, -2260, -2325, -2390, -2456, -2522, -2589,
];

static NATURAL_SAMPLES: [i16; 100] = [
    -310, -400, 530, 356, 224, 89, 23, -10, -58, -16, 461, 599, 536, 701, 770,
    605, 497, 461, 560, 404, 110, 224, 131, 104, -97, 155, 278, -154, -1165,
    -598, 737, 125, -592, 41, 11, -247, -10, 65, 92, 80, -304, 71, 167, -1, 122,
    233, 161, -43, 278, 479, 485, 407, 266, 650, 134, 80, 236, 68, 260, 269, 179,
    53, 140, 275, 293, 296, 104, 257, 152, 311, 182, 263, 245, 125, 314, 140, 44,
    203, 230, -235, -286, 23, 107, 92, -91, 38, 464, 443, 176, 98, -784, -2449,
    -1891, -1045, -1600, -1462, -1384, -1261, -949, -730,
];

static B0: [i16; 224] = [
    1200, 1142, 1088, 1038, 991, 948, 907, 869, 833, 799, 768, 738, 710, 683, 658,
    634, 612, 590, 570, 551, 533, 515, 499, 483, 468, 454, 440, 427, 415, 403,
    391, 380, 370, 360, 350, 341, 332, 323, 315, 307, 300, 292, 285, 278, 272,
    265, 259, 253, 247, 242, 237, 231, 226, 221, 217, 212, 208, 204, 199, 195,
    192, 188, 184, 180, 177, 174, 170, 167, 164, 161, 158, 155, 153, 150, 147,
    145, 142, 140, 137, 135, 133, 131, 128, 126, 124, 122, 120, 119, 117, 115,
    113, 111, 110, 108, 106, 105, 103, 102, 100, 99, 97, 96, 95, 93, 92, 91, 90,
    88, 87, 86, 85, 84, 83, 82, 80, 79, 78, 77, 76, 75, 75, 74, 73, 72, 71,
    70, 69, 68, 68, 67, 66, 65, 64, 64, 63, 62, 61, 61, 60, 59, 59, 58, 57,
    57, 56, 56, 55, 55, 54, 54, 53, 53, 52, 52, 51, 51, 50, 50, 49, 49, 48, 48,
    47, 47, 46, 46, 45, 45, 44, 44, 43, 43, 42, 42, 41, 41, 41, 41, 40, 40,
    39, 39, 38, 38, 38, 38, 37, 37, 36, 36, 36, 36, 35, 35, 35, 35, 34, 34, 33,
    33, 33, 33, 32, 32, 32, 32, 31, 31, 31, 31, 30, 30, 30, 30, 29, 29, 29, 29,
    28, 28, 28, 28, 27, 27,
];

static AMPTABLE: [i16; 88] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 7,
    8, 9, 10, 11, 13, 14, 16, 18, 20, 22, 25, 28, 32,
    35, 40, 45, 51, 57, 64, 71, 80, 90, 101, 114, 128,
    142, 159, 179, 202, 227, 256, 284, 318, 359, 405,
    455, 512, 568, 638, 719, 881, 911, 1024, 1137, 1276,
    1438, 1622, 1823, 2048, 2273, 2552, 2875, 3244, 3645,
    4096, 4547, 5104, 5751, 6488, 7291, 8192, 9093, 10207,
    11502, 12976, 14582, 16384, 18350, 20644, 23429,
    26214, 29491, 32767,
];

const AMP_PAR_FACTOR: [f64; 7] = [0.6, 0.4, 0.15, 0.06, 0.04, 0.022, 0.03];

#[derive(Clone, Copy, Default)]
struct Resonator {
    a: f64,
    b: f64,
    c: f64,
    p1: f64,
    p2: f64,
}

impl Resonator {
    fn step(&mut self, input: f64) -> f64 {
        let x = self.a * input + self.b * self.p1 + self.c * self.p2;
        self.p2 = self.p1;
        self.p1 = x;
        x
    }

    fn anti_step(&mut self, input: f64) -> f64 {
        let x = self.a * input + self.b * self.p1 + self.c * self.p2;
        self.p2 = self.p1;
        self.p1 = input;
        x
    }
}

#[derive(Clone, Copy, Default)]
struct KPeak {
    freq: i32,
    bw: i32,
    ap: i32,
    bp: i32,
    freq1: f64,
    bw1: f64,
    ap1: f64,
    bp1: f64,
    freq_inc: f64,
    bw_inc: f64,
    ap_inc: f64,
    bp_inc: f64,
}

/// C's `klatt_frame_t`.
#[derive(Clone, Default)]
struct KFrame {
    f0hz10: i32,
    avdb: i32,
    fhz: [i32; 10],
    bhz: [i32; 10],
    ap: [i32; 10],
    bphz: [i32; 10],
    asp: i32,
    kopen: i32,
    aturb: i32,
    tltdb: i32,
    af: i32,
    kskew: i32,
    ab: i32,
    avpdb: i32,
    gain0: i32,
    avdb_tmp: i32,
}

/// A `WCMD_KLATT` entry: its length, frames, and the first frame of the next
/// Klatt entry when one follows before any wave or pause.
pub struct KlattCmd<'a> {
    pub length: i32,
    pub fr1: &'a SpectFrame,
    pub fr2: &'a SpectFrame,
    pub next: Option<&'a SpectFrame>,
}

/// The parts of `WGEN_DATA` that `Wavegen_Klatt` reads and advances.
pub struct KlattWdata<'a> {
    pub pitch: &'a mut i32,
    pub pitch_ix: &'a mut i32,
    pub pitch_inc: i32,
    pub pitch_base: i32,
    pub pitch_range: i32,
    pub pitch_env: &'a [u8],
    pub amplitude: i32,
}

/// `klatt_global_t` and the statics of `klatt.c`.
pub struct Klatt {
    glsource: i32,
    f0_flutter: i32,
    nspfr: i64,
    nper: i64,
    ns: i64,
    t0: i64,
    nopen: i64,
    nmod: i64,
    nrand: i64,
    pulse_shape_a: f64,
    pulse_shape_b: f64,
    minus_pi_t: f64,
    two_pi_t: f64,
    onemd: f64,
    decay: f64,
    amp_bypas: f64,
    amp_voice: f64,
    par_amp_voice: f64,
    amp_aspir: f64,
    amp_frica: f64,
    amp_breth: f64,
    amp_gain0: f64,
    sample_factor: f64,
    original_f0: i64,
    fadein: i32,
    fadeout: i32,
    scale_wav: i32,
    rsn: [Resonator; N_RSN],
    kt_frame: KFrame,
    peaks: [KPeak; N_PEAKS],
    end_wave: i32,
    klattp: [i32; N_KLATTP],
    klattp1: [f64; N_KLATTP],
    klattp_inc: [f64; N_KLATTP],
    nsamples: i64,
    sample_count: i64,
    prev_ffreq: [i16; 7],
    time_count: i32,
    noise: f64,
    voice: f64,
    vlast: f64,
    glotlast: f64,
    nlast: f64,
    natural_vwave: f64,
    skew: i64,
}

fn db_to_lin(db: i64) -> f64 {
    if !(0..=87).contains(&db) {
        return 0.0;
    }
    f64::from(AMPTABLE[db as usize]) * 0.001
}

impl Klatt {
    /// `KlattInit`.
    pub fn new() -> Self {
        const FORMANT_HZ: [i32; 10] = [280, 688, 1064, 2806, 3260, 3700, 6500, 7000, 8000, 280];
        const BANDWIDTH: [i32; 10] = [89, 160, 70, 160, 200, 200, 500, 500, 500, 89];
        const PARALLEL_AMP: [i32; 10] = [0, 59, 59, 59, 59, 59, 59, 0, 0, 0];
        const PARALLEL_BW: [i32; 10] = [59, 59, 89, 149, 200, 200, 500, 0, 0, 0];
        let kt_frame = KFrame {
            fhz: FORMANT_HZ,
            bhz: BANDWIDTH,
            ap: PARALLEL_AMP,
            bphz: PARALLEL_BW,
            f0hz10: 1000,
            avdb: 59,
            kopen: 40,
            af: 50,
            gain0: 62,
            ..KFrame::default()
        };
        let mut k = Klatt {
            glsource: IMPULSIVE,
            f0_flutter: 20,
            nspfr: 0,
            nper: 0,
            ns: 0,
            t0: 0,
            nopen: 0,
            nmod: 0,
            nrand: 0,
            pulse_shape_a: 0.0,
            pulse_shape_b: 0.0,
            minus_pi_t: 0.0,
            two_pi_t: 0.0,
            onemd: 0.0,
            decay: 0.0,
            amp_bypas: 0.0,
            amp_voice: 0.0,
            par_amp_voice: 0.0,
            amp_aspir: 0.0,
            amp_frica: 0.0,
            amp_breth: 0.0,
            amp_gain0: 0.0,
            sample_factor: 3.0,
            original_f0: 0,
            fadein: 0,
            fadeout: 0,
            scale_wav: SCALE_WAV_TAB[IMPULSIVE as usize],
            rsn: [Resonator::default(); N_RSN],
            kt_frame,
            peaks: [KPeak::default(); N_PEAKS],
            end_wave: 0,
            klattp: [0; N_KLATTP],
            klattp1: [0.0; N_KLATTP],
            klattp_inc: [0.0; N_KLATTP],
            nsamples: 0,
            sample_count: 0,
            prev_ffreq: [0; 7],
            time_count: 0,
            noise: 0.0,
            voice: 0.0,
            vlast: 0.0,
            glotlast: 0.0,
            nlast: 0.0,
            natural_vwave: 0.0,
            skew: 0,
        };
        k.kt_frame.bhz[0] = BANDWIDTH[0];
        k.reset(2);
        k
    }

    fn setabc(&self, f: i64, bw: i64, rp: &mut Resonator) {
        let r = (self.minus_pi_t * bw as f64).exp();
        rp.c = -(r * r);
        rp.b = r * (self.two_pi_t * f as f64).cos() * 2.0;
        rp.a = 1.0 - rp.b - rp.c;
    }

    fn setzeroabc(&self, f: i64, bw: i64, rp: &mut Resonator) {
        let f = -f;
        let r = (self.minus_pi_t * bw as f64).exp();
        rp.c = -(r * r);
        rp.b = r * (self.two_pi_t * f as f64).cos() * 2.0;
        rp.a = 1.0 - rp.b - rp.c;
        if rp.a != 0.0 {
            rp.a = 1.0 / rp.a;
            rp.c *= -rp.a;
            rp.b *= -rp.a;
        }
    }

    fn set_rsn(&mut self, ix: usize, f: i64, bw: i64) {
        let mut r = self.rsn[ix];
        self.setabc(f, bw, &mut r);
        self.rsn[ix] = r;
    }

    /// `KlattReset`.
    pub fn reset(&mut self, control: i32) {
        if control == 2 {
            let flp = (950 * SAMRATE) / 10000;
            let blp = (630 * SAMRATE) / 10000;
            self.minus_pi_t = -PI / SAMRATE as f64;
            self.two_pi_t = -2.0 * self.minus_pi_t;
            self.set_rsn(RLP, flp, blp);
        }
        if control > 0 {
            self.nper = 0;
            self.t0 = 0;
            self.nopen = 0;
            self.nmod = 0;
            for r in &mut self.rsn[RGL..N_RSN] {
                r.p1 = 0.0;
                r.p2 = 0.0;
            }
        }
        for r in &mut self.rsn[..=R6P] {
            r.p1 = 0.0;
            r.p2 = 0.0;
        }
    }

    fn flutter(&mut self) {
        let t = f64::from(self.time_count);
        let fla = f64::from(self.f0_flutter) / 50.0;
        let flb = self.original_f0 as f64 / 100.0;
        let flc = (PI * 12.7 * t).sin();
        let fld = (PI * 7.1 * t).sin();
        let fle = (PI * 4.7 * t).sin();
        let delta_f0 = fla * flb * (flc + fld + fle) * 10.0;
        self.kt_frame.f0hz10 = (i64::from(self.kt_frame.f0hz10) + delta_f0 as i64) as i32;
        self.time_count += 1;
    }

    fn sampled_source(&self, source_num: i32) -> f64 {
        let (samples, n): (&[i16], i64) =
            if source_num == 0 { (&NATURAL_SAMPLES, 100) } else { (&NATURAL_SAMPLES2, 256) };
        if self.t0 == 0 {
            return 0.0;
        }
        let ftemp = self.nper as f64 / self.t0 as f64 * n as f64;
        let itemp = ftemp as i64;
        let temp_diff = ftemp - itemp as f64;
        let current = f64::from(samples[(itemp % n) as usize]);
        let next = f64::from(samples[((itemp + 1) % n) as usize]);
        let result = current + (next - current) * temp_diff;
        result * self.sample_factor
    }

    fn impulsive_source(&mut self) -> f64 {
        const DOUBLET: [f64; 3] = [0.0, 13_000_000.0, -13_000_000.0];
        let vwave = if self.nper < 3 { DOUBLET[self.nper as usize] } else { 0.0 };
        self.rsn[RGL].step(vwave)
    }

    fn natural_source(&mut self) -> f64 {
        if self.nper < self.nopen {
            self.pulse_shape_a -= self.pulse_shape_b;
            self.natural_vwave += self.pulse_shape_a;
            return self.natural_vwave * 0.028;
        }
        self.natural_vwave = 0.0;
        0.0
    }

    fn gen_noise(&mut self, rand: &mut Rand) -> f64 {
        self.nrand = rand.next(-8191, 8191);
        let noise = self.nrand as f64 + (0.75 * self.nlast);
        self.nlast = noise;
        noise
    }

    fn pitch_synch_par_reset(&mut self) {
        let (f0hz10, avdb_tmp, aturb, kopen) =
            (self.kt_frame.f0hz10, self.kt_frame.avdb_tmp, self.kt_frame.aturb, self.kt_frame.kopen);
        if f0hz10 > 0 {
            self.t0 = (40 * SAMRATE) / i64::from(f0hz10);
            self.amp_voice = db_to_lin(i64::from(avdb_tmp));
            self.nmod = self.t0;
            if avdb_tmp > 0 {
                self.nmod >>= 1;
            }
            self.amp_breth = db_to_lin(i64::from(aturb)) * 0.1;
            self.nopen = 4 * i64::from(kopen);
            if self.glsource == IMPULSIVE && self.nopen > 263 {
                self.nopen = 263;
            }
            if self.nopen >= self.t0 - 1 {
                self.nopen = self.t0 - 2;
            }
            if self.nopen < 40 {
                self.nopen = 40;
            }
            self.pulse_shape_b = f64::from(B0.get((self.nopen - 40) as usize).copied().unwrap_or(27));
            self.pulse_shape_a = (self.pulse_shape_b * self.nopen as f64) * 0.333;
            let temp = SAMRATE / self.nopen;
            let mut gl = self.rsn[RGL];
            self.setabc(0, temp, &mut gl);
            let temp1 = self.nopen as f64 * 0.00833;
            gl.a *= temp1 * temp1;
            self.rsn[RGL] = gl;
            let temp = self.t0 - self.nopen;
            if i64::from(self.kt_frame.kskew) > temp {
                self.kt_frame.kskew = temp as i32;
            }
            let kskew = i64::from(self.kt_frame.kskew);
            self.skew = if self.skew >= 0 { kskew } else { -kskew };
            self.t0 += self.skew;
            self.skew = -self.skew;
        } else {
            self.t0 = 4;
            self.amp_voice = 0.0;
            self.nmod = self.t0;
            self.amp_breth = 0.0;
            self.pulse_shape_a = 0.0;
            self.pulse_shape_b = 0.0;
        }
        if self.t0 != 4 || self.ns == 0 {
            self.decay = 0.033 * f64::from(self.kt_frame.tltdb);
            self.onemd = if self.decay > 0.0 { 1.0 - self.decay } else { 1.0 };
        }
    }

    fn frame_init(&mut self) {
        let fr = self.kt_frame.clone();
        self.original_f0 = i64::from(fr.f0hz10 / 10);
        self.kt_frame.avdb_tmp = (fr.avdb - 7).max(0);
        self.amp_aspir = db_to_lin(i64::from(fr.asp)) * 0.05;
        self.amp_frica = db_to_lin(i64::from(fr.af)) * 0.25;
        self.par_amp_voice = db_to_lin(i64::from(fr.avpdb));
        self.amp_bypas = db_to_lin(i64::from(fr.ab)) * 0.05;
        let mut amp_par = [0.0f64; 7];
        for (ix, a) in amp_par.iter_mut().enumerate() {
            *a = db_to_lin(i64::from(fr.ap[ix])) * AMP_PAR_FACTOR[ix];
        }
        let mut gain0 = i64::from(fr.gain0) - 3;
        if gain0 <= 0 {
            gain0 = 57;
        }
        self.amp_gain0 = db_to_lin(gain0) / f64::from(self.scale_wav);
        for ix in 1..=9 {
            self.set_rsn(ix, i64::from(fr.fhz[ix]), i64::from(fr.bhz[ix]));
        }
        let mut nz = self.rsn[RNZ];
        self.setzeroabc(i64::from(fr.fhz[0]), i64::from(fr.bhz[0]), &mut nz);
        self.rsn[RNZ] = nz;
        for ix in 0..=6 {
            self.set_rsn(RNPP + ix, i64::from(fr.fhz[ix]), i64::from(fr.bphz[ix]));
            self.rsn[RNPP + ix].a *= amp_par[ix];
        }
        self.set_rsn(ROUT, 0, SAMRATE / 2);
    }

    fn voice_source(&mut self) -> f64 {
        match self.glsource {
            IMPULSIVE => self.impulsive_source(),
            NATURAL => self.natural_source(),
            SAMPLED => self.sampled_source(0),
            SAMPLED2 => self.sampled_source(1),
            _ => self.voice,
        }
    }

    fn glottal_sample(&mut self) {
        for _ in 0..4 {
            self.voice = self.voice_source();
            if self.nper >= self.t0 {
                self.nper = 0;
                self.pitch_synch_par_reset();
            }
            self.voice = self.rsn[RLP].step(self.voice);
            self.nper += 1;
        }
        if self.glsource == 5 {
            let v = self.nper as f64 / self.t0 as f64;
            self.voice = ((v * 2.0) - 1.0) * 6000.0;
        }
    }

    fn parallel_bank(&mut self, mut out: f64, sourc: f64) -> f64 {
        for r in &mut self.rsn[R2P..=R6P] {
            out = r.step(sourc) - out;
        }
        out
    }

    /// `parwave`: `nspfr` samples as the integers C writes before echo and clipping.
    fn parwave(
        &mut self,
        rand: &mut Rand,
        amplitude: i32,
        mix: &mut dyn FnMut() -> Option<i32>,
        room: usize,
    ) -> (Vec<i32>, bool) {
        self.flutter();
        let mut out_samples = Vec::with_capacity(self.nspfr.max(0) as usize);
        self.ns = 0;
        while self.ns < self.nspfr {
            out_samples.push(self.parwave_sample(rand, amplitude, mix));
            self.ns += 1;
            if out_samples.len() >= room {
                return (out_samples, true);
            }
        }
        (out_samples, false)
    }

    fn parwave_sample(&mut self, rand: &mut Rand, amplitude: i32, mix: &mut dyn FnMut() -> Option<i32>) -> i32 {
        self.noise = self.gen_noise(rand);
        if self.nper > self.nmod {
            self.noise *= 0.5;
        }
        let frics = self.amp_frica * self.noise;
        self.glottal_sample();
        self.voice = (self.voice * self.onemd) + (self.vlast * self.decay);
        self.vlast = self.voice;
        if self.nper < self.nopen {
            self.voice += self.amp_breth * self.nrand as f64;
        }
        let mut glotout = self.amp_voice * self.voice;
        let mut par_glotout = self.par_amp_voice * self.voice;
        let aspiration = self.amp_aspir * self.noise;
        glotout += aspiration;
        par_glotout += aspiration;
        let out = self.cascade_parallel(glotout, par_glotout, frics);
        let mut temp = f64::from((out * f64::from(amplitude) * self.amp_gain0) as i32);
        if let Some(z2) = mix() {
            temp += f64::from(z2);
        }
        self.apply_fades(temp) as i32
    }

    fn cascade_parallel(&mut self, glotout: f64, par_glotout: f64, frics: f64) -> f64 {
        let mut c = self.rsn[RNZ].anti_step(glotout);
        for ix in (1..=9).rev() {
            c = self.rsn[ix].step(c);
        }
        let mut out = c;
        out += self.rsn[RNPP + 1].step(par_glotout);
        out += self.rsn[RNPP].step(par_glotout);        let sourc = frics + par_glotout - self.glotlast;
        self.glotlast = par_glotout;
        out = self.parallel_bank(out, sourc);
        let outbypas = self.amp_bypas * sourc;
        out = outbypas - out;
        self.rsn[ROUT].step(out)
    }

    fn apply_fades(&mut self, mut temp: f64) -> f64 {
        if self.fadein < 64 {
            temp = (temp * f64::from(self.fadein)) / 64.0;
            self.fadein += 1;
        }
        if self.fadeout > 0 {
            self.fadeout -= 1;
            temp = (temp * f64::from(self.fadeout)) / 64.0;
            if self.fadeout == 0 {
                self.fadein = 0;
            }
        }
        temp
    }

    /// The end-of-wave decision of `SetSynth_Klatt` and the formant-discontinuity
    /// reset.  `next` is the first frame of the next Klatt command, when one
    /// follows before any wave or pause.
    fn set_end_wave(&mut self, fr1: &SpectFrame, fr2: &SpectFrame, next: Option<&SpectFrame>) {
        self.end_wave = match next {
            None => 1,
            Some(fr3) if (1..6).any(|ix| fr3.ffreq[ix] != fr2.ffreq[ix]) => 2,
            Some(_) => 0,
        };
        if (1..6).any(|ix| self.prev_ffreq[ix] != fr1.ffreq[ix]) {
            self.reset(0);
        }
        self.prev_ffreq = fr2.ffreq;
    }

    fn set_klattp(&mut self, length: i32, fr1: &SpectFrame, fr2: &SpectFrame) {
        let has = fr1.frflags & FRFLAG_KLATT != 0;
        for ix in 0..N_KLATTP {
            if ix >= 5 || !has {
                self.klattp1[ix] = 0.0;
                self.klattp[ix] = 0;
                self.klattp_inc[ix] = 0.0;
            } else {
                let v1 = i32::from(fr1.klattp[ix]);
                self.klattp1[ix] = f64::from(v1);
                self.klattp[ix] = v1;
                self.klattp_inc[ix] = f64::from((i32::from(fr2.klattp[ix]) - v1) * STEPSIZE) / f64::from(length);
            }
        }
    }

    fn set_peaks(&mut self, length: i32, fr1: &SpectFrame, fr2: &SpectFrame, v: &WVoice) {
        let len = f64::from(length);
        for ix in 1..6 {
            let p = &mut self.peaks[ix];
            p.freq1 = f64::from(i32::from(fr1.ffreq[ix]) * v.freq[ix]) / 256.0 + f64::from(v.freqadd[ix]);
            p.freq = p.freq1 as i32;
            let next = f64::from(i32::from(fr2.ffreq[ix]) * v.freq[ix]) / 256.0 + f64::from(v.freqadd[ix]);
            p.freq_inc = ((next - p.freq1) * f64::from(STEPSIZE)) / len;
            if ix < 4 {
                p.bw1 = f64::from(i32::from(fr1.bw[ix]) * 2) * (f64::from(v.width[ix]) / 256.0);
                p.bw = p.bw1 as i32;
                let next = f64::from(i32::from(fr2.bw[ix]) * 2);
                p.bw_inc = ((next - p.bw1) * f64::from(STEPSIZE)) / len;
            }
        }
        let np = f64::from(self.kt_frame.fhz[F_NP]);
        let nz = |f: &SpectFrame| {
            let x = i32::from(f.klattp[KLATT_FNZ]) * 2;
            if x == 0 { np } else { f64::from(x) }
        };
        let (first, next) = (nz(fr1), nz(fr2));
        let p0 = &mut self.peaks[0];
        p0.freq1 = first;
        p0.freq = first as i32;
        p0.freq_inc = ((next - first) * f64::from(STEPSIZE)) / len;
        p0.bw1 = 89.0;
        p0.bw = 89;
        p0.bw_inc = 0.0;
        if fr1.frflags & FRFLAG_KLATT != 0 {
            for ix in 1..7 {
                let p = &mut self.peaks[ix];
                p.bp1 = f64::from(i32::from(fr1.klatt_bp[ix]) * 4);
                p.bp = p.bp1 as i32;
                let next = f64::from(i32::from(fr2.klatt_bp[ix]) * 4);
                p.bp_inc = ((next - p.bp1) * f64::from(STEPSIZE)) / len;
                p.ap1 = f64::from(fr1.klatt_ap[ix]);
                p.ap = p.ap1 as i32;
                let next = f64::from(fr2.klatt_ap[ix]);
                p.ap_inc = ((next - p.ap1) * f64::from(STEPSIZE)) / len;
            }
        }
    }

    /// `SetSynth_Klatt` with `control == 1`.
    fn set_synth(&mut self, length: i32, fr1: &SpectFrame, fr2: &SpectFrame, next: Option<&SpectFrame>, v: &WVoice) {
        if v.klatt > 0 && v.klatt <= 5 {
            self.glsource = v.klatt;
            self.scale_wav = SCALE_WAV_TAB[self.glsource as usize];
        }
        self.f0_flutter = v.flutter / 32;
        self.set_end_wave(fr1, fr2, next);
        self.set_klattp(length, fr1, fr2);
        self.nsamples = i64::from(length);
        self.set_peaks(length, fr1, fr2, v);
    }

    fn load_frame(&mut self) {
        let f = &mut self.kt_frame;
        for ix in 0..6 {
            f.fhz[ix] = self.peaks[ix].freq;
            if ix < 4 {
                f.bhz[ix] = self.peaks[ix].bw;
            }
        }
        for ix in 1..7 {
            f.ap[ix] = self.peaks[ix].ap;
        }
        f.avdb = self.klattp[KLATT_AV];
        f.avpdb = self.klattp[6];
        f.af = self.klattp[7];
        f.ab = self.klattp[8];
        f.asp = self.klattp[KLATT_ASPR];
        f.aturb = self.klattp[9];
        f.kskew = self.klattp[KLATT_SKEW];
        f.tltdb = self.klattp[KLATT_TILT];
        f.kopen = self.klattp[5];
    }

    fn advance(&mut self) {
        for p in &mut self.peaks {
            p.freq1 += p.freq_inc;
            p.freq = p.freq1 as i32;
            p.bw1 += p.bw_inc;
            p.bw = p.bw1 as i32;
            p.bp1 += p.bp_inc;
            p.bp = p.bp1 as i32;
            p.ap1 += p.ap_inc;
            p.ap = p.ap1 as i32;
        }
        for ix in 0..N_KLATTP {
            self.klattp1[ix] += self.klattp_inc[ix];
            self.klattp[ix] = self.klattp1[ix] as i32;
        }
    }

    /// `Wavegen_Klatt` for one command: the integer samples, before echo and
    /// clipping, and whether the `room` samples left in the output buffer ran
    /// out, in which case C calls again with `resume` set.
    #[allow(clippy::too_many_arguments)]
    pub fn wavegen(
        &mut self,
        cmd: &KlattCmd,
        resume: bool,
        wdata: &mut KlattWdata,
        voice: &WVoice,
        rand: &mut Rand,
        mix: &mut dyn FnMut() -> Option<i32>,
        room: usize,
    ) -> (Vec<i32>, bool) {
        if !resume {
            self.set_synth(cmd.length, cmd.fr1, cmd.fr2, cmd.next, voice);
            self.sample_count = 0;
        }
        let mut out = Vec::new();
        while self.sample_count < self.nsamples {
            self.kt_frame.f0hz10 = (*wdata.pitch * 10) / 4096;
            self.load_frame();
            self.advance();
            advance_pitch(wdata);
            self.nspfr = (self.nsamples - self.sample_count).min(i64::from(STEPSIZE));
            self.frame_init();
            let (chunk, full) = self.parwave(rand, wdata.amplitude, mix, room - out.len());
            self.sample_count += chunk.len() as i64;
            out.extend(chunk);
            if full {
                return (out, true);
            }
        }
        if self.end_wave > 0 {
            self.fadeout = 64;
            self.end_wave = 0;
            self.sample_count -= 64;
            self.nspfr = 64;
            let (chunk, full) = self.parwave(rand, wdata.amplitude, mix, room - out.len());
            self.sample_count += chunk.len() as i64;
            out.extend(chunk);
            return (out, full);
        }
        (out, false)
    }
}

impl Default for Klatt {
    fn default() -> Self {
        Self::new()
    }
}

fn advance_pitch(w: &mut KlattWdata) {
    *w.pitch_ix += w.pitch_inc;
    let ix = (*w.pitch_ix >> 8).clamp(0, 127) as usize;
    let x = i32::from(w.pitch_env[ix]) * w.pitch_range;
    *w.pitch = (x >> 8) + w.pitch_base;
}
