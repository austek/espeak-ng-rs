//! Port of `ProcessSsmlTag` (`ssml.c`) with the param stack, the prosody
//! mnemonic tables and `ParseSsmlReference`.
//!
//! Terminators are the raw `u32` C values (`translate.h`); 0 means "no clause
//! end, keep reading".

use super::attrs::{attr_copy, attr_eq, attr_lookup, attr_number, get_ssml_attribute, is_space, prosody_value, AttrVal};
use super::voice_stack::{BaseVoice, VoiceCatalog, VoiceStack, TAG_CLOSE};

pub const N_SPEECH_PARAM: usize = 15;
pub const N_PARAM_STACK: usize = 20;
pub const P_SILENCE: usize = 0;
pub const P_RATE: usize = 1;
pub const P_VOLUME: usize = 2;
pub const P_PITCH: usize = 3;
pub const P_RANGE: usize = 4;
pub const P_PUNCTUATION: usize = 5;
pub const P_CAPITALS: usize = 6;
pub const P_SSML_BREAK_MUL: usize = 10;
pub const P_EMPHASIS: usize = 12;

/// `CLAUSE_*` terminators from `translate.h`.
pub mod clause {
    pub const PAUSE_LONG: u32 = 0x0080_0000;
    pub const NONE: u32 = 0x4000;
    pub const PARAGRAPH: u32 = 70 | 0x0008_0000;
    pub const VOICE: u32 = 0x4000 | 0x0002_0000;
    pub const PERIOD: u32 = 40 | 0x0008_0000;
    pub const COLON: u32 = 30 | 0x0004_0000;
}

/// `SAYAS_*` modes from `translate.h`.
pub mod sayas {
    pub const CHARS: i32 = 0x12;
    pub const GLYPHS: i32 = 0x13;
    pub const SINGLE_CHARS: i32 = 0x14;
    pub const KEY: i32 = 0x24;
    pub const DIGITS: i32 = 0x40;
    pub const DIGITS1: i32 = 0xc1;
}

pub const SSML_SPEAK: u32 = 1;
pub const SSML_VOICE: u32 = 2;
pub const SSML_PROSODY: u32 = 3;
pub const SSML_SAYAS: u32 = 4;
pub const SSML_MARK: u32 = 5;
pub const SSML_SENTENCE: u32 = 6;
pub const SSML_PARAGRAPH: u32 = 7;
pub const SSML_PHONEME: u32 = 8;
pub const SSML_SUB: u32 = 9;
pub const SSML_STYLE: u32 = 10;
pub const SSML_AUDIO: u32 = 11;
pub const SSML_EMPHASIS: u32 = 12;
pub const SSML_BREAK: u32 = 13;
pub const SSML_IGNORE_TEXT: u32 = 14;
pub const HTML_BREAK: u32 = 15;
pub const HTML_NOSPACE: u32 = 16;

const TAGS: [(&str, u32); 31] = [
    ("speak", SSML_SPEAK), ("voice", SSML_VOICE), ("prosody", SSML_PROSODY), ("say-as", SSML_SAYAS),
    ("mark", SSML_MARK), ("s", SSML_SENTENCE), ("p", SSML_PARAGRAPH), ("phoneme", SSML_PHONEME),
    ("sub", SSML_SUB), ("tts:style", SSML_STYLE), ("audio", SSML_AUDIO), ("emphasis", SSML_EMPHASIS),
    ("break", SSML_BREAK), ("metadata", SSML_IGNORE_TEXT), ("br", HTML_BREAK), ("li", HTML_BREAK),
    ("dd", HTML_BREAK), ("img", HTML_BREAK), ("td", HTML_BREAK), ("h1", SSML_PARAGRAPH),
    ("h2", SSML_PARAGRAPH), ("h3", SSML_PARAGRAPH), ("h4", SSML_PARAGRAPH), ("hr", SSML_PARAGRAPH),
    ("script", SSML_IGNORE_TEXT), ("style", SSML_IGNORE_TEXT), ("font", HTML_NOSPACE),
    ("b", HTML_NOSPACE), ("i", HTML_NOSPACE), ("strong", HTML_NOSPACE), ("em", HTML_NOSPACE),
];

/// `ignore_if_self_closing`, indexed by tag type (`code` shares `HTML_NOSPACE`).
const IGNORE_IF_SELF_CLOSING: [bool; 19] = [
    false, true, true, true, true, false, false, false, false, true, true, false, true, false, true,
    false, false, false, false,
];

const MNEM_VOLUME: [(&str, i32); 7] =
    [("default", 100), ("silent", 0), ("x-soft", 30), ("soft", 65), ("medium", 100), ("loud", 150), ("x-loud", 230)];
const MNEM_RATE: [(&str, i32); 6] =
    [("default", 100), ("x-slow", 60), ("slow", 80), ("medium", 100), ("fast", 125), ("x-fast", 160)];
const MNEM_PITCH: [(&str, i32); 6] =
    [("default", 100), ("x-low", 70), ("low", 85), ("medium", 100), ("high", 110), ("x-high", 120)];
const MNEM_RANGE: [(&str, i32); 6] =
    [("default", 100), ("x-low", 20), ("low", 50), ("medium", 100), ("high", 140), ("x-high", 180)];
const MNEM_PUNCT: [(&str, i32); 3] = [("none", 1), ("all", 2), ("some", 3)];
const MNEM_CAPITALS: [(&str, i32); 4] = [("no", 0), ("icon", 1), ("spelling", 2), ("pitch", 20)];
const MNEM_INTERPRET_AS: [(&str, i32); 5] = [
    ("characters", sayas::CHARS),
    ("tts:char", sayas::SINGLE_CHARS),
    ("tts:key", sayas::KEY),
    ("tts:digits", sayas::DIGITS),
    ("telephone", sayas::DIGITS1),
];
const MNEM_BREAK: [(&str, i32); 6] =
    [("none", 0), ("x-weak", 1), ("weak", 2), ("medium", 3), ("strong", 4), ("x-strong", 5)];
const MNEM_EMPHASIS: [(&str, i32); 5] =
    [("none", 1), ("reduced", 2), ("moderate", 3), ("strong", 4), ("x-strong", 5)];
const BREAK_VALUE: [i32; 6] = [0, 7, 14, 21, 40, 80];
const KEY_NAMES: [(&str, char); 4] =
    [("space ", '\u{e020}'), ("tab ", '\u{e009}'), ("underscore ", '\u{e05f}'), ("double-quote ", '"')];
const EMPHASIS_RANGE: [i32; 6] = [50, 50, 40, 70, 90, 100];
const EMPHASIS_VOLUME_TONE: [i32; 6] = [100, 100, 70, 110, 135, 150];
const EMPHASIS_VOLUME: [i32; 6] = [100, 100, 75, 100, 120, 150];

/// C `PARAM_STACK`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamFrame {
    pub tag_type: u32,
    pub parameter: [i32; N_SPEECH_PARAM],
}

/// C `namedata`: names are NUL-terminated and addressed by byte offset.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameData {
    bytes: Vec<u8>,
}

impl NameData {
    pub fn add(&mut self, name: &str) -> usize {
        let ix = self.bytes.len();
        self.bytes.extend_from_slice(name.as_bytes());
        self.bytes.push(0);
        ix
    }

    pub fn get(&self, ix: usize) -> Option<&str> {
        let tail = self.bytes.get(ix..)?;
        let end = tail.iter().position(|&b| b == 0)?;
        std::str::from_utf8(&tail[..end]).ok()
    }
}

/// What the tag processor needs from outside.
pub struct SsmlEnv<'a> {
    pub voices: &'a dyn VoiceCatalog,
    /// `translator->langopts.tone_language == 1`.
    pub tone_language: bool,
    /// Size of the clause buffer (`n_outbuf`).
    pub n_outbuf: usize,
    /// `SetSpeed` for a rate: `(pause_factor, clause_pause_factor)`.
    pub pause_factors: &'a dyn Fn(i32) -> (i32, i32),
    /// `LoadSoundFile2`: sound-icon index of a file.
    pub load_sound: &'a dyn Fn(&str) -> Option<i32>,
    /// `uri_callback(1, uri, xmlbase)`; 0 accepts the audio.
    pub uri_callback: Option<&'a UriCallback>,
}

pub type UriCallback = dyn Fn(&str, Option<&str>) -> i32;

/// Everything `ProcessSsmlTag` reads and writes through pointers or statics.
#[derive(Clone, Debug)]
pub struct SsmlState {
    pub voice: VoiceStack,
    pub param_stack: Vec<ParamFrame>,
    pub n_param_stack: usize,
    pub speech_parameters: [i32; N_SPEECH_PARAM],
    pub sayas_mode: i32,
    pub sayas_start: usize,
    pub ignore_text: bool,
    pub audio_text: bool,
    pub clear_skipping_text: bool,
    pub skip_marker: String,
    pub xmlbase: Option<String>,
    pub names: NameData,
    pub option_punctuation: i32,
    pub option_capitals: i32,
}

impl SsmlState {
    /// `InitText2` with `param_stack[0]` holding `defaults`.
    pub fn new(defaults: [i32; N_SPEECH_PARAM], base: BaseVoice, base_variant: &str) -> SsmlState {
        let blank = ParamFrame { tag_type: 0, parameter: [-1; N_SPEECH_PARAM] };
        let mut param_stack = vec![blank; N_PARAM_STACK];
        param_stack[0].parameter = defaults;
        SsmlState {
            voice: VoiceStack::new(base, base_variant),
            param_stack,
            n_param_stack: 1,
            speech_parameters: defaults,
            sayas_mode: 0,
            sayas_start: 0,
            ignore_text: false,
            audio_text: false,
            clear_skipping_text: false,
            skip_marker: String::new(),
            xmlbase: None,
            names: NameData::default(),
            option_punctuation: defaults[P_PUNCTUATION],
            option_capitals: defaults[P_CAPITALS],
        }
    }

    fn push_param(&mut self, tag_type: u32) -> usize {
        let ix = self.n_param_stack;
        let ix = ix.min(N_PARAM_STACK - 1);
        if self.n_param_stack < N_PARAM_STACK - 1 {
            self.n_param_stack += 1;
        }
        self.param_stack[ix] = ParamFrame { tag_type, parameter: [-1; N_SPEECH_PARAM] };
        ix
    }

    fn pop_param(&mut self, tag_type: u32, out: &mut String) {
        let tag_type = tag_type & !TAG_CLOSE;
        let top = (0..self.n_param_stack).rfind(|&ix| self.param_stack[ix].tag_type == tag_type);
        if let Some(top) = top.filter(|&t| t > 0) {
            self.n_param_stack = top;
        }
        self.process_params(out);
    }

    fn process_params(&mut self, out: &mut String) {
        let mut new = [-1; N_SPEECH_PARAM];
        for frame in &self.param_stack[..self.n_param_stack] {
            for (slot, &v) in new.iter_mut().zip(&frame.parameter) {
                if v >= 0 {
                    *slot = v;
                }
            }
        }
        for param in 0..N_SPEECH_PARAM {
            if new[param] != self.speech_parameters[param] {
                self.apply_param(param, new[param], out);
                self.speech_parameters[param] = new[param];
            }
        }
    }

    fn apply_param(&mut self, param: usize, value: i32, out: &mut String) {
        let letter = match param {
            P_RATE => 'S',
            P_VOLUME => 'A',
            P_PITCH => 'P',
            P_RANGE => 'R',
            P_EMPHASIS => 'F',
            P_PUNCTUATION => return self.option_punctuation = value - 1,
            P_CAPITALS => return self.option_capitals = value,
            _ => return,
        };
        out.push_str(&format!("\u{1}{value}{letter}"));
    }

    fn set_prosody(&mut self, sp: usize, param: usize, attr: &AttrVal) {
        let table: &[(&str, i32)] = match param {
            P_RATE => &MNEM_RATE,
            P_VOLUME => &MNEM_VOLUME,
            P_PITCH => &MNEM_PITCH,
            _ => &MNEM_RANGE,
        };
        let value = attr_lookup(Some(attr), table, -1);
        if value >= 0 {
            self.param_stack[sp].parameter[param] = self.param_stack[0].parameter[param].wrapping_mul(value) / 100;
            return;
        }
        let (sign, value) = prosody_value(param == P_RATE, attr);
        let current = self.speech_parameters[param];
        self.param_stack[sp].parameter[param] = match sign {
            0 => value,
            2 => current.wrapping_mul(value) / 100,
            _ => current.wrapping_add(value.wrapping_mul(sign)),
        };
    }
}

type AttrFn<'a> = dyn Fn(&str) -> Option<AttrVal<'a>> + 'a;

fn lookup_tag(name: &[u8]) -> u32 {
    TAGS.iter().find(|(n, _)| n.as_bytes() == name).map_or(0, |&(_, t)| t)
}

fn tag_name_of(tag: &[char]) -> (Vec<u8>, usize) {
    let mut name = Vec::new();
    for &c in tag.iter().take(39) {
        if c == '\0' || is_space(c) {
            break;
        }
        name.push((c as u32 as u8).to_ascii_lowercase());
    }
    let ix = name.len();
    (name, ix)
}

/// `ProcessSsmlTag`: `tag` is the text between `<` and `>`. Appends the tag's
/// text and embedded commands to `out` and returns the clause terminator.
pub fn process_ssml_tag(st: &mut SsmlState, env: &SsmlEnv, tag: &[char], out: &mut String) -> u32 {
    let mut buf: Vec<char> = tag.iter().copied().take_while(|&c| c != '\0').collect();
    let self_closing = buf.last() == Some(&'/');
    if self_closing {
        *buf.last_mut().unwrap_or(&mut ' ') = ' ';
    }
    let (name, px) = tag_name_of(&buf);
    let closing = name.first() == Some(&b'/');
    let base = lookup_tag(if closing { &name[1..] } else { &name });
    if base != HTML_NOSPACE {
        out.push(' ');
    }
    if !closing && self_closing && IGNORE_IF_SELF_CLOSING.get(base as usize).copied().unwrap_or(false) {
        return 0;
    }
    let tag_type = if closing { base + TAG_CLOSE } else { base };
    dispatch(st, env, &buf, px, tag_type, self_closing, out)
}

fn dispatch(st: &mut SsmlState, env: &SsmlEnv, buf: &[char], px: usize, tag_type: u32, self_closing: bool, out: &mut String) -> u32 {
    let attr = |name: &str| get_ssml_attribute(buf, px, name);
    match tag_type {
        SSML_STYLE => style_open(st, &attr, out),
        SSML_PROSODY => prosody_open(st, &attr, out),
        SSML_EMPHASIS => emphasis_open(st, env, &attr, out),
        t if t == SSML_STYLE + TAG_CLOSE || t == SSML_PROSODY + TAG_CLOSE || t == SSML_EMPHASIS + TAG_CLOSE => {
            st.pop_param(t, out);
        }
        SSML_PHONEME => phoneme(env, &attr, out),
        SSML_SAYAS => sayas_open(st, &attr, out),
        t if t == SSML_SAYAS + TAG_CLOSE => sayas_close(st, out),
        SSML_SUB => sub_open(st, env, &attr, out),
        SSML_IGNORE_TEXT => st.ignore_text = true,
        t if t == SSML_SUB + TAG_CLOSE || t == SSML_IGNORE_TEXT + TAG_CLOSE => st.ignore_text = false,
        SSML_MARK => return mark(st, &attr, out),
        SSML_AUDIO => return audio_open(st, env, &attr, self_closing, out),
        t if t == SSML_AUDIO + TAG_CLOSE => {
            st.pop_param(t, out);
            st.audio_text = false;
            return clause::NONE;
        }
        SSML_BREAK => return break_tag(st, env, &attr, out),
        _ => return voice_tags(st, env, buf, px, tag_type),
    }
    0
}

fn style_open(st: &mut SsmlState, attr: &AttrFn<'_>, out: &mut String) {
    let sp = st.push_param(SSML_STYLE);
    let (field, mode) = (attr("field"), attr("mode"));
    if attr_eq(field.as_ref(), "punctuation") {
        st.param_stack[sp].parameter[P_PUNCTUATION] = attr_lookup(mode.as_ref(), &MNEM_PUNCT, -1);
    } else if attr_eq(field.as_ref(), "capital_letters") {
        st.param_stack[sp].parameter[P_CAPITALS] = attr_lookup(mode.as_ref(), &MNEM_CAPITALS, -1);
    }
    st.process_params(out);
}

fn prosody_open(st: &mut SsmlState, attr: &AttrFn<'_>, out: &mut String) {
    let sp = st.push_param(SSML_PROSODY);
    for (param, name) in [(P_RATE, "rate"), (P_VOLUME, "volume"), (P_PITCH, "pitch"), (P_RANGE, "range")] {
        if let Some(a) = attr(name) {
            st.set_prosody(sp, param, &a);
        }
    }
    st.process_params(out);
}

fn emphasis_open(st: &mut SsmlState, env: &SsmlEnv, attr: &AttrFn<'_>, out: &mut String) {
    let sp = st.push_param(SSML_EMPHASIS);
    let level = attr("level").map_or(3, |a| attr_lookup(Some(&a), &MNEM_EMPHASIS, -1));
    let level = if level < 0 { 3 } else { level as usize };
    let frame = &mut st.param_stack[sp].parameter;
    if env.tone_language {
        frame[P_RANGE] = EMPHASIS_RANGE[level];
        frame[P_VOLUME] = EMPHASIS_VOLUME_TONE[level];
    } else {
        frame[P_VOLUME] = EMPHASIS_VOLUME[level];
        frame[P_EMPHASIS] = level as i32;
    }
    st.process_params(out);
}

fn phoneme(env: &SsmlEnv, attr: &AttrFn<'_>, out: &mut String) {
    let alphabet = attr("alphabet");
    if attr_lookup(alphabet.as_ref(), &[("espeak", 1)], -1) == 1 {
        out.push_str("[[");
        out.push_str(&attr_copy(attr("ph").as_ref(), env.n_outbuf.saturating_sub(out.len())));
        out.push_str("]]");
    }
}

fn sayas_open(st: &mut SsmlState, attr: &AttrFn<'_>, out: &mut String) {
    let mut value = attr_lookup(attr("interpret-as").as_ref(), &MNEM_INTERPRET_AS, -1);
    if attr_lookup(attr("format").as_ref(), &[("glyphs", 1)], -1) == 1 {
        value = sayas::GLYPHS;
    }
    let detail = attr_number(attr("detail").as_ref(), 0, false);
    if value == sayas::DIGITS {
        value = if detail <= 1 { sayas::DIGITS1 } else { sayas::DIGITS.wrapping_add(detail) };
    }
    out.push_str(&format!("\u{1}{value}Y"));
    st.sayas_start = out.len();
    st.sayas_mode = value;
}

fn sayas_close(st: &mut SsmlState, out: &mut String) {
    if st.sayas_mode == sayas::KEY {
        replace_key_name(out, st.sayas_start);
    }
    out.push_str("\u{1}Y");
    st.sayas_mode = 0;
}

fn replace_key_name(out: &mut String, start: usize) {
    let Some(tail) = out.get(start..) else { return };
    if let Some(&(_, ch)) = KEY_NAMES.iter().find(|(name, _)| *name == tail) {
        out.truncate(start);
        out.push(ch);
    }
}

fn sub_open(st: &mut SsmlState, env: &SsmlEnv, attr: &AttrFn<'_>, out: &mut String) {
    if let Some(alias) = attr("alias") {
        st.ignore_text = true;
        out.push_str(&attr_copy(Some(&alias), env.n_outbuf.saturating_sub(out.len())));
    }
}

fn mark(st: &mut SsmlState, attr: &AttrFn<'_>, out: &mut String) -> u32 {
    let Some(name_attr) = attr("name") else { return 0 };
    let name = attr_copy(Some(&name_attr), 160);
    if !name.is_empty() && st.skip_marker == name {
        st.clear_skipping_text = true;
        st.skip_marker.clear();
        return clause::NONE;
    }
    let ix = st.names.add(&name);
    out.push_str(&format!("\u{1}{ix}M"));
    0
}

fn audio_open(st: &mut SsmlState, env: &SsmlEnv, attr: &AttrFn<'_>, self_closing: bool, out: &mut String) -> u32 {
    let sp = st.push_param(SSML_AUDIO);
    if let Some(src) = attr("src") {
        audio_source(st, env, sp, &attr_copy(Some(&src), 160), out);
    }
    st.process_params(out);
    if self_closing {
        st.pop_param(SSML_AUDIO, out);
    } else {
        st.audio_text = true;
    }
    clause::NONE
}

fn audio_source(st: &mut SsmlState, env: &SsmlEnv, sp: usize, src: &str, out: &mut String) {
    match env.uri_callback {
        None => {
            let path = match &st.xmlbase {
                Some(base) if !src.starts_with('/') => format!("{base}/{src}"),
                _ => src.to_string(),
            };
            if let Some(ix) = (env.load_sound)(&path).filter(|&ix| ix >= 0) {
                out.push_str(&format!("\u{1}{ix}I"));
                st.param_stack[sp].parameter[P_SILENCE] = 1;
            }
        }
        Some(callback) => {
            let ix = st.names.add(src);
            if callback(src, st.xmlbase.as_deref()) == 0 {
                out.push_str(&format!("\u{1}{ix}U"));
                st.param_stack[sp].parameter[P_SILENCE] = 1;
            }
        }
    }
}

fn break_tag(st: &mut SsmlState, env: &SsmlEnv, attr: &AttrFn<'_>, out: &mut String) -> u32 {
    let mut value = 21;
    let mut terminator = clause::NONE;
    if let Some(strength) = attr("strength") {
        let level = attr_lookup(Some(&strength), &MNEM_BREAK, -1);
        let level = if level < 0 { 2 } else { level };
        if level < 3 {
            out.push_str(&format!("\u{1}{level}B"));
            terminator = 0;
        }
        value = BREAK_VALUE[level as usize];
    }
    if let Some(time) = attr("time") {
        value = break_time(st, env, attr_number(Some(&time), 0, true));
        if terminator == 0 {
            terminator = clause::NONE;
        }
    }
    if terminator == 0 {
        return 0;
    }
    if value > 0xfff {
        value = (value / 32).min(0xfff);
        terminator |= clause::PAUSE_LONG;
    }
    terminator + value as u32
}

/// Pause in units of 10 ms at the current speed for `ms` milliseconds.
fn break_time(st: &mut SsmlState, env: &SsmlEnv, ms: i32) -> i32 {
    let ms = ms.wrapping_mul(st.speech_parameters[P_SSML_BREAK_MUL]) / 100;
    let wpm = st.speech_parameters[P_RATE];
    st.param_stack[0].parameter[P_RATE] = wpm;
    let (pause, clause_pause) = (env.pause_factors)(wpm);
    let scaled = |factor: i32| ms.wrapping_mul(256).checked_div(factor.wrapping_mul(10)).unwrap_or(0);
    let value = scaled(clause_pause);
    if value < 200 { scaled(pause) } else { value }
}

fn voice_tags(st: &mut SsmlState, env: &SsmlEnv, buf: &[char], px: usize, tag_type: u32) -> u32 {
    let base = tag_type & !TAG_CLOSE;
    let closing = tag_type & TAG_CLOSE != 0;
    match (base, closing) {
        (SSML_SPEAK, false) => {
            if let Some(a) = get_ssml_attribute(buf, px, "xml:base") {
                st.names.add(&attr_copy(Some(&a), 160));
            }
            voice_open(st, env, buf, px, tag_type)
        }
        (SSML_VOICE, false) => voice_open(st, env, buf, px, tag_type),
        (SSML_SPEAK, true) | (SSML_VOICE, true) => voice_close(st, env, buf, px, base, tag_type),
        (HTML_BREAK, _) => clause::COLON,
        (SSML_SENTENCE, false) | (SSML_PARAGRAPH, false) => para_open(st, env, buf, px, tag_type),
        (SSML_SENTENCE, true) => {
            let mut flag = 0;
            if st.voice.top_tag() == SSML_SENTENCE {
                flag = st.voice.get_voice_attributes(env.voices, buf, px, tag_type);
            }
            clause::PERIOD + flag
        }
        (SSML_PARAGRAPH, true) => para_close(st, env, buf, px, tag_type),
        _ => 0,
    }
}

fn voice_open(st: &mut SsmlState, env: &SsmlEnv, buf: &[char], px: usize, tag_type: u32) -> u32 {
    match st.voice.get_voice_attributes(env.voices, buf, px, tag_type) {
        0 => 0,
        _ => clause::VOICE,
    }
}

fn voice_close(st: &mut SsmlState, env: &SsmlEnv, buf: &[char], px: usize, base: u32, tag_type: u32) -> u32 {
    while st.voice.depth > 1 && st.voice.top_tag() != base {
        st.voice.depth -= 1;
    }
    let change = st.voice.get_voice_attributes(env.voices, buf, px, tag_type);
    if base == SSML_SPEAK { clause::PERIOD + change } else { change }
}

fn para_open(st: &mut SsmlState, env: &SsmlEnv, buf: &[char], px: usize, tag_type: u32) -> u32 {
    let mut flag = 0;
    for outer in [SSML_SENTENCE, SSML_PARAGRAPH] {
        let applies = st.voice.top_tag() == outer && (outer == tag_type || tag_type == SSML_PARAGRAPH);
        if applies {
            flag |= st.voice.get_voice_attributes(env.voices, buf, px, outer + TAG_CLOSE);
        }
    }
    flag |= st.voice.get_voice_attributes(env.voices, buf, px, tag_type);
    clause::PARAGRAPH + flag
}

fn para_close(st: &mut SsmlState, env: &SsmlEnv, buf: &[char], px: usize, tag_type: u32) -> u32 {
    let top = st.voice.top_tag();
    if top == SSML_SENTENCE || top == SSML_PARAGRAPH {
        return st.voice.get_voice_attributes(env.voices, buf, px, tag_type) + clause::PARAGRAPH;
    }
    clause::PARAGRAPH
}

/// `ParseSsmlReference`: resolves the text after `&` up to `;`. Returns the
/// replacement code point; `c2` is set to a space for named entities when it was 0.
pub fn parse_ssml_reference(reference: &str, c2: &mut u32) -> Option<u32> {
    if let Some(num) = reference.strip_prefix('#') {
        return match num.strip_prefix('x') {
            Some(hex) => scan_number(hex, 16),
            None => scan_number(num, 10),
        };
    }
    let value = match reference {
        "gt" => '>' as u32,
        "lt" => 0xe000 + '<' as u32,
        "amp" => '&' as u32,
        "quot" => '"' as u32,
        "nbsp" => ' ' as u32,
        "apos" => '\'' as u32,
        _ => return None,
    };
    if *c2 == 0 {
        *c2 = ' ' as u32;
    }
    Some(value)
}

/// `sscanf("%x"/"%d")`: leading digits only; `None` when there are none.
fn scan_number(text: &str, radix: u32) -> Option<u32> {
    let digits: String = text.chars().take_while(|c| c.is_digit(radix)).collect();
    u32::from_str_radix(&digits, radix).ok()
}
