//! A port of C espeak-ng's synthesis back end: `CalcPitches`, `CalcLengths`,
//! `Generate` and `wavegen.c`, driven clause by clause from the translator's
//! phoneme codes.

pub mod breath;
pub mod generate;
pub mod klatt;
pub mod langtab;
pub mod lengths;
pub mod phlist;
pub mod pitches;
pub mod speed;
pub mod splayer;
pub mod tone;
pub mod voice;
pub mod voicefile;
pub mod wavegen;

use std::path::Path;

use crate::phoneme::PhonemeData;
use crate::synthesize::intonation::{parse_tunes, Tune};
use crate::synthesize::VoiceParams;
use crate::translate::PhonemeCode;
use phlist::{ClauseCodes, Ph, SFLAG_EMBEDDED};
use speed::Speed;
use voice::{LangSynth, WVoice};
use wavegen::{Wavegen, EMBEDDED_DEFAULT, EMBED_A, EMBED_P, EMBED_S, EMBED_S2, N_EMBEDDED_VALUES};

/// What a queued marker announces (`espeakEVENT_*` of `DoMarker`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Word,
    Sentence,
    /// `EMBED_M`: the value is the name's offset in the clause reader's name data.
    Mark,
    /// `EMBED_U`: the value is the audio URI's offset in the clause reader's name data.
    Play,
    /// End of a clause; the value is `count_sentences`.
    End,
    /// `WavegenSetVoice`; the value is the new voice's sample rate.
    SampleRate,
}

/// `DoMarker`'s payload: a source position, a word length and a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marker {
    pub kind: MarkerKind,
    pub pos: u32,
    pub len: u8,
    pub value: i32,
}

impl Marker {
    pub const fn new(kind: MarkerKind) -> Self {
        Marker { kind, pos: 0, len: 0, value: 0 }
    }
}

/// A marker the wave generator reached, with the message-wide sample offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawEvent {
    pub marker: Marker,
    pub sample: usize,
}

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
    Marker(Marker),
}

/// `EMBED_M`: a named marker.
pub const EMBED_M: usize = 10;

/// `EMBED_*` number for an embedded command letter.
pub fn embed_code(letter: char) -> Option<usize> {
    Some(match letter {
        'P' => 1,
        'S' => 2,
        'A' => 3,
        'R' => 4,
        'H' => 5,
        'T' => 6,
        'I' => 7,
        'Y' => 9,
        'M' => EMBED_M,
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

/// The translator options and generator voice a voice's files give.
fn load_voice(data_dir: &Path, params: &VoiceParams, tunes: &[Tune]) -> (LangSynth, WVoice) {
    match voicefile::load(&params.voice_files, tunes) {
        Some(l) => (with_dictionary_flags(l.tr, data_dir, &params.lang), l.voice),
        None => (lang_synth(data_dir, params, tunes), wvoice(params)),
    }
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
    data_dir: std::path::PathBuf,
    base_lang: String,
    active: Option<String>,
    pending_wvoice: Option<WVoice>,
    wave_flag: bool,
    icon_bytes: Vec<u8>,
    icon_clips: Vec<generate::IconClip>,
    voice: WVoice,
    tr: LangSynth,
    tunes: Vec<Tune>,
    wavegen: Wavegen,
    speed: Speed,
    wpm_s: i32,
    wpm_s2: i32,
    no_final_pause: bool,
    max_clause_pause: i32,
    switched: std::collections::HashMap<String, PhonemeData>,
    emitted: usize,
    pub events: Vec<RawEvent>,
}

impl<'a> Renderer<'a> {
    pub fn new(phdata: &'a PhonemeData, data_dir: &Path, params: &VoiceParams) -> Self {
        let tunes = parse_tunes(&phdata.intonations);
        let (tr, voice) = load_voice(data_dir, params, &tunes);
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
            data_dir: data_dir.to_path_buf(),
            base_lang: params.lang.clone(),
            active: None,
            pending_wvoice: None,
            wave_flag: false,
            icon_bytes: Vec::new(),
            icon_clips: Vec::new(),
            voice,
            tr,
            tunes,
            wavegen,
            speed,
            wpm_s: rate,
            wpm_s2: rate,
            no_final_pause: params.no_final_pause,
            max_clause_pause: 0,
            switched: std::collections::HashMap::new(),
            emitted: 0,
            events: Vec::new(),
        }
    }

    /// Sample offsets of the word markers, in queue order.
    pub fn word_marks(&self) -> Vec<usize> {
        self.marks(MarkerKind::Word)
    }

    /// Sample offsets of the sentence markers, in queue order.
    pub fn sentence_marks(&self) -> Vec<usize> {
        self.marks(MarkerKind::Sentence)
    }

    fn marks(&self, kind: MarkerKind) -> Vec<usize> {
        self.events.iter().filter(|e| e.marker.kind == kind).map(|e| e.sample).collect()
    }

    /// `TranslateClause`'s pause: a clause with no words only makes up the
    /// previous pause, and the last clause of the text without words, or with
    /// no end pause wanted, keeps just 10 ms.
    fn clause_pause(&mut self, pause: i32, empty: bool, last: bool) -> i32 {
        let pause = if empty {
            let made_up = (pause - self.max_clause_pause).max(0);
            self.max_clause_pause += made_up;
            made_up
        } else {
            self.max_clause_pause = pause;
            pause
        };
        if last && (empty || self.no_final_pause) { 10 } else { pause }
    }

    /// The sound-icon table (`soundicon_tab`): 16-bit clips `EMBED_I` plays by index.
    pub fn set_icons(&mut self, clips: &[Vec<i16>]) {
        self.icon_bytes = clips.iter().flatten().flat_map(|s| s.to_le_bytes()).collect();
        let mut offset = 0;
        self.icon_clips = clips
            .iter()
            .map(|c| {
                let clip = generate::IconClip { offset, len: c.len() as i32 };
                offset += c.len() * 2;
                clip
            })
            .collect();
    }

    /// `SpeakNextClause` for a clause that ends in a voice change (`LoadVoiceVariant`,
    /// `DoVoiceChange`): the next voice is loaded before the clause reaches the generator and
    /// queued behind it, so the wavegen switches before the echo tail plays out. The next clause
    /// uses the new tables and formants; the embedded rate, pitch and volume stay.
    pub fn clause_then_voice(&mut self, clause: &ClauseCodes, start_sentence: bool, next: &VoiceParams) -> Vec<i16> {
        let loaded = load_voice(&self.data_dir, next, &self.tunes);
        self.wavegen.set_global_roughness(loaded.1.roughness);
        self.pending_wvoice = Some(loaded.1.clone());
        let pcm = self.clause(clause, start_sentence);
        self.install_voice(next, loaded);
        pcm
    }

    fn install_voice(&mut self, params: &VoiceParams, (tr, voice): (LangSynth, WVoice)) {
        let sets_speed = voice.speed_percent != WVoice::default().speed_percent;
        self.select_table(&params.lang);
        self.tr = tr;
        self.voice = voice;
        if sets_speed {
            self.speed.set(3, self.wpm_s, &self.voice);
        }
    }

    /// `SelectPhonemeTable(voice->phoneme_tab_ix)`: the table of `lang` serves the clauses
    /// that follow; the base language keeps the table the renderer was given.
    fn select_table(&mut self, lang: &str) {
        if lang == self.base_lang {
            self.active = None;
            return;
        }
        if !self.switched.contains_key(lang) {
            let Ok(mut pd) = PhonemeData::load(&self.data_dir) else { return };
            if crate::translate::select_phoneme_table(&mut pd, &self.data_dir, lang).is_err() {
                return;
            }
            self.switched.insert(lang.to_string(), pd);
        }
        self.active = Some(lang.to_string());
    }

    pub fn clause(&mut self, clause: &ClauseCodes, start_sentence: bool) -> Vec<i16> {
        let codes = &clause.codes;
        let base_pause = clause.terminator.pause_ms() as i32;
        let clause_type = clause.terminator.intonation() as usize;
        let empty = codes
            .iter()
            .all(|c| c.marker.is_none() && (c.code == 0 || c.code == crate::phoneme::PHON_END_WORD));
        let post_pause = self.clause_pause(base_pause, empty, clause.eof);
        let phdata = active_table(self.phdata, &self.switched, self.active.as_deref());
        let mut c = phlist::build_list(codes, phdata, &self.switched, post_pause, start_sentence, self.tr.stress_flags);
        if let Some(k) = clause.counters {
            let end = c.list.len().saturating_sub(2);
            c.list[end].source = Some(crate::translate::SourceIx { ix: k.end_ix, len: 0 });
        }
        pitches::calc_pitches(&mut c.list, &self.tr, &self.tunes, clause_type, phdata);
        let speeds = speeds_along(&c.list, &mut self.wpm_s, &self.voice);
        lengths::calc_lengths(&mut c.list, &self.tr, &|i| speeds[i.min(speeds.len() - 1)]);
        self.dump(&c.list);
        let mut g = generate::Generator::new(phdata, &self.voice, &self.tr, self.speed, self.wpm_s2)
            .with_icons(&self.icon_clips)
            .with_wave_flag(self.wave_flag)
            .with_counters(clause.counters);
        g.generate(&mut c.list);
        self.speed = g.speed;
        self.wpm_s2 = g.embedded_s2_value();
        self.wave_flag = g.wave_flag();
        self.dump_queue(&g.queue, &g.frames);
        let base = self.emitted;
        let events = &mut self.events;
        let mut on_marker = |marker: &Marker, pos: usize| events.push(RawEvent { marker: *marker, sample: base + pos });
        let with_icons: Vec<u8>;
        let wave_data: &[u8] = if self.icon_bytes.is_empty() {
            &phdata.phondata
        } else {
            with_icons = [phdata.phondata.as_slice(), self.icon_bytes.as_slice()].concat();
            &with_icons
        };
        self.wavegen.run(&g.queue, &g.frames, wave_data, &mut on_marker);
        if let Some(next) = self.pending_wvoice.take() {
            self.wavegen.change_voice(next);
            let marker = Marker { kind: MarkerKind::SampleRate, pos: 0, len: 0, value: wavegen::SAMPLERATE };
            self.events.push(RawEvent { marker, sample: base + self.wavegen.out.len() });
        }
        self.wavegen.finish();
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
                "PH {} {} code={} type={} stress={} len={} env={} p1={} p2={} amp={} pre={} nw={} sf={:x} tone={}",
                i, p.ph.mnemonic_str(), p.code(), p.typ, p.stresslevel, p.length, p.env, p.pitch1, p.pitch2, p.amp, p.prepause, p.newword, p.synthflags,
                p.tone.as_ref().map_or("0".to_string(), |t| format!("{}:{}:{}-{}", t.code, t.mnemonic_str(), t.start_type, t.end_type))
            );
        }
    }
}

fn active_table<'t>(
    base: &'t PhonemeData,
    tables: &'t std::collections::HashMap<String, PhonemeData>,
    active: Option<&str>,
) -> &'t PhonemeData {
    active.and_then(|lang| tables.get(lang)).unwrap_or(base)
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

/// Synthesize clauses as the SSML reader ended them (C `SpeakNextClause`): each keeps its
/// terminator and end-of-text flag, and the next clause starts a sentence only when this one
/// ended one.
pub fn render_clauses(
    clauses: &[ClauseCodes],
    phdata: &PhonemeData,
    data_dir: &Path,
    params: &VoiceParams,
    voices: &std::collections::HashMap<String, VoiceParams>,
    icons: &[Vec<i16>],
    sink: &mut dyn FnMut(&[i16]),
) -> Vec<RawEvent> {
    let mut r = Renderer::new(phdata, data_dir, params);
    r.set_icons(icons);
    let all: Vec<PhonemeCode> = clauses.iter().flat_map(|c| c.codes.iter().cloned()).collect();
    r.switched = switched_tables(&all, data_dir, &params.lang);
    let mut start_sentence = true;
    for c in clauses {
        let opens_sentence = c.counters.map_or(start_sentence, |k| k.new_sentence);
        let pcm = match c.voice_after.as_ref().and_then(|id| voices.get(id)) {
            Some(next) => r.clause_then_voice(c, opens_sentence, next),
            None => r.clause(c, opens_sentence),
        };
        start_sentence = c.terminator.is_sentence();
        sink(&pcm);
    }
    r.events
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
    for c in &clauses {
        let pcm = r.clause(c, start_sentence);
        start_sentence = c.terminator.is_sentence();
        sink(&pcm);
    }
    (r.word_marks(), r.sentence_marks())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthesize::Synthesizer;

    fn hello_world_events() -> Option<(Vec<RawEvent>, Vec<usize>, Vec<usize>)> {
        let dir = Path::new("espeak-ng-data");
        if !dir.join("phontab").exists() {
            return None;
        }
        let tr = crate::translate::Translator::new("en", Some(dir)).ok()?;
        let mut phdata = PhonemeData::load(dir).ok()?;
        crate::translate::select_phoneme_table(&mut phdata, dir, "en").ok()?;
        let codes = tr.translate_to_codes("hello world").ok()?;
        let params = VoiceParams { lang: "en".into(), data_dir: Some(dir.to_path_buf()), ..VoiceParams::default() };
        let mut r = Renderer::new(&phdata, dir, &params);
        for c in phlist::split_clauses(&codes) {
            r.clause(&c, true);
        }
        let (_, words, sentences) = Synthesizer::new(params).synthesize_codes_with_marks(&codes, &phdata).ok()?;
        Some((r.events, words, sentences))
    }

    #[test]
    fn hello_world_queues_sentence_then_two_words() {
        let Some((events, words, sentences)) = hello_world_events() else { return };
        let kinds: Vec<MarkerKind> = events.iter().map(|e| e.marker.kind).collect();
        assert_eq!(kinds, [MarkerKind::Sentence, MarkerKind::Word, MarkerKind::Word]);
        let samples: Vec<usize> = events.iter().map(|e| e.sample).collect();
        assert_eq!(samples[0], 0);
        assert_eq!(&samples[1..], words.as_slice());
        assert!(sentences.is_empty());
    }
}
