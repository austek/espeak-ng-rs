//! A port of C espeak-ng's synthesis back end: `CalcPitches`, `CalcLengths`,
//! `Generate` and `wavegen.c`, driven clause by clause from the translator's
//! phoneme codes.

pub mod breath;
pub mod generate;
pub mod langtab;
pub mod lengths;
pub mod phlist;
pub mod pitches;
pub mod speed;
pub mod voice;
pub mod voicefile;
pub mod wavegen;

use std::path::Path;

use crate::phoneme::PhonemeData;
use crate::synthesize::intonation::{parse_tunes, Tune};
use crate::synthesize::VoiceParams;
use crate::translate::PhonemeCode;
use phlist::{Ph, SFLAG_EMBEDDED};
use speed::Speed;
use voice::{LangSynth, WVoice};
use wavegen::{Wavegen, EMBEDDED_DEFAULT, EMBED_A, EMBED_P, EMBED_S, EMBED_S2, N_EMBEDDED_VALUES};

pub const MARKER_WORD: u8 = 1;
pub const MARKER_SENTENCE: u8 = 2;

/// A pitch or amplitude envelope: one of the built-in `envelope_data[]`, or a
/// 128-byte table in `phondata`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Env {
    Builtin(usize),
    Data(usize),
}

/// One entry of C's `wcmdq`.  Frames are indices into the generator's arena.
#[derive(Clone, Debug)]
pub enum Wcmd {
    Pitch { len: i32, env: Env, p1: i32, p2: i32 },
    Amplitude { len: i32, env: Option<Env>, amp: i32 },
    Pause(i32),
    Wave { len: i32, addr: usize, scale: i32, amp: i32 },
    Wave2 { len: i32, max: i32, addr: usize, scale: i32, amp: i32 },
    Spect { spect2: bool, len: i32, modn: i32, fr1: usize, fr2: usize },
    FmtAmplitude(i32),
    Embedded { cmd: i32, value: i32 },
    Marker(u8),
}

/// `EMBED_*` number for an embedded command letter.
pub fn embed_code(letter: char) -> Option<usize> {
    Some(match letter {
        'P' => 1,
        'S' => 2,
        'A' => 3,
        'R' => 4,
        'H' => 5,
        'T' => 6,
        'B' => 12,
        'F' => 13,
        _ => return None,
    })
}

/// `SetEmbedded`'s value update: absolute, or relative by `sign` bits.
pub fn apply_embedded(cur: i32, sign: i32, value: i32, max: i32) -> i32 {
    let v = match sign {
        0x40 => cur + value,
        0x60 => cur - value,
        _ => value,
    };
    v.clamp(0, max)
}

/// Clause pause (ms), intonation type and whether a new sentence follows,
/// from the punctuation that ended the clause (`TranslateClause`).
fn clause_info(c: Option<char>, last: bool, no_final_pause: bool) -> (i32, usize, bool) {
    let (pause, tone, sentence) = match c {
        Some(',') => (200, 1, false),
        Some(';') => (300, 1, false),
        Some(':') => (300, 0, false),
        Some('?') => (400, 2, true),
        Some('!') => (450, 3, true),
        _ => (400, 0, true),
    };
    if last && no_final_pause {
        return (10, tone, sentence);
    }
    (pause, tone, sentence)
}

fn read_tunes_line(data_dir: &Path, lang: &str) -> Option<Vec<String>> {
    let entry = crate::voices::list_voices(data_dir).into_iter().find(|v| v.language.eq_ignore_ascii_case(lang))?;
    let text = std::fs::read_to_string(data_dir.join("voices").join(&entry.file))
        .or_else(|_| std::fs::read_to_string(data_dir.join("lang").join(&entry.file)))
        .ok()?;
    text.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        (it.next() == Some("tunes")).then(|| it.map(str::to_string).collect())
    })
}

/// The translator options for `lang`, with the voice file's overrides.
pub fn lang_synth(data_dir: &Path, voice: &VoiceParams, tunes: &[Tune]) -> LangSynth {
    let lang = voice.lang.as_str();
    let mut t = LangSynth::for_lang(lang);
    t.stress_flags = crate::dictionary::stress::StressOpts::for_lang_in(lang, data_dir).stress_flags;
    if let Some(l) = voice.stress_lengths {
        t.stress_lengths = l.map(|v| v as i32);
    }
    for (i, &a) in voice.stress_amps.iter().enumerate() {
        if a > 0 {
            t.stress_amps[i] = a;
        }
    }
    if let Some(g) = crate::voices::voice_intonation(data_dir, lang) {
        t.intonation_group = g as usize;
    }
    if let Some(names) = read_tunes_line(data_dir, lang) {
        t.intonation_group = 0;
        for (ix, n) in names.iter().take(6).enumerate() {
            if let Some(k) = tunes.iter().position(|tu| &tu.name == n) {
                t.tunes[ix] = k;
            }
        }
    }
    t
}

fn with_dictionary_flags(mut t: LangSynth, data_dir: &Path, lang: &str) -> LangSynth {
    t.stress_flags = crate::dictionary::stress::StressOpts::for_lang_in(lang, data_dir).stress_flags;
    t
}

/// The generator voice for these parameters (`LoadVoice` + variant).
pub fn wvoice(voice: &VoiceParams) -> WVoice {
    let mut v = WVoice::default();
    if let Some((p1, p2)) = voice.voice_pitch {
        v.set_pitch(p1, p2);
    }
    for i in 0..7 {
        let (f, h, w) = (voice.formant_freq_pct[i], voice.formant_height_pct[i], voice.formant_width_pct[i]);
        if (f, h, w) != (100, 100, 100) {
            v.set_formant(i, f, h, w, 0);
        }
    }
    if voice.echo_amp > 0 {
        v.echo_delay = (voice.echo_delay_samples as i32 * 1000) / 22050;
        v.echo_amp = voice.echo_amp;
    }
    if voice.flutter > 0 {
        v.flutter = voice.flutter * 32;
    }
    v.tone_adjust = voice.tone_adjust;
    v
}

fn speeds_along(list: &[Ph], wpm: &mut i32, voice: &WVoice) -> Vec<Speed> {
    let mut sp = Speed::default();
    sp.set(3, *wpm, voice);
    let mut out = Vec::with_capacity(list.len());
    for p in list {
        if p.synthflags & SFLAG_EMBEDDED != 0 {
            for c in p.embedded.iter().filter(|c| c.letter == 'S') {
                let sign = match c.relative {
                    1 => 0x40,
                    -1 => 0x60,
                    _ => 0,
                };
                *wpm = apply_embedded(*wpm, sign, c.value, 2000);
                sp.set(1, *wpm, voice);
            }
        }
        out.push(sp);
    }
    out
}

/// Renders clause after clause, keeping the C engine's state between them.
pub struct Renderer<'a> {
    phdata: &'a PhonemeData,
    voice: WVoice,
    tr: LangSynth,
    tunes: Vec<Tune>,
    wavegen: Wavegen,
    speed: Speed,
    wpm_s: i32,
    wpm_s2: i32,
    no_final_pause: bool,
    switched: std::collections::HashMap<String, PhonemeData>,
    emitted: usize,
    pub word_marks: Vec<usize>,
    pub sentence_marks: Vec<usize>,
}

impl<'a> Renderer<'a> {
    pub fn new(phdata: &'a PhonemeData, data_dir: &Path, params: &VoiceParams) -> Self {
        let tunes = parse_tunes(&phdata.intonations);
        let (tr, voice) = match voicefile::load(&params.voice_files, &tunes) {
            Some(l) => (with_dictionary_flags(l.tr, data_dir, &params.lang), l.voice),
            None => (lang_synth(data_dir, params, &tunes), wvoice(params)),
        };
        let mut ev: [i32; N_EMBEDDED_VALUES] = EMBEDDED_DEFAULT;
        let rate = params.user_rate_wpm as i32;
        ev[EMBED_S] = rate;
        ev[EMBED_S2] = rate;
        ev[EMBED_P] = params.user_pitch.min(99) as i32;
        ev[EMBED_A] = params.user_volume as i32;
        let mut speed = Speed::default();
        speed.set(3, rate, &voice);
        let wavegen = Wavegen::new(voice.clone(), ev, params.rand_seed);
        Renderer {
            phdata,
            voice,
            tr,
            tunes,
            wavegen,
            speed,
            wpm_s: rate,
            wpm_s2: rate,
            no_final_pause: params.no_final_pause,
            switched: std::collections::HashMap::new(),
            emitted: 0,
            word_marks: Vec::new(),
            sentence_marks: Vec::new(),
        }
    }

    /// Synthesize one clause; returns the samples it produced.
    /// The clause pause (ms) for an SSML `<break>`, as `ProcessSsmlTag` packs it.
    fn break_pause(&self, ms: i32, exact: bool) -> i32 {
        let mut value = if exact {
            let v = (ms * 256) / (self.speed.clause_pause_factor * 10);
            if v < 200 { (ms * 256) / (self.speed.pause_factor * 10) } else { v }
        } else {
            ms / 10
        };
        if value > 0xfff {
            value = (value / 32).min(0xfff) * 32;
        }
        value * 10
    }

    pub fn clause(&mut self, codes: &[PhonemeCode], clause_char: Option<char>, brk: Option<(i32, bool)>, start_sentence: bool, last: bool) -> Vec<i16> {
        let (mut post_pause, mut clause_type, _) = clause_info(clause_char, last, self.no_final_pause);
        if let Some((ms, exact)) = brk {
            post_pause = self.break_pause(ms, exact);
            clause_type = 4;
        }
        let mut c = phlist::build_list(codes, self.phdata, &self.switched, post_pause, start_sentence, self.tr.stress_flags);
        pitches::calc_pitches(&mut c.list, &self.tr, &self.tunes, clause_type);
        let speeds = speeds_along(&c.list, &mut self.wpm_s, &self.voice);
        lengths::calc_lengths(&mut c.list, &self.tr, &|i| speeds[i.min(speeds.len() - 1)]);
        self.dump(&c.list);
        let mut g = generate::Generator::new(self.phdata, &self.voice, &self.tr, self.speed, self.wpm_s2);
        g.generate(&mut c.list);
        if !c.embedded_after.is_empty() {
            g.do_embedded(&c.embedded_after);
        }
        self.speed = g.speed;
        self.wpm_s2 = g.embedded_s2_value();
        self.dump_queue(&g.queue, &g.frames);
        let base = self.emitted;
        let (words, sentences) = (&mut self.word_marks, &mut self.sentence_marks);
        let mut on_marker = |kind: u8, pos: usize| match kind {
            MARKER_WORD => words.push(base + pos),
            MARKER_SENTENCE => sentences.push(base + pos),
            _ => {}
        };
        self.wavegen.run(&g.queue, &g.frames, &self.phdata.phondata, &mut on_marker);
        if last {
            self.wavegen.finish();
        }
        let pcm = std::mem::take(&mut self.wavegen.out);
        self.emitted += pcm.len();
        pcm
    }

    fn dump_queue(&self, queue: &[Wcmd], frames: &[crate::synthesize::phondata::SpectFrame]) {
        if std::env::var_os("ESPEAK_RS_SPDUMP").is_none() {
            return;
        }
        for c in queue {
            match c {
                Wcmd::Spect { spect2, len, modn, fr1, fr2 } => {
                    let (a, b) = (&frames[*fr1], &frames[*fr2]);
                    eprintln!(
                        "Q {} len={} mod={} f={},{},{} h={},{},{} -> f={},{},{} h={},{},{}",
                        if *spect2 { 4 } else { 3 }, len, modn, a.ffreq[1], a.ffreq[2], a.ffreq[3], a.fheight[1], a.fheight[2], a.fheight[3],
                        b.ffreq[1], b.ffreq[2], b.ffreq[3], b.fheight[1], b.fheight[2], b.fheight[3]
                    );
                }
                Wcmd::Pitch { len, p1, p2, .. } => eprintln!("Q PITCH len={len} p1={p1} p2={p2}"),
                Wcmd::Amplitude { len, amp, env } => eprintln!("Q AMP len={len} amp={amp} env={}", env.is_some() as u8),
                Wcmd::Wave { len, scale, amp, .. } => eprintln!("Q WAVE len={len} scale={scale} amp={amp}"),
                Wcmd::Wave2 { len, scale, amp, max, .. } => eprintln!("Q WAVE2 len={} scale={scale} amp={amp}", len | (max << 16)),
                Wcmd::Pause(len) => eprintln!("Q PAUSE len={len}"),
                Wcmd::FmtAmplitude(a) => eprintln!("Q FMTAMP {a}"),
                Wcmd::Embedded { cmd, value } => eprintln!("Q cmd=12 {cmd} {value}"),
                Wcmd::Marker(_) => {}
            }
        }
    }

    fn dump(&self, list: &[Ph]) {
        if std::env::var_os("ESPEAK_RS_SPDUMP").is_none() {
            return;
        }
        for (i, p) in list.iter().enumerate() {
            eprintln!(
                "PH {} {} code={} type={} stress={} len={} env={} p1={} p2={} amp={} pre={} nw={} sf={:x}",
                i, p.ph.mnemonic_str(), p.code(), p.typ, p.stresslevel, p.length, p.env, p.pitch1, p.pitch2, p.amp, p.prepause, p.newword, p.synthflags
            );
        }
    }
}

/// The phoneme tables of the languages `phonSWITCH` markers name; a switched
/// word's phonemes are that table's, as C's `ph` pointers are.
fn switched_tables(codes: &[PhonemeCode], data_dir: &Path, own: &str) -> std::collections::HashMap<String, PhonemeData> {
    let mut out = std::collections::HashMap::new();
    for c in codes {
        let Some(crate::translate::CodeMarker::LangSwitch(lang)) = &c.marker else { continue };
        if lang == own || out.contains_key(lang) {
            continue;
        }
        let Ok(mut pd) = PhonemeData::load(data_dir) else { continue };
        if crate::translate::select_phoneme_table(&mut pd, data_dir, lang).is_ok() {
            out.insert(lang.clone(), pd);
        }
    }
    out
}

/// Synthesize a whole code stream; `sink` receives each clause as it is done.
pub fn render(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    data_dir: &Path,
    params: &VoiceParams,
    sink: &mut dyn FnMut(&[i16]),
) -> (Vec<usize>, Vec<usize>) {
    let clauses = phlist::split_clauses(codes);
    let mut r = Renderer::new(phdata, data_dir, params);
    r.switched = switched_tables(codes, data_dir, &params.lang);
    let mut start_sentence = true;
    let n = clauses.len();
    for (i, c) in clauses.iter().enumerate() {
        let pcm = r.clause(&c.codes, c.clause_char, c.brk, start_sentence, i + 1 == n);
        start_sentence = c.brk.is_none() && clause_info(c.clause_char, false, false).2;
        sink(&pcm);
    }
    (r.word_marks, r.sentence_marks)
}
