//! C's `TranslateLetter`: naming a single letter, with the alphabet's name in
//! front of a letter from another script and the letter itself taken from the
//! language `alphabets[]` nominates.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::{
    decompose_hangul, derived_letter, letter_name, lowercase_for_lang, parse_inline_phonemes,
    primary_bcp47_subtag, raw_list_entry, resolve_dict_stem, select_phoneme_table,
    word_to_phonemes_inner, CodeMarker, LangOptions, PhonemeCode, PosExpect, Translator,
};
use super::{
    apply_list_editing_instructions, double_lengthened_consonants,
    resolve_virtual_phonemes_in_codes,
};
use crate::dictionary::stress::{set_word_stress, StressOpts};
use crate::dictionary::Dictionary;
use crate::phoneme::{PhonemeData, PHON_PAUSE, PHON_PAUSE_VSHORT, PHON_SWITCH};

#[derive(Clone, Copy)]
pub(crate) struct Alphabet {
    pub name: &'static str,
    pub offset: u32,
    min: u32,
    max: u32,
    pub language: &'static str,
    pub dont_name: bool,
    pub not_letters: bool,
    pub words: bool,
    no_symbol: bool,
    not_code: bool,
}

const fn alphabet(
    name: &'static str,
    offset: u32,
    min: u32,
    max: u32,
    language: &'static str,
    flags: u8,
) -> Alphabet {
    Alphabet {
        name,
        offset,
        min,
        max,
        language,
        dont_name: flags & 0x01 != 0,
        not_letters: flags & 0x02 != 0,
        words: flags & 0x04 != 0,
        not_code: flags & 0x08 != 0,
        no_symbol: flags & 0x10 != 0,
    }
}

/// The `alphabets[]` table of `tr_languages.c`, in the same order.
const ALPHABETS: [Alphabet; 28] = [
    alphabet("_el", 0x380, 0x380, 0x3ff, "el", 0x01 | 0x02 | 0x04),
    alphabet("_cyr", 0x420, 0x400, 0x52f, "", 0),
    alphabet("_hy", 0x530, 0x530, 0x58f, "hy", 0x04),
    alphabet("_he", 0x590, 0x590, 0x5ff, "he", 0),
    alphabet("_ar", 0x600, 0x600, 0x6ff, "ar", 0x04),
    alphabet("_syc", 0x700, 0x700, 0x74f, "", 0),
    alphabet("_hi", 0x900, 0x900, 0x97f, "hi", 0x04),
    alphabet("_bn", 0x980, 0x980, 0x9ff, "bn", 0x04),
    alphabet("_gur", 0xa00, 0xa00, 0xa7f, "pa", 0x04),
    alphabet("_gu", 0xa80, 0xa80, 0xaff, "gu", 0x04),
    alphabet("_or", 0xb00, 0xb00, 0xb7f, "", 0),
    alphabet("_ta", 0xb80, 0xb80, 0xbff, "ta", 0x04),
    alphabet("_te", 0xc00, 0xc00, 0xc7f, "te", 0),
    alphabet("_kn", 0xc80, 0xc80, 0xcff, "kn", 0x04),
    alphabet("_ml", 0xd00, 0xd00, 0xd7f, "ml", 0x04),
    alphabet("_si", 0xd80, 0xd80, 0xdff, "si", 0x04),
    alphabet("_th", 0xe00, 0xe00, 0xe7f, "", 0),
    alphabet("_lo", 0xe80, 0xe80, 0xeff, "", 0),
    alphabet("_ti", 0xf00, 0xf00, 0xfff, "", 0),
    alphabet("_my", 0x1000, 0x1000, 0x109f, "", 0),
    alphabet("_ka", 0x10a0, 0x10a0, 0x10ff, "ka", 0x04),
    alphabet("_ko", 0x1100, 0x1100, 0x11ff, "ko", 0x04),
    alphabet("_eth", 0x1200, 0x1200, 0x139f, "", 0),
    alphabet("_braille", 0x2800, 0x2800, 0x28ff, "", 0x10),
    alphabet("_ja", 0x3040, 0x3040, 0x30ff, "", 0x08),
    alphabet("_zh", 0x3100, 0x3100, 0x9fff, "", 0x08 | 0x20),
    alphabet("_ko", 0xa700, 0xa700, 0xd7ff, "ko", 0x08 | 0x04),
    alphabet("_shaw", 0x10450, 0x10450, 0x1047f, "en", 0),
];

/// C's `AlphabetFromChar`.
pub(crate) fn alphabet_from_char(c: u32) -> Option<&'static Alphabet> {
    ALPHABETS.iter().find(|a| c <= a.max).filter(|a| c >= a.min)
}

/// `langopts.alt_alphabet`, its language, and `langopts.our_alphabet`.
pub(crate) fn voice_alphabets(lang: &str) -> (u32, &'static str, u32) {
    match primary_bcp47_subtag(lang) {
        "ar" => (0x3100, "cmn", 0),
        "ka" => (0x420, "ru", 0),
        "hr" | "bs" | "sr" => (0, "en", 0x420),
        "ko" => (0, "en", 0xa700),
        "cmn" | "yue" | "zh" => (0, "en", 0x3100),
        _ => (0, "en", 0),
    }
}

/// One piece of a phoneme string whose table can change part-way through, as
/// C's `phonSWITCH <table>` pairs do.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Item {
    Ph(u8),
    Switch(String),
}

pub(super) fn encode(items: &[Item]) -> Vec<u8> {
    let mut out = Vec::with_capacity(items.len() + 4);
    for item in items {
        match item {
            Item::Ph(b) => out.push(*b),
            Item::Switch(name) => {
                out.push(PHON_SWITCH);
                out.push(name.len() as u8);
                out.extend_from_slice(name.as_bytes());
            }
        }
    }
    out
}

pub(super) fn decode(bytes: &[u8]) -> Vec<Item> {
    let mut items = Vec::with_capacity(bytes.len());
    let mut ix = 0;
    while ix < bytes.len() {
        let b = bytes[ix];
        ix += 1;
        if b == PHON_SWITCH {
            let n = bytes.get(ix).copied().map_or(0, usize::from);
            let name = bytes.get(ix + 1..ix + 1 + n).unwrap_or(&[]);
            items.push(Item::Switch(String::from_utf8_lossy(name).into_owned()));
            ix += 1 + n;
        } else if b != 0 {
            items.push(Item::Ph(b));
        }
    }
    items
}

pub(super) fn has_switch(items: &[Item]) -> bool {
    items.iter().any(|i| matches!(i, Item::Switch(_)))
}

pub(super) fn plain_bytes(items: &[Item]) -> Vec<u8> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Ph(b) => Some(*b),
            Item::Switch(_) => None,
        })
        .collect()
}

/// Everything one language needs to name letters.
pub(super) struct LangCtx {
    pub dict: Dictionary,
    pub phdata: PhonemeData,
    pub stress_opts: StressOpts,
    pub options: LangOptions,
}

impl LangCtx {
    fn load(lang: &str, data_dir: &Path) -> Option<Self> {
        let stem = resolve_dict_stem(data_dir, lang)?;
        let path = super::dict_path(data_dir, &stem)?;
        let mut dict = Dictionary::from_bytes(&stem, std::fs::read(path).ok()?).ok()?;
        let mut options = Translator::new(lang, Some(data_dir)).ok()?.options;
        options.switched = true;
        dict.dict_condition = options.dict_condition;
        let mut phdata = PhonemeData::load(data_dir).ok()?;
        select_phoneme_table(&mut phdata, data_dir, lang).ok()?;
        let stress_opts = StressOpts::for_lang(lang);
        Some(Self { dict, phdata, stress_opts, options })
    }

    pub(super) fn ctx(&self) -> Ctx<'_> {
        Ctx {
            dict: &self.dict,
            phdata: &self.phdata,
            stress: &self.stress_opts,
            options: &self.options,
        }
    }
}

/// The other languages C reaches through `translator3`, loaded on first use.
pub(crate) struct AltLangs {
    data_dir: PathBuf,
    cache: RefCell<HashMap<String, Option<Rc<LangCtx>>>>,
}

impl AltLangs {
    pub(crate) fn new(data_dir: &Path) -> Self {
        Self { data_dir: data_dir.to_path_buf(), cache: RefCell::new(HashMap::new()) }
    }

    pub(super) fn get(&self, lang: &str) -> Option<Rc<LangCtx>> {
        self.cache
            .borrow_mut()
            .entry(lang.to_string())
            .or_insert_with(|| LangCtx::load(lang, &self.data_dir).map(Rc::new))
            .clone()
    }
}

#[derive(Clone, Copy)]
pub(super) struct Ctx<'a> {
    pub dict: &'a Dictionary,
    pub phdata: &'a PhonemeData,
    pub stress: &'a StressOpts,
    pub options: &'a LangOptions,
}

impl Ctx<'_> {
    fn lang(&self) -> &str {
        &self.options.lang
    }

    fn name_of(&self, letter: char) -> Vec<u8> {
        letter_name(letter, self.dict, self.phdata, self.stress, self.options)
    }

    fn entry(&self, key: &str) -> Option<Vec<u8>> {
        raw_list_entry(key, self.dict, self.phdata, self.stress, self.options).map(|(ph, _)| ph)
    }
}

/// The closing switch a stream left in the code list, which a stream that
/// follows at once replaces.
pub(crate) struct StreamTail {
    at: usize,
    before: String,
}

fn code(b: u8) -> PhonemeCode {
    PhonemeCode { code: b, is_boundary: false, clause_char: None, marker: None }
}

fn marker(table: &str) -> PhonemeCode {
    PhonemeCode { marker: Some(CodeMarker::LangSwitch(table.to_string())), ..Default::default() }
}

fn opens_with_switch(items: &[Item]) -> bool {
    matches!(items, [Item::Switch(_), ..] | [Item::Ph(PHON_PAUSE), Item::Switch(_), ..])
}

/// A switch back to the voice that closed another word's region just before.
fn closing_marker(codes: &[PhonemeCode], voice: &str) -> Option<StreamTail> {
    let at = codes.iter().rposition(|c| !(c.is_boundary && c.marker.is_none()))?;
    let is_voice = |c: &PhonemeCode| matches!(&c.marker, Some(CodeMarker::LangSwitch(t)) if t == voice);
    if !is_voice(&codes[at]) {
        return None;
    }
    let before = codes[..at]
        .iter()
        .rev()
        .find_map(|c| match &c.marker {
            Some(CodeMarker::LangSwitch(t)) => Some(t.clone()),
            _ => None,
        })?;
    Some(StreamTail { at, before })
}

/// C's `phonSWITCH` clean-up in `MakePhonemeList`: a switch to the table already
/// in force, or one followed by another switch (a pause may sit between), is dropped.
pub(super) fn push_stream(
    codes: &mut Vec<PhonemeCode>,
    bytes: &[u8],
    voice: &str,
    tail: &mut Option<StreamTail>,
) {
    let items = decode(bytes);
    let mut current = voice.to_string();
    if tail.is_none() {
        *tail = closing_marker(codes, voice);
    }
    if let Some(t) = tail.take() {
        let only_gaps = codes[t.at + 1..].iter().all(|c| c.is_boundary && c.marker.is_none());
        if only_gaps && opens_with_switch(&items) {
            codes.remove(t.at);
            current = t.before;
        }
    }
    for (ix, item) in items.iter().enumerate() {
        match item {
            Item::Ph(b) => codes.push(code(*b)),
            Item::Switch(table) => {
                let rest = &items[ix + 1..];
                if *table == current || opens_with_switch(rest) {
                    continue;
                }
                if ix + 1 == items.len() {
                    *tail = Some(StreamTail { at: codes.len(), before: current.clone() });
                }
                codes.push(marker(table));
                current = table.clone();
            }
        }
    }
}

/// What `TranslateLetter` makes of a letter.
pub(super) enum LetterOut {
    /// The phonemes, with a table switch wherever a name comes from another language.
    Items(Vec<Item>),
    /// The language's list hands the letter to another language (`phonSWITCH <lang>`).
    HandOver(Vec<u8>),
}

/// The speaker of a letter (`tr`) and the voice it is spoken for (`translator`).
pub(super) struct LetterEnv<'a> {
    pub voice: Ctx<'a>,
    pub tr: Ctx<'a>,
    pub alts: &'a AltLangs,
}

fn non_ascii_digit(letter: char) -> Option<char> {
    const STARTS: [u32; 16] = [
        0x660, 0x6f0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66, 0xe50,
        0xed0, 0xf20, 0x1040, 0x1090,
    ];
    let c = letter as u32;
    STARTS
        .iter()
        .find(|&&s| c >= s && c <= s + 9)
        .and_then(|&s| char::from_digit(c - s, 10))
}

fn switched(lang: &str, mut inner: Vec<Item>, back: &str) -> Vec<Item> {
    let mut out = vec![Item::Ph(PHON_PAUSE), Item::Switch(lang.to_string())];
    out.append(&mut inner);
    out.push(Item::Switch(back.to_string()));
    out
}

fn bytes_as_items(bytes: &[u8]) -> Vec<Item> {
    bytes.iter().filter(|&&b| b != 0).map(|&b| Item::Ph(b)).collect()
}

/// A name from a table other than the voice's, run through that table's
/// phoneme programs the way `MakePhonemeList` runs them over the whole list.
fn in_own_table(ctx: Ctx, bytes: &[u8]) -> Vec<Item> {
    let mut codes: Vec<PhonemeCode> = bytes.iter().filter(|&&b| b != 0).map(|&b| code(b)).collect();
    let mut from_dict = vec![true; codes.len()];
    double_lengthened_consonants(&mut codes, &mut from_dict, ctx.phdata);
    resolve_virtual_phonemes_in_codes(&mut codes, ctx.phdata, &from_dict, ctx.stress.reduce);
    apply_list_editing_instructions(
        &mut codes,
        ctx.phdata,
        ctx.stress.vowel_pause,
        ctx.stress.word_gap,
    );
    codes.iter().map(|c| Item::Ph(c.code)).collect()
}

/// The name of the letter's alphabet, spoken ahead of the letter (C: `ph_alphabet`).
fn alphabet_name(env: &LetterEnv, alphabet: Option<&Alphabet>) -> Vec<Item> {
    let Some(a) = alphabet else { return Vec::new() };
    let (alt, _, our) = voice_alphabets(env.voice.lang());
    let skipped = a.dont_name
        || a.offset == env.voice.dict.letter_bits_offset
        || (a.offset != 0 && (a.offset == alt || a.offset == our));
    if skipped {
        return Vec::new();
    }
    let (voice_lang, tr_lang) = (env.voice.lang(), env.tr.lang());
    match env.voice.entry(a.name) {
        Some(name) if voice_lang == tr_lang => bytes_as_items(&name),
        Some(name) => switched(voice_lang, bytes_as_items(&name), tr_lang),
        None => env
            .alts
            .get("en")
            .and_then(|en| en.ctx().entry(a.name).map(|name| in_own_table(en.ctx(), &name)))
            .map(|name| switched("en", name, voice_lang))
            .unwrap_or_default(),
    }
}

fn hangul_name(alt: &LangCtx, letter: char) -> Vec<u8> {
    let Some(jamo) = decompose_hangul(&letter.to_string()) else { return Vec::new() };
    let c = alt.ctx();
    let expect = PosExpect { defer_stress: true, ..PosExpect::default() };
    let mut ph = word_to_phonemes_inner(&jamo, c.dict, c.phdata, c.stress, c.options, 0, expect)
        .phonemes;
    set_word_stress(&mut ph, c.phdata, c.stress, Some(0), -1, 0);
    ph.retain(|&b| b != 0);
    ph
}

/// The letter's name in the language `alphabets[]` assigns its script.
fn foreign_name(env: &LetterEnv, alphabet: Option<&Alphabet>, letter: char) -> Vec<Item> {
    let (alt, alt_lang, _) = voice_alphabets(env.voice.lang());
    let language = match alphabet {
        Some(a) if a.offset != 0 && a.offset == alt => alt_lang,
        Some(a) if !a.language.is_empty() && !a.not_letters => a.language,
        _ => "en",
    };
    let own = primary_bcp47_subtag(env.tr.lang()) == language;
    if own && language != "ko" {
        return Vec::new();
    }
    let Some(alt_ctx) = env.alts.get(language) else { return Vec::new() };
    let is_syllable = (0xac00..=0xd7af).contains(&(letter as u32));
    let mut name = if is_syllable {
        hangul_name(&alt_ctx, letter)
    } else {
        alt_ctx.ctx().name_of(letter)
    };
    let mut lang = language.to_string();
    if name.first() == Some(&PHON_SWITCH) {
        let next = String::from_utf8_lossy(&name[1..]).trim_end_matches('\0').to_string();
        let Some(again) = env.alts.get(&next) else { return Vec::new() };
        name = again.ctx().name_of(letter);
        lang = next;
    }
    if name.is_empty() {
        return Vec::new();
    }
    let Some(owner) = env.alts.get(&lang) else { return Vec::new() };
    switched(&lang, in_own_table(owner.ctx(), &name), env.tr.lang())
}

fn braille_dots(letter: char) -> String {
    (0..8).filter(|ix| letter as u32 & (1 << ix) != 0).map(|ix| char::from(b'1' + ix as u8)).collect()
}

/// "letter", then the digits of the code point: what C says for a character it cannot name.
fn unnamed(env: &LetterEnv, alphabet: Option<&Alphabet>, letter: char) -> Vec<Item> {
    const HEX_NAMES: [&str; 6] = ["'e:j", "b'i:", "s'i:", "d'i:", "'i:", "'ef"];
    let voice = env.voice;
    let mut out: Vec<u8> = Vec::new();
    if !alphabet.is_some_and(|a| a.no_symbol) {
        out = Some(letter)
            .filter(|c| c.is_alphabetic())
            .and_then(|_| voice.entry("_?A"))
            .or_else(|| (!letter.is_whitespace()).then(|| voice.entry("_??")).flatten())
            .unwrap_or_else(|| parse_inline_phonemes("l'et@", voice.phdata));
    }
    if alphabet.is_some_and(|a| a.not_code) {
        return bytes_as_items(&out);
    }
    let digits = match alphabet {
        Some(a) if a.offset == 0x2800 => braille_dots(letter),
        _ => format!("{:x}", letter as u32),
    };
    for digit in digits.chars() {
        out.push(PHON_PAUSE_VSHORT);
        let mut spelled = voice.name_of(digit);
        if spelled.first().is_none_or(|&b| b == PHON_SWITCH) && digit >= 'a' {
            spelled = parse_inline_phonemes(HEX_NAMES[(digit as u8 - b'a') as usize], voice.phdata);
        }
        out.extend(spelled);
    }
    out.push(PHON_PAUSE);
    if env.tr.lang() != voice.lang() {
        out = restress_under(env.tr.phdata, &out);
    }
    bytes_as_items(&out)
}

/// The name is read in the voice but staged under the other translator's
/// phoneme table, so a stress mark survives only before a vowel of that table.
fn restress_under(phdata: &PhonemeData, bytes: &[u8]) -> Vec<u8> {
    let mut pending: Option<u8> = None;
    let mut out = Vec::with_capacity(bytes.len());
    for &b in bytes {
        match phdata.get(b).map(|p| p.typ) {
            Some(crate::phoneme::PH_STRESS) => pending = Some(b),
            Some(crate::phoneme::PH_VOWEL) => {
                out.extend(pending.take());
                out.push(b);
            }
            _ => out.push(b),
        }
    }
    out
}

fn named_by_tr(env: &LetterEnv, letter: char) -> (char, Vec<u8>) {
    for candidate in std::iter::once(letter).chain(derived_letter(letter)) {
        let name = env.tr.name_of(candidate);
        if !name.is_empty() {
            return (candidate, name);
        }
    }
    (letter, Vec::new())
}

/// A lone letter of a script the voice hands to `target`: `TranslateWord2` wraps
/// what `TranslateLetter` makes of it in a switch to `target` and one back.
pub(super) fn spell_in_switched(
    voice: Ctx,
    alts: &AltLangs,
    target: &str,
    letter: char,
) -> Option<Vec<Item>> {
    let mut target = target.to_string();
    for attempt in 0..2 {
        let tr = alts.get(&target)?;
        let env = LetterEnv { voice, tr: tr.ctx(), alts };
        match translate_letter(&env, letter) {
            LetterOut::Items(items) => return Some(wrap_switched(items, &tr.options.lang, voice)),
            LetterOut::HandOver(name) if attempt == 0 => target = switch_language(&name),
            LetterOut::HandOver(_) => return Some(Vec::new()),
        }
    }
    None
}

pub(super) fn switch_language(phonemes: &[u8]) -> String {
    let name: Vec<u8> = phonemes[1..].iter().copied().take_while(|b| b.is_ascii_alphanumeric() || *b == b'-').collect();
    if name.is_empty() { "en".to_string() } else { String::from_utf8_lossy(&name).into_owned() }
}

fn wrap_switched(mut items: Vec<Item>, target: &str, voice: Ctx) -> Vec<Item> {
    let opens = matches!(items.as_slice(), [Item::Ph(PHON_PAUSE), Item::Switch(_), ..]);
    if !opens {
        items.insert(0, Item::Switch(target.to_string()));
    }
    items.push(Item::Switch(voice.lang().to_string()));
    items
}

/// C's `TranslateLetter` for one letter, spoken by `env.tr` for `env.voice`.
pub(super) fn translate_letter(env: &LetterEnv, letter: char) -> LetterOut {
    let lower = lowercase_for_lang(&letter.to_string(), env.tr.lang()).chars().next();
    let (letter, mut name) = named_by_tr(env, lower.unwrap_or(letter));
    if name.first() == Some(&PHON_SWITCH) {
        return LetterOut::HandOver(name);
    }
    if name.is_empty() {
        if let Some(digit) = non_ascii_digit(letter) {
            name = env.tr.name_of(digit);
        }
    }
    let alphabet = alphabet_from_char(letter as u32);
    let mut items = alphabet_name(env, alphabet);
    let mut spoken = bytes_as_items(&name);
    if spoken.is_empty() {
        spoken = foreign_name(env, alphabet, letter);
    }
    if spoken.is_empty() {
        spoken = unnamed(env, alphabet, letter);
    }
    items.append(&mut spoken);
    LetterOut::Items(items)
}
