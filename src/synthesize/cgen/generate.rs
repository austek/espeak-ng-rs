//! `Generate`, `DoSpect2`, `LookupSpect` and their helpers from
//! `synthesize.c` / `synthdata.c`: the clause phoneme list becomes a queue of
//! wavegen commands.

use std::collections::HashMap;

use super::phlist::{Ph, PHLIST_START_OF_SENTENCE, PHLIST_START_OF_WORD, SFLAG_LENGTHEN, SFLAG_NEXT_PAUSE, SFLAG_SEQCONTINUE};
use super::speed::Speed;
use super::voice::{LangSynth, WVoice};
use super::{Env, Wcmd, MARKER_SENTENCE, MARKER_WORD};
use crate::phoneme::{
    PhonemeData, PHON_LENGTHEN, PH_FRICATIVE, PH_LIQUID, PH_LONG, PH_NASAL, PH_NOPAUSE, PH_PAUSE,
    PH_PREVOICE, PH_STOP, PH_TRILL, PH_VFRICATIVE, PH_VOWEL, PH_VSTOP,
};
use crate::synthesize::bytecode::{self, Neighbours};
use crate::synthesize::phondata::{SpectFrame, SpectSeq, FRAME_T2_SIZE, FRAME_T_SIZE, FRFLAG_KLATT};

const FRFLAG_VOWEL_CENTRE: u16 = 0x02;
const FRFLAG_LEN_MOD: u16 = 0x04;
const FRFLAG_BREAK_LF: u16 = 0x08;
const FRFLAG_BREAK: u16 = 0x10;
const FRFLAG_FORMANT_RATE: u16 = 0x20;
const FRFLAG_MODULATE: u16 = 0x40;
const FRFLAG_DEFER_WAV: u16 = 0x80;
const FRFLAG_LEN_MOD2: u16 = 0x4000;
const FRFLAG_COPIED: u16 = 0x8000;

const PD_FMT: usize = 0;
const PD_WAV: usize = 1;
const PD_VWLSTART: usize = 2;
const PD_VWLEND: usize = 3;
const PD_ADDWAV: usize = 4;
const PD_FORNEXTPH: i32 = 0x2;
const PD_DONTLENGTHEN: i32 = 0x4;

const PITCH_FALL: usize = 0;
const PITCH_RISE: usize = 2;
const RMS_GLOTTAL1: i32 = 35;
const RMS_START: i32 = 28;
const VOWEL_FRONT_LENGTH: i32 = 50;
const SAMPLERATE: i32 = 22050;
const FORMANT_RATE: [i32; 9] = [240, 170, 170, 170, 170, 170, 170, 170, 170];

static SQRT_TAB: [i16; 200] = [
    0, 64, 90, 110, 128, 143, 156, 169, 181, 192, 202, 212, 221, 230, 239, 247, 256, 263, 271, 278,
    286, 293, 300, 306, 313, 320, 326, 332, 338, 344, 350, 356, 362, 367, 373, 378, 384, 389, 394,
    399, 404, 409, 414, 419, 424, 429, 434, 438, 443, 448, 452, 457, 461, 465, 470, 474, 478, 483,
    487, 491, 495, 499, 503, 507, 512, 515, 519, 523, 527, 531, 535, 539, 543, 546, 550, 554, 557,
    561, 565, 568, 572, 576, 579, 583, 586, 590, 593, 596, 600, 603, 607, 610, 613, 617, 620, 623,
    627, 630, 633, 636, 640, 643, 646, 649, 652, 655, 658, 662, 665, 668, 671, 674, 677, 680, 683,
    686, 689, 692, 695, 698, 701, 704, 706, 709, 712, 715, 718, 721, 724, 726, 729, 732, 735, 738,
    740, 743, 746, 749, 751, 754, 757, 759, 762, 765, 768, 770, 773, 775, 778, 781, 783, 786, 789,
    791, 794, 796, 799, 801, 804, 807, 809, 812, 814, 817, 819, 822, 824, 827, 829, 832, 834, 836,
    839, 841, 844, 846, 849, 851, 853, 856, 858, 861, 863, 865, 868, 870, 872, 875, 877, 879, 882,
    884, 886, 889, 891, 893, 896, 898, 900, 902,
];

/// The parts of C's `PHONEME_DATA` that `Generate` reads.
#[derive(Default, Clone)]
struct PhData {
    sound_addr: [u32; 5],
    sound_param: [i32; 5],
    set_length: i32,
    pause_before: i32,
    pd_control: i32,
    vowel_transition: [u32; 4],
}

/// C's `FMT_PARAMS`.
#[derive(Default, Clone)]
struct Fmt {
    std_length: i32,
    fmt_addr: u32,
    fmt_length: i32,
    fmt_amp: i32,
    fmt_control: i32,
    fmt2_addr: u32,
    fmt2_lenadj: i32,
    wav_addr: u32,
    wav_amp: i32,
    transition0: u32,
    transition1: u32,
    use_vowelin: bool,
}

#[derive(Clone, Copy)]
struct FrameRef {
    frame: usize,
    frflags: u16,
    length: i32,
}

/// The `synthesize.c` statics, for one clause.
pub struct Generator<'a> {
    phdata: &'a PhonemeData,
    voice: &'a WVoice,
    tr: &'a LangSynth,
    pub speed: Speed,
    pub queue: Vec<Wcmd>,
    pub frames: Vec<SpectFrame>,
    loaded: HashMap<usize, usize>,
    last_pitch_cmd: Option<usize>,
    last_amp_cmd: usize,
    last_frame: Option<usize>,
    last_wcmdq: Option<usize>,
    pitch_length: i32,
    amp_length: i32,
    modn_flags: i32,
    fmt_amplitude: i32,
    syllable_start: usize,
    syllable_end: usize,
    syllable_centre: Option<usize>,
    wave_flag: bool,
    seq_len_adjust: i32,
    embedded_s2: i32,
}

impl<'a> Generator<'a> {
    pub fn new(phdata: &'a PhonemeData, voice: &'a WVoice, tr: &'a LangSynth, speed: Speed, embedded_s2: i32) -> Self {
        Generator {
            phdata,
            voice,
            tr,
            speed,
            queue: Vec::new(),
            frames: Vec::new(),
            loaded: HashMap::new(),
            last_pitch_cmd: None,
            last_amp_cmd: 0,
            last_frame: None,
            last_wcmdq: None,
            pitch_length: 0,
            amp_length: 0,
            modn_flags: 0,
            fmt_amplitude: 0,
            syllable_start: 0,
            syllable_end: 0,
            syllable_centre: None,
            wave_flag: false,
            seq_len_adjust: 0,
            embedded_s2,
        }
    }

    pub fn embedded_s2_value(&self) -> i32 {
        self.embedded_s2
    }

    fn push(&mut self, c: Wcmd) {
        self.queue.push(c);
    }

    fn tail(&self) -> usize {
        self.queue.len()
    }

    // ---- queue helpers ----------------------------------------------------

    fn end_amplitude(&mut self) {
        if self.amp_length > 0 {
            if let Some(Wcmd::Amplitude { len, .. }) = self.queue.get_mut(self.last_amp_cmd) {
                if *len == 0 {
                    *len = self.amp_length;
                }
            }
            self.amp_length = 0;
        }
    }

    fn end_pitch(&mut self, voice_break: bool) {
        if self.pitch_length > 0 {
            if let Some(ix) = self.last_pitch_cmd {
                if let Some(Wcmd::Pitch { len, .. }) = self.queue.get_mut(ix) {
                    if *len == 0 {
                        *len = self.pitch_length;
                    }
                }
                self.pitch_length = 0;
            }
        }
        if voice_break {
            self.last_wcmdq = None;
            self.last_frame = None;
            self.syllable_end = self.tail();
            self.smooth_spect();
            self.syllable_centre = None;
        }
    }

    fn do_amplitude(&mut self, amp: i32, env: Option<Env>) {
        self.last_amp_cmd = self.tail();
        self.amp_length = 0;
        self.push(Wcmd::Amplitude { len: 0, env, amp });
    }

    fn do_pitch(&mut self, env: Env, pitch1: i32, pitch2: i32) {
        self.end_pitch(false);
        let (env, p1, p2) = if pitch1 == 255 { (Env::Builtin(PITCH_FALL), 55, 76) } else { (env, pitch1, pitch2) };
        self.last_pitch_cmd = Some(self.tail());
        self.pitch_length = 0;
        self.push(Wcmd::Pitch { len: 0, env, p1, p2: p2.max(0) });
    }

    fn pause_length(&self, pause: i32, control: i32) -> i32 {
        let len = if control == 0 {
            if pause >= 200 { pause * self.speed.clause_pause_factor / 256 } else { pause * self.speed.pause_factor / 256 }
        } else {
            pause * self.speed.wav_factor / 256
        };
        len.max(self.speed.min_pause)
    }

    pub fn do_pause(&mut self, length: i32, control: i32) {
        let len = if length == 0 {
            0
        } else {
            let len = self.pause_length(length, control);
            if len < 90000 { len * SAMPLERATE / 1000 } else { len * (SAMPLERATE / 25) / 40 }
        };
        self.end_pitch(true);
        self.push(Wcmd::Pause(len));
        self.last_frame = None;
        if self.fmt_amplitude != 0 {
            self.fmt_amplitude = 0;
            self.push(Wcmd::FmtAmplitude(0));
        }
    }

    // ---- samples --------------------------------------------------------

    fn do_sample2(&mut self, index: u32, which: i32, std_length: i32, control: i32, length_mod: i32, amp: i32) -> i32 {
        let index = (index & 0x7f_ffff) as usize;
        let pd = &self.phdata.phondata;
        let Some(head) = pd.get(index..index + 4) else { return 0 };
        let wav_scale = head[2] as i32;
        let mut wav_length = head[1] as i32 * 256 + head[0] as i32;
        if wav_length == 0 {
            return 0;
        }
        let mut min_length = self.speed.min_sample_len;
        if wav_scale == 0 {
            min_length *= 2;
        }
        let mut std_length = std_length;
        if std_length > 0 {
            std_length = std_length * SAMPLERATE / 1000;
            if wav_scale == 0 {
                std_length *= 2;
            }
            min_length = min_length.max(min_length * std_length / wav_length);
        } else {
            std_length = wav_length;
        }
        if length_mod > 0 {
            std_length = std_length * length_mod / 256;
        }
        let mut length = std_length * self.speed.wav_factor / 256;
        if control & PD_DONTLENGTHEN != 0 && length > std_length {
            length = std_length;
        }
        length = length.max(min_length);
        if wav_scale == 0 {
            length /= 2;
            wav_length /= 2;
        }
        if amp < 0 {
            return length;
        }
        self.queue_sample(index + 4, which, length, wav_length, wav_scale, amp)
    }

    fn queue_sample(&mut self, addr: usize, which: i32, mut length: i32, wav_length: i32, scale: i32, amp: i32) -> i32 {
        let len4 = wav_length / 4;
        let k = if scale == 0 { 2 } else { 1 };
        if which & 0x100 != 0 {
            self.last_wcmdq = Some(self.tail());
            self.push(Wcmd::Wave2 { len: length, max: wav_length, addr, scale, amp });
            return length;
        }
        let x;
        if length > wav_length {
            x = len4 * 3;
            length -= x;
        } else {
            x = length;
            length = 0;
        }
        self.last_wcmdq = Some(self.tail());
        self.push(Wcmd::Wave { len: x, addr, scale, amp });
        while length > len4 * 3 {
            self.last_wcmdq = Some(self.tail());
            self.push(Wcmd::Wave { len: len4 * 2, addr: addr + (len4 * k) as usize, scale, amp });
            length -= len4 * 2;
        }
        if length > 0 {
            let off = (wav_length - length) * k;
            self.last_wcmdq = Some(self.tail());
            self.push(Wcmd::Wave { len: length, addr: addr + off.max(0) as usize, scale, amp });
        }
        length
    }

    fn do_sample3(&mut self, ph: &PhData, length_mod: i32) -> i32 {
        self.end_pitch(true);
        let mut amp2 = ph.sound_param[PD_WAV];
        if amp2 == 0 {
            amp2 = 100;
        }
        amp2 = amp2 * 32 / 100;
        self.seq_len_adjust = 0;
        let len = if ph.sound_addr[PD_WAV] == 0 {
            0
        } else {
            self.do_sample2(ph.sound_addr[PD_WAV], 2, ph.set_length * 2, ph.pd_control, length_mod, amp2)
        };
        self.last_frame = None;
        len
    }

    // ---- frames ---------------------------------------------------------

    fn load_seq(&mut self, addr: u32) -> Vec<FrameRef> {
        let addr = addr as usize;
        let Some(seq) = SpectSeq::parse(&self.phdata.phondata, addr) else { return Vec::new() };
        let size = if seq.is_klatt { FRAME_T_SIZE } else { FRAME_T2_SIZE };
        let n = seq.frames.len().min(24);
        let mut out = Vec::with_capacity(n);
        for (i, f) in seq.frames.into_iter().take(n).enumerate() {
            let key = addr + 4 + i * size;
            let (frflags, length) = (f.frflags, f.length as i32);
            let idx = *self.loaded.entry(key).or_insert_with(|| {
                self.frames.push(f);
                self.frames.len() - 1
            });
            out.push(FrameRef { frame: idx, frflags, length });
        }
        out
    }

    fn copy_frame(&mut self, frame: usize, copy: bool) -> usize {
        if !copy && self.frames[frame].frflags & FRFLAG_COPIED != 0 {
            return frame;
        }
        let mut f = self.frames[frame].clone();
        f.length = 0;
        f.frflags |= FRFLAG_COPIED;
        self.frames.push(f);
        self.frames.len() - 1
    }

    fn set_frame_rms(&mut self, fr: usize, new_rms: i32) {
        let f = &mut self.frames[fr];
        if f.rms == 0 {
            return;
        }
        let x = ((new_rms * 64) / f.rms as i32).min(199);
        let x = SQRT_TAB[x.max(0) as usize] as i32;
        for h in f.fheight.iter_mut() {
            *h = ((*h as i32 * x) / 0x200) as u8;
        }
    }

    #[allow(clippy::too_many_arguments)] // mirrors the C AdjustFormants() argument list
    fn adjust_formants(&mut self, fr: usize, target: i32, min: i32, max: i32, f1_adj: i32, f3_adj: i32, hf_reduce: i32, flags: i32) {
        let target = target * self.voice.formant_factor / 256;
        let f = &mut self.frames[fr];
        let x = ((target - f.ffreq[2] as i32) / 2).min(max).max(min);
        f.ffreq[2] = (f.ffreq[2] as i32 + x) as i16;
        f.ffreq[3] = (f.ffreq[3] as i32 + f3_adj) as i16;
        let f3_adj = if flags & 0x20 != 0 { -f3_adj } else { f3_adj };
        f.ffreq[4] = (f.ffreq[4] as i32 + f3_adj) as i16;
        f.ffreq[5] = (f.ffreq[5] as i32 + f3_adj) as i16;
        let lower = |f: &mut SpectFrame, x: i32, both: bool| {
            f.ffreq[1] = (f.ffreq[1] as i32 + x) as i16;
            if both {
                f.ffreq[0] = (f.ffreq[0] as i32 + x) as i16;
            }
        };
        match f1_adj {
            1 => lower(f, (235 - f.ffreq[1] as i32).clamp(-100, -60), false),
            2 => lower(f, (235 - f.ffreq[1] as i32).clamp(-300, -150), true),
            3 => {
                let x = (100 - f.ffreq[1] as i32).max(-400);
                lower(f, if x > -300 { -400 } else { x }, true)
            }
            _ => {}
        }
        for h in f.fheight.iter_mut().skip(2) {
            *h = ((*h as i32 * hf_reduce) / 100) as u8;
        }
    }

    fn vowel_closeness(&self, fr: usize) -> i32 {
        match self.frames[fr].ffreq[1] {
            f if f < 300 => 3,
            f if f < 400 => 2,
            f if f < 500 => 1,
            _ => 0,
        }
    }

    fn formant_transition(&mut self, seq: &mut Vec<FrameRef>, data1: u32, data2: u32, which: i32) -> i32 {
        if seq.len() < 2 {
            return 0;
        }
        let len = ((data1 & 0x3f) * 2) as i32;
        let rms = ((data1 >> 6) & 0x3f) as i32;
        let flags = (data1 >> 12) as i32;
        let t = Transition {
            f2: ((data2 & 0x3f) * 50) as i32,
            f2_min: (((data2 >> 6) & 0x1f) as i32 - 15) * 50,
            f2_max: (((data2 >> 11) & 0x1f) as i32 - 15) * 50,
            f3_adj: (((data2 >> 16) & 0x1f) as i32 - 15) * 50,
            f3_amp: (((data2 >> 21) & 0x1f) * 8) as i32,
            f1: ((data2 >> 26) & 0x7) as i32,
            vcolour: (data2 >> 29) as i32,
        };
        let fr = if which == 1 {
            Some(self.transition_in(seq, len, rms, flags, &t))
        } else {
            self.transition_out(seq, len, rms * 2, flags, &t)
        };
        if let Some(fr) = fr {
            if flags & 4 != 0 {
                self.frames[fr].frflags |= FRFLAG_FORMANT_RATE;
            }
            if flags & 2 != 0 {
                self.frames[fr].frflags |= FRFLAG_BREAK;
            }
        }
        if flags & 0x40 != 0 {
            self.do_pause(20, 0);
        }
        if flags & 16 != 0 { len } else { 0 }
    }

    fn transition_in(&mut self, seq: &mut [FrameRef], len: i32, rms: i32, flags: i32, t: &Transition) -> usize {
        let fr = self.copy_frame(seq[0].frame, false);
        seq[0].frame = fr;
        seq[0].length = if len > 0 { len } else { VOWEL_FRONT_LENGTH };
        seq[0].frflags |= FRFLAG_LEN_MOD2;
        self.frames[fr].frflags |= FRFLAG_LEN_MOD2;
        let next_rms = self.frames[seq[1].frame].rms as i32;
        if t.f2 != 0 {
            if rms & 0x20 != 0 {
                self.set_frame_rms(fr, (next_rms * (rms & 0x1f)) / 30);
            }
            self.adjust_formants(fr, t.f2, t.f2_min, t.f2_max, t.f1, t.f3_adj, t.f3_amp, flags);
            if rms & 0x20 == 0 {
                self.set_frame_rms(fr, rms * 2);
            }
        } else if flags & 8 != 0 {
            self.set_frame_rms(fr, (next_rms * 24) / 32);
        } else {
            self.set_frame_rms(fr, RMS_START);
        }
        if flags & 8 != 0 {
            self.modn_flags = 0x800 + (self.vowel_closeness(fr) << 8);
        }
        fr
    }

    fn transition_out(&mut self, seq: &mut Vec<FrameRef>, len: i32, rms: i32, flags: i32, t: &Transition) -> Option<usize> {
        if t.f2 == 0 && flags == 0 {
            return None;
        }
        let n = seq.len();
        let mut rms = rms;
        let fr = if flags & 8 != 0 {
            let fr = self.copy_frame(seq[n - 1].frame, false);
            seq[n - 1].frame = fr;
            rms = RMS_GLOTTAL1;
            self.modn_flags = 0x400 + (self.vowel_closeness(fr) << 8);
            fr
        } else {
            seq[n - 1].length = len;
            let fr = self.copy_frame(seq[n - 1].frame, true);
            seq.push(FrameRef { frame: fr, frflags: 0, length: 0 });
            if len > 36 {
                self.seq_len_adjust += len - 36;
            }
            if t.f2 != 0 {
                self.adjust_formants(fr, t.f2, t.f2_min, t.f2_max, t.f1, t.f3_adj, t.f3_amp, flags);
            }
            fr
        };
        self.set_frame_rms(fr, rms);
        if t.vcolour > 0 && t.vcolour <= 2 {
            self.vowel_colour(seq, t.vcolour as usize - 1);
        }
        Some(fr)
    }

    fn vowel_colour(&mut self, seq: &mut [FrameRef], vc: usize) {
        const VCOLOURING: [[i32; 5]; 2] = [[243, 272, 256, 256, 256], [256, 256, 240, 240, 240]];
        for r in seq.iter_mut() {
            let fr = self.copy_frame(r.frame, false);
            r.frame = fr;
            let f = &mut self.frames[fr];
            for formant in 1..=5 {
                f.ffreq[formant] = ((f.ffreq[formant] as i32 * VCOLOURING[vc][formant - 1]) / 256) as i16;
            }
        }
    }

    fn lookup_spect(&mut self, this: &Ph, which: i32, fmt: &Fmt) -> Vec<FrameRef> {
        let mut all = self.load_seq(fmt.fmt_addr);
        if all.is_empty() {
            return all;
        }
        self.seq_len_adjust = fmt.fmt2_lenadj + fmt.fmt_length;
        let seq_break = all.iter().rposition(|f| f.frflags & FRFLAG_VOWEL_CENTRE != 0).unwrap_or(0);
        let mut frames = if seq_break > 0 {
            if which == 1 { all[..=seq_break].to_vec() } else { all.split_off(seq_break) }
        } else {
            all
        };
        if this.typ == PH_VOWEL && fmt.fmt2_addr == 0 && fmt.use_vowelin {
            let adj = self.formant_transition(&mut frames, fmt.transition0, fmt.transition1, which);
            self.seq_len_adjust += adj;
        }
        let nf1 = frames.len() - 1;
        let length1: i32 = frames[..nf1].iter().map(|f| f.length).sum();
        if fmt.fmt2_addr != 0 {
            self.append_seq2(&mut frames, fmt.fmt2_addr);
        }
        if length1 > 0 {
            self.scale_lengths(&mut frames[..nf1], length1, which, this, fmt);
        }
        frames
    }

    fn append_seq2(&mut self, frames: &mut Vec<FrameRef>, addr: u32) {
        let seq2 = self.load_seq(addr);
        let mut nf = frames.len() - 1;
        for (ix, f) in seq2.iter().enumerate() {
            if nf >= frames.len() {
                frames.push(FrameRef { frame: f.frame, frflags: f.frflags, length: f.length });
            }
            frames[nf].length = f.length;
            if ix > 0 {
                frames[nf].frame = f.frame;
                frames[nf].frflags = f.frflags;
            }
            nf += 1;
        }
    }

    fn scale_lengths(&mut self, frames: &mut [FrameRef], length1: i32, which: i32, this: &Ph, fmt: &Fmt) {
        let factor;
        if which == 2 {
            let mut length_std = (fmt.std_length + self.seq_len_adjust - 45).max(10);
            if this.synthflags & SFLAG_LENGTHEN != 0 {
                length_std += self.phdata.get(PHON_LENGTHEN).map_or(0, |p| p.std_length as i32) * 2;
            }
            factor = (length_std * 256) / length1;
        } else {
            if which == 1 {
                if fmt.fmt_control == 1 && fmt.std_length < 130 {
                    frames[0].length = (frames[0].length * fmt.std_length) / 130;
                }
            } else if fmt.std_length > 0 {
                self.seq_len_adjust += fmt.std_length - length1;
            }
            if self.seq_len_adjust == 0 {
                return;
            }
            factor = ((length1 + self.seq_len_adjust) * 256) / length1;
        }
        for f in frames.iter_mut() {
            f.length = (f.length * factor) / 256;
        }
    }

    fn do_spect2(&mut self, list: &[Ph], ix: usize, which: i32, fmt: &mut Fmt, modulation: i32) -> i32 {
        if fmt.fmt_addr == 0 {
            return 0;
        }
        let this = &list[ix];
        let mut length_mod = if this.length == 0 { 256 } else { this.length };
        let mut length_min = SAMPLERATE / 70;
        if which == 2 {
            let thr = self.tr.long_vowel_threshold;
            if thr > 0 && (this.ph.std_length as i32 >= thr || this.synthflags & SFLAG_LENGTHEN != 0 || this.ph.phflags & PH_LONG != 0) {
                length_min *= 2;
            }
        }
        if which == 1 && (this.typ == PH_LIQUID || list[ix - 1].typ == PH_LIQUID || list[ix - 1].typ == PH_NASAL) {
            length_mod = length_mod.max(self.tr.sonorant_min);
        }
        self.modn_flags = 0;
        let frames = self.lookup_spect(this, which, fmt);
        if frames.is_empty() {
            return 0;
        }
        if fmt.fmt_amp != self.fmt_amplitude {
            self.fmt_amplitude = fmt.fmt_amp;
            self.push(Wcmd::FmtAmplitude(fmt.fmt_amp));
        }
        let mut spect2 = false;
        if fmt.wav_addr == 0 && self.wave_flag {
            spect2 = true;
            self.wave_flag = false;
        }
        self.link_last_frame(frames[0].frame);
        if this.typ == PH_VOWEL && which == 2 {
            self.smooth_spect();
            self.syllable_centre = Some(self.tail());
        }
        let lengths = self.frame_lengths(&frames, length_mod, length_min);
        self.emit_frames(&frames, &lengths, fmt, which, modulation, spect2)
    }

    fn link_last_frame(&mut self, frame1: usize) {
        let (Some(last), Some(lq)) = (self.last_frame, self.last_wcmdq) else { return };
        let lf = &self.frames[last];
        if !((lf.length < 2 || lf.frflags & FRFLAG_VOWEL_CENTRE != 0) && lf.frflags & FRFLAG_BREAK == 0) {
            return;
        }
        let mut target = frame1;
        if lf.frflags & FRFLAG_BREAK_LF != 0 {
            let fr = self.copy_frame(frame1, true);
            let src = self.frames[last].clone();
            let f = &mut self.frames[fr];
            for i in 3..8 {
                if i < 7 {
                    f.ffreq[i] = src.ffreq[i];
                }
                f.fheight[i] = src.fheight[i];
            }
            target = fr;
        }
        if let Some(Wcmd::Spect { fr2, .. }) = self.queue.get_mut(lq) {
            *fr2 = target;
        }
    }

    fn frame_lengths(&self, frames: &[FrameRef], length_mod: i32, length_min: i32) -> Vec<i32> {
        let mut lengths = vec![0i32; frames.len()];
        let mut sum = 0;
        for i in 1..frames.len() {
            let f = frames[i - 1];
            let factor = if f.frflags & FRFLAG_LEN_MOD != 0 {
                (length_mod * (256 - self.speed.lenmod_factor) + 256 * self.speed.lenmod_factor) / 256
            } else if f.frflags & FRFLAG_LEN_MOD2 != 0 {
                (length_mod * (256 - self.speed.lenmod2_factor) + 256 * self.speed.lenmod2_factor) / 256
            } else {
                length_mod
            };
            let len = (f.length * SAMPLERATE / 1000) * factor / 256;
            sum += len;
            lengths[i] = len;
        }
        if sum > 0 && sum < length_min {
            lengths.iter_mut().for_each(|l| *l = *l * length_min / sum);
        }
        lengths
    }

    fn emit_frames(&mut self, frames: &[FrameRef], lengths: &[i32], fmt: &mut Fmt, which: i32, modulation: i32, spect2: bool) -> i32 {
        let mut frame1 = frames[0].frame;
        let mut modulation = modulation;
        let mut total = 0;
        let n = frames.len();
        for frameix in 1..n {
            let frame2 = frames[frameix].frame;
            if fmt.wav_addr != 0 && self.frames[frame1].frflags & FRFLAG_DEFER_WAV == 0 {
                self.seq_len_adjust = 0;
                let amp = if fmt.wav_amp == 0 { 32 } else { fmt.wav_amp * 32 / 100 };
                self.do_sample2(fmt.wav_addr, which + 0x100, 0, fmt.fmt_control, 0, amp);
                self.wave_flag = true;
                fmt.wav_addr = 0;
            }
            if modulation >= 0 {
                if self.frames[frame1].frflags & FRFLAG_MODULATE != 0 {
                    modulation = 6;
                }
                if frameix == n - 1 && self.modn_flags & 0xf00 != 0 {
                    modulation |= self.modn_flags;
                }
            }
            let len = lengths[frameix];
            self.pitch_length += len;
            self.amp_length += len;
            if len == 0 {
                self.last_frame = None;
                frame1 = frame2;
                continue;
            }
            self.last_wcmdq = Some(self.tail());
            if modulation >= 0 {
                self.push(Wcmd::Spect { spect2, len, modn: modulation, fr1: frame1, fr2: frame2 });
            }
            self.last_frame = Some(frame2);
            frame1 = frame2;
            total += len;
        }
        if which != 1 && self.fmt_amplitude != 0 {
            self.fmt_amplitude = 0;
            self.push(Wcmd::FmtAmplitude(0));
        }
        total
    }

    // ---- SmoothSpect ------------------------------------------------------

    fn smooth_spect(&mut self) {
        if self.syllable_start == self.syllable_end {
            return;
        }
        let Some(centre) = self.syllable_centre.filter(|&c| c != self.syllable_start) else {
            self.syllable_start = self.syllable_end;
            return;
        };
        if let Some(Wcmd::Spect { fr1, .. }) = self.queue.get(centre).cloned() {
            self.smooth_backwards(centre, fr1);
        }
        self.smooth_forwards(centre);
        self.syllable_start = self.syllable_end;
    }

    fn limit_formants(&mut self, frame1: usize, frame: usize, len: i32, skip_lf: bool) -> Option<usize> {
        let mut copy: Option<usize> = None;
        for pk in 0..6 {
            if skip_lf && pk < 3 {
                continue;
            }
            let f1 = self.frames[frame1].ffreq[pk] as i32;
            let f2 = self.frames[frame].ffreq[pk] as i32;
            let diff = f2 - f1;
            let base = if diff > 0 { f1 * 2 + f2 } else { f1 + f2 * 2 };
            let allowed = (base * FORMANT_RATE[pk]) / 3000;
            let allowed = (allowed * len) / 256;
            if diff > allowed || diff < -allowed {
                let c = *copy.get_or_insert_with(|| self.copy_frame(frame, false));
                let v = if diff > allowed { f1 + allowed } else { f1 - allowed };
                self.frames[c].ffreq[pk] = v as i16;
            }
        }
        copy
    }

    fn smooth_backwards(&mut self, centre: usize, frame_centre: usize) {
        let mut frame = frame_centre;
        let mut frame2 = frame_centre;
        let mut ix = centre;
        while ix > 0 {
            ix -= 1;
            let cmd = self.queue[ix].clone();
            match cmd {
                Wcmd::Pause(_) | Wcmd::Wave { .. } => break,
                Wcmd::Spect { len, fr1, fr2, .. } => {
                    if fr2 != frame {
                        break;
                    }
                    self.set_spect(ix, None, Some(frame2));
                    let frame1 = frame2;
                    frame = fr1;
                    frame2 = fr1;
                    let flags = self.frames[frame].frflags;
                    if flags & FRFLAG_BREAK != 0 {
                        break;
                    }
                    let len = if flags & FRFLAG_FORMANT_RATE != 0 { len * 12 / 10 } else { len };
                    if let Some(c) = self.limit_formants(frame1, frame, len, flags & FRFLAG_BREAK_LF != 0) {
                        frame2 = c;
                        self.set_spect(ix, Some(c), None);
                    }
                }
                _ => {}
            }
            if ix == self.syllable_start {
                break;
            }
        }
    }

    fn smooth_forwards(&mut self, centre: usize) {
        let mut frame: Option<usize> = None;
        let mut frame2 = 0usize;
        let mut ix = centre;
        while ix < self.queue.len() {
            let cmd = self.queue[ix].clone();
            match cmd {
                Wcmd::Pause(_) | Wcmd::Wave { .. } => break,
                Wcmd::Spect { len, fr1, fr2, .. } => {
                    let mut frame1 = fr1;
                    if let Some(f) = frame {
                        if fr1 != f {
                            break;
                        }
                        self.set_spect(ix, Some(frame2), None);
                        frame1 = frame2;
                    }
                    frame = Some(fr2);
                    frame2 = fr2;
                    let flags = self.frames[frame1].frflags;
                    if flags & FRFLAG_BREAK != 0 {
                        break;
                    }
                    let len = if flags & FRFLAG_FORMANT_RATE != 0 { len * 6 / 5 } else { len };
                    if let Some(c) = self.limit_formants(frame1, fr2, len, false) {
                        frame2 = c;
                        self.set_spect(ix, None, Some(c));
                    }
                }
                _ => {}
            }
            ix += 1;
            if ix == self.syllable_end {
                break;
            }
        }
    }

    fn set_spect(&mut self, ix: usize, new_fr1: Option<usize>, new_fr2: Option<usize>) {
        if let Some(Wcmd::Spect { fr1, fr2, .. }) = self.queue.get_mut(ix) {
            if let Some(f) = new_fr1 {
                *fr1 = f;
            }
            if let Some(f) = new_fr2 {
                *fr2 = f;
            }
        }
    }

    fn start_syllable(&mut self) {
        if self.syllable_end == self.syllable_start {
            self.syllable_end = self.tail();
        }
    }

    // ---- InterpretPhoneme -------------------------------------------------

    fn interp(&self, list: &[Ph], ix: usize) -> PhData {
        self.interp_ctl(list, ix, false)
    }

    fn interp_ctl(&self, list: &[Ph], ix: usize, pre_voicing: bool) -> PhData {
        let p = &list[ix];
        let get = |i: usize| list.get(i);
        let code = |i: Option<&Ph>| i.map_or(0, |q| q.code());
        let stress = |i: Option<&Ph>| i.map_or(0, |q| q.stresslevel & 7);
        let nb = Neighbours {
            prev: code(ix.checked_sub(1).and_then(get)),
            this: p.code(),
            next: code(get(ix + 1)),
            next2: code(get(ix + 2)),
            stress: p.stresslevel & 7,
            prev_stress: stress(ix.checked_sub(1).and_then(get)),
            next_stress: stress(get(ix + 1)),
            next2_stress: stress(get(ix + 2)),
            wordstress: p.wordstress,
            this_wordstart: p.newword != 0,
            prev_wordstart: ix.checked_sub(1).and_then(get).is_some_and(|q| q.newword != 0),
            next_wordstart: get(ix + 1).is_some_and(|q| q.newword != 0),
            next2_wordstart: get(ix + 2).is_some_and(|q| q.newword != 0),
            this_next_pause: p.synthflags & SFLAG_NEXT_PAUSE != 0,
            pre_voicing,
            ..word_vowels(list, ix)
        };
        let window: Vec<&Ph> = (ix.saturating_sub(1)..(ix + 3).min(list.len())).map(|i| &list[i]).collect();
        let lookup = |c: u8| {
            window.iter().find(|q| q.code() == c).map(|q| q.ph.clone()).or_else(|| self.phdata.get(c).cloned())
        };
        let e = bytecode::interpret_phoneme(p.ph.program, &self.phdata.phonindex, &nb, lookup);
        PhData {
            sound_addr: e.sound_addr,
            sound_param: e.sound_param,
            set_length: e.set_length.map_or(p.ph.std_length as i32, |v| v as i32) + e.add_length as i32,
            pause_before: e.pause_before_ms as i32,
            pd_control: if e.pd_fornextph { PD_FORNEXTPH } else { 0 },
            vowel_transition: e.vowel_transition,
        }
    }

    // ---- Generate ---------------------------------------------------------

    /// `Generate` for one clause.
    pub fn generate(&mut self, list: &mut [Ph]) {
        self.syllable_start = self.tail();
        self.syllable_end = self.tail();
        self.do_pause(0, 0);
        for ix in 1..list.len() {
            if !list[ix].embedded.is_empty() {
                let cmds = list[ix].embedded.clone();
                self.do_embedded(&cmds);
            }
            self.phoneme(list, ix);
        }
        self.end_pitch(true);
    }

    /// `DoEmbedded`.
    pub fn do_embedded(&mut self, cmds: &[crate::translate::EmbeddedCmd]) {
        for c in cmds {
            let sign = match c.relative {
                1 => 0x40,
                -1 => 0x60,
                _ => 0,
            };
            if c.letter == crate::translate::ssml::BREAK_LETTER {
                if c.relative == 1 {
                    self.do_pause(c.value.max(0), 0);
                } else {
                    self.end_pitch(true);
                    self.push(Wcmd::Pause(c.value.max(0) * SAMPLERATE / 1000));
                    self.last_frame = None;
                }
                continue;
            }
            let Some(code) = super::embed_code(c.letter) else { continue };
            if code == super::wavegen::EMBED_S {
                self.embedded_s2 = super::apply_embedded(self.embedded_s2, sign, c.value, 2000);
                self.speed.set(2, self.embedded_s2, self.voice);
                continue;
            }
            self.do_pause(10, 0);
            self.push(Wcmd::Embedded { cmd: code as i32 + sign, value: c.value.max(0) });
        }
    }

    fn phoneme(&mut self, list: &mut [Ph], ix: usize) {
        let p = list[ix].clone();
        if p.newword != 0 {
            let merge = p.typ == PH_VOWEL && self.tr.word_merge & 1 != 0;
            if !(merge || p.ph.phflags & PH_NOPAUSE != 0) {
                self.last_frame = None;
            }
            if p.newword & PHLIST_START_OF_SENTENCE != 0 {
                self.push(Wcmd::Marker(MARKER_SENTENCE));
            }
            if p.newword & PHLIST_START_OF_WORD != 0 {
                self.push(Wcmd::Marker(MARKER_WORD));
            }
        }
        self.end_amplitude();
        if p.prepause > 0 && p.ph.phflags & PH_PREVOICE == 0 {
            self.do_pause(p.prepause, 1);
        }
        match p.typ {
            PH_PAUSE => self.do_pause(p.length, 0),
            PH_STOP => self.stop(list, ix),
            PH_FRICATIVE => {
                let d = self.interp(list, ix);
                if p.synthflags & SFLAG_LENGTHEN != 0 {
                    self.do_sample3(&d, p.length);
                }
                self.do_sample3(&d, p.length);
            }
            PH_VSTOP => self.vstop(list, ix),
            PH_VFRICATIVE => self.vfricative(list, ix),
            PH_NASAL => self.nasal(list, ix),
            PH_LIQUID => self.liquid(list, ix),
            PH_VOWEL => self.vowel(list, ix),
            _ => {}
        }
    }

    fn fmt_from(d: &PhData) -> Fmt {
        Fmt { fmt_addr: d.sound_addr[PD_FMT], fmt_amp: d.sound_param[PD_FMT], ..Default::default() }
    }

    fn stop(&mut self, list: &mut [Ph], ix: usize) {
        let next = list[ix + 1].clone();
        let released = next.typ == PH_VOWEL || (next.newword == 0 && next.typ == PH_LIQUID);
        if !released {
            list[ix].synthflags |= SFLAG_NEXT_PAUSE;
        }
        let p = list[ix].clone();
        if p.ph.phflags & PH_PREVOICE != 0 {
            let d = self.interp_ctl(list, ix, true);
            let mut fmt = Self::fmt_from(&d);
            if self.last_pitch_cmd.is_none() {
                self.do_amplitude(next.amp, None);
                self.do_pitch(Env::Builtin(p.env), next.pitch1, next.pitch2);
            }
            self.do_spect2(list, ix, 0, &mut fmt, 0);
        }
        let mut d = self.interp(list, ix);
        d.pd_control |= PD_DONTLENGTHEN;
        self.do_sample3(&d, 0);
    }

    fn pitch_from(&mut self, src: &Ph, amp_from: &Ph) {
        self.do_amplitude(amp_from.amp, None);
        self.do_pitch(Env::Builtin(src.env), src.pitch1, src.pitch2);
    }

    fn vstop(&mut self, list: &mut [Ph], ix: usize) {
        let (prev, p, next) = (list[ix - 1].clone(), list[ix].clone(), list[ix + 1].clone());
        let next2_newword = list.get(ix + 2).map_or(0, |q| q.newword);
        let mut pre_voiced = false;
        if next.typ == PH_VOWEL {
            self.pitch_from(&p, &p);
            pre_voiced = true;
        } else if next.typ == PH_LIQUID && next.newword == 0 {
            self.pitch_from(&next, &next);
            pre_voiced = true;
        } else if self.last_pitch_cmd.is_none() {
            self.do_amplitude(next.amp, None);
            self.do_pitch(Env::Builtin(p.env), p.pitch1, p.pitch2);
        }
        if prev.typ == PH_VOWEL || p.ph.phflags & PH_PREVOICE != 0 {
            let d = self.interp_ctl(list, ix, true);
            let mut fmt = Fmt { fmt_control: PD_DONTLENGTHEN, ..Self::fmt_from(&d) };
            self.do_spect2(list, ix, 0, &mut fmt, 0);
            if p.synthflags & SFLAG_LENGTHEN != 0 {
                self.do_pause(25, 1);
                self.do_spect2(list, ix, 0, &mut fmt, 0);
            }
        } else if p.synthflags & SFLAG_LENGTHEN != 0 {
            self.do_pause(50, 0);
        }
        if pre_voiced {
            self.start_syllable();
        } else {
            list[ix].synthflags |= SFLAG_NEXT_PAUSE;
        }
        let d = self.interp(list, ix);
        let mut fmt = Fmt {
            fmt_control: PD_DONTLENGTHEN,
            wav_addr: d.sound_addr[PD_ADDWAV],
            wav_amp: d.sound_param[PD_ADDWAV],
            ..Self::fmt_from(&d)
        };
        self.do_spect2(list, ix, 0, &mut fmt, 0);
        if p.newword == 0 && next2_newword == 0 {
            if next.typ == PH_VFRICATIVE {
                self.do_pause(20, 0);
            }
            if next.typ == PH_FRICATIVE {
                self.do_pause(12, 0);
            }
        }
    }

    fn vfricative(&mut self, list: &mut [Ph], ix: usize) {
        let (p, next) = (list[ix].clone(), list[ix + 1].clone());
        if next.typ == PH_VOWEL {
            self.pitch_from(&p, &p);
        } else if next.typ == PH_LIQUID {
            self.pitch_from(&next, &next);
        } else if self.last_pitch_cmd.is_none() {
            self.pitch_from(&p, &p);
        }
        if next.typ == PH_VOWEL || (next.typ == PH_LIQUID && next.newword == 0) {
            self.start_syllable();
        } else {
            list[ix].synthflags |= SFLAG_NEXT_PAUSE;
        }
        let d = self.interp(list, ix);
        let mut fmt = Fmt {
            std_length: d.set_length * 2,
            wav_addr: d.sound_addr[PD_ADDWAV],
            wav_amp: d.sound_param[PD_ADDWAV],
            ..Self::fmt_from(&d)
        };
        if p.synthflags & SFLAG_LENGTHEN != 0 {
            self.do_spect2(list, ix, 0, &mut fmt, 0);
        }
        self.do_spect2(list, ix, 0, &mut fmt, 0);
    }

    fn nasal(&mut self, list: &mut [Ph], ix: usize) {
        let (prev, p, next) = (list[ix - 1].clone(), list[ix].clone(), list[ix + 1].clone());
        if p.synthflags & SFLAG_SEQCONTINUE == 0 {
            self.pitch_from(&p, &p);
        }
        if prev.typ == PH_NASAL {
            self.last_frame = None;
        }
        let d = self.interp(list, ix);
        let mut fmt = Fmt { std_length: d.set_length * 2, ..Self::fmt_from(&d) };
        if next.typ == PH_VOWEL {
            self.start_syllable();
            self.do_spect2(list, ix, 0, &mut fmt, 0);
        } else if prev.typ == PH_VOWEL && p.synthflags & SFLAG_SEQCONTINUE != 0 {
            self.do_spect2(list, ix, 0, &mut fmt, 0);
        } else {
            self.last_frame = None;
            self.do_spect2(list, ix, 0, &mut fmt, 0);
            self.last_frame = None;
        }
    }

    fn liquid(&mut self, list: &mut [Ph], ix: usize) {
        let (prev, p, next) = (list[ix - 1].clone(), list[ix].clone(), list[ix + 1].clone());
        let modulation = if p.ph.phflags & PH_TRILL != 0 { 5 } else { 0 };
        if p.synthflags & SFLAG_SEQCONTINUE == 0 {
            self.pitch_from(&p, &p);
        }
        if prev.typ == PH_NASAL {
            self.last_frame = None;
        }
        if next.typ == PH_VOWEL {
            self.start_syllable();
        }
        let d = self.interp(list, ix);
        let value = d.pause_before - p.prepause;
        if value > 0 {
            self.do_pause(value, 1);
        }
        let mut fmt = Fmt {
            std_length: d.set_length * 2,
            wav_addr: d.sound_addr[PD_ADDWAV],
            wav_amp: d.sound_param[PD_ADDWAV],
            ..Self::fmt_from(&d)
        };
        self.do_spect2(list, ix, 0, &mut fmt, modulation);
    }

    fn vowel_start(&self, list: &[Ph], ix: usize, d: &PhData, fmt: &mut Fmt) -> bool {
        let prev = &list[ix - 1];
        let mut vowelstart_prev = false;
        fmt.fmt_addr = d.sound_addr[PD_VWLSTART];
        if fmt.fmt_addr != 0 && d.pd_control & PD_FORNEXTPH == 0 {
            fmt.fmt_length = d.sound_param[PD_VWLSTART];
        } else if prev.typ != PH_PAUSE {
            let dp = self.interp(list, ix - 1);
            fmt.fmt_addr = dp.sound_addr[PD_VWLSTART];
            if fmt.fmt_addr != 0 && dp.pd_control & PD_FORNEXTPH != 0 {
                vowelstart_prev = true;
                fmt.fmt2_lenadj = dp.sound_param[PD_VWLSTART];
            }
            fmt.transition0 = dp.vowel_transition[0];
            fmt.transition1 = dp.vowel_transition[1];
        }
        if fmt.fmt_addr == 0 {
            fmt.use_vowelin = true;
            fmt.fmt_control = 1;
            fmt.fmt_addr = d.sound_addr[PD_FMT];
        }
        vowelstart_prev
    }

    fn vowel(&mut self, list: &mut [Ph], ix: usize) {
        let (prev, p, next) = (list[ix - 1].clone(), list[ix].clone(), list[ix + 1].clone());
        let stress = (p.stresslevel & 0xf) as i32;
        let d = self.interp(list, ix);
        let mut fmt = Fmt { std_length: d.set_length * 2, ..Default::default() };
        let vowelstart_prev = self.vowel_start(list, ix, &d, &mut fmt);
        fmt.fmt_amp = d.sound_param[PD_FMT];
        let env = Env::Builtin(p.env);
        self.start_syllable();
        let modulation = if stress <= 1 { 1 } else if stress >= 7 { 3 } else { 2 };
        if prev.typ == PH_VSTOP || prev.typ == PH_VFRICATIVE {
            self.do_amplitude(p.amp, None);
            self.do_pitch(env, p.pitch1, p.pitch2);
            self.do_spect2(list, ix, 1, &mut fmt, modulation);
        } else if prev.typ == PH_LIQUID || prev.typ == PH_NASAL {
            self.do_amplitude(p.amp, None);
            self.do_spect2(list, ix, 1, &mut fmt, modulation);
            self.do_pitch(env, p.pitch1, p.pitch2);
        } else if vowelstart_prev {
            self.do_pitch(Env::Builtin(PITCH_RISE), p.pitch2 - 15, p.pitch2);
            self.do_amplitude(p.amp - 1, None);
            self.do_spect2(list, ix, 1, &mut fmt, modulation);
            self.do_pitch(env, p.pitch1, p.pitch2);
        } else {
            if p.synthflags & SFLAG_SEQCONTINUE == 0 {
                self.do_amplitude(p.amp, None);
                self.do_pitch(env, p.pitch1, p.pitch2);
            }
            self.do_spect2(list, ix, 1, &mut fmt, modulation);
        }
        let mut fmt2 = fmt;
        fmt2.fmt_addr = d.sound_addr[PD_FMT];
        fmt2.fmt_amp = d.sound_param[PD_FMT];
        fmt2.transition0 = 0;
        fmt2.transition1 = 0;
        fmt2.fmt2_addr = d.sound_addr[PD_VWLEND];
        if fmt2.fmt2_addr != 0 {
            fmt2.fmt2_lenadj = d.sound_param[PD_VWLEND];
        } else if next.typ != PH_PAUSE {
            fmt2.fmt2_lenadj = 0;
            let dn = self.interp(list, ix + 1);
            fmt2.use_vowelin = true;
            fmt2.transition0 = dn.vowel_transition[2];
            fmt2.transition1 = dn.vowel_transition[3];
            fmt2.fmt2_addr = dn.sound_addr[PD_VWLEND];
            if fmt2.fmt2_addr != 0 {
                fmt2.fmt2_lenadj = dn.sound_param[PD_VWLEND];
            }
        }
        self.do_spect2(list, ix, 2, &mut fmt2, modulation);
    }
}

/// The word-scoped vowel context `InterpretCondition` reads (`nextVowel`,
/// `prevVowel`, `isFirstVowel`, `isAfterStress`).
fn word_vowels(list: &[Ph], ix: usize) -> Neighbours {
    let start = (1..=ix).rev().find(|&i| list[i].newword != 0).unwrap_or(1);
    let end = (ix + 1..list.len()).find(|&i| list[i].newword != 0).unwrap_or(list.len());
    let is_v = |i: usize| list[i].typ == PH_VOWEL;
    let next = (ix + 1..end).find(|&i| is_v(i));
    let prev = (start..ix).rev().find(|&i| is_v(i));
    let position = (start..=ix).filter(|&i| is_v(i)).count();
    Neighbours {
        next_vowel: next.map_or(0, |i| list[i].code()),
        next_vowel_stress: next.map_or(bytecode::STRESS_UNKNOWN, |i| list[i].stresslevel & 7),
        prev_vowel: prev.map_or(0, |i| list[i].code()),
        prev_vowel_stress: prev.map_or(bytecode::STRESS_UNKNOWN, |i| list[i].stresslevel & 7),
        vowel_position: position.min(255) as u8,
        vowels_tracked: true,
        after_stress: (start..ix).any(|i| list[i].stresslevel & 7 >= 4),
        ..Default::default()
    }
}

struct Transition {
    f2: i32,
    f2_min: i32,
    f2_max: i32,
    f3_adj: i32,
    f3_amp: i32,
    f1: i32,
    vcolour: i32,
}

#[allow(dead_code)]
fn is_klatt(f: &SpectFrame) -> bool {
    f.frflags & FRFLAG_KLATT != 0
}
