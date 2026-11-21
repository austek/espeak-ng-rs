//! Markup mode as C `SpeakNextClause` runs it: the clause reader ends each clause, the
//! translator of the current voice translates it, and a voice change takes effect after the
//! clause that carried it.

use super::readclause::ssml_tag::{NameData, SsmlEnv, SsmlState, N_SPEECH_PARAM, P_CAPITALS, P_PUNCTUATION, P_RATE, P_SSML_BREAK_MUL};
use super::readclause::voice_stack::{BaseVoice, EntryCatalog};
use super::readclause::{CharMap, ClauseReader, ReaderLang, ReaderOptions, SourceClause, SsmlTags, Terminator};
use super::{CodeMarker, PhonemeCode, Result, Token, Translator};
use crate::synthesize::cgen::phlist::{ClauseCodes, ClauseCounters};
use crate::voices::{list_catalog, list_voices, select_voice_by_name, VoiceEntry};

/// How the token stream of one translation call is cut into clauses.
#[derive(Clone, Copy)]
pub(crate) enum TokenizeMode {
    Plain,
    /// The text is one clause already: only a closing boundary ends it.  `no_tonic` is C
    /// `CalcPitches` `clause_type == 4`: the clause gets no tonic syllable.
    Clause { no_tonic: bool },
}

impl TokenizeMode {
    pub(crate) fn apply(self, tokens: Vec<Token>) -> Vec<Token> {
        match self {
            TokenizeMode::Plain => tokens,
            TokenizeMode::Clause { .. } => single_clause(tokens),
        }
    }

    pub(crate) fn no_tonic(self) -> bool {
        matches!(self, TokenizeMode::Clause { no_tonic: true })
    }
}

const INTONATION_NONE: u32 = 4;

/// The translator statics C keeps across the clauses of one text (`option_emphasis` and
/// `word_emphasis`, reset only by `InitText`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ClauseCarry {
    pub emphasis: i32,
    pub say_as: super::say_as::SayAs,
    pub counts: MessageCounts,
}

/// `count_words`, `count_sentences` and `new_sentence`: kept for the whole message, across
/// voice changes, and reset only by `InitText`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MessageCounts {
    words: u32,
    sentences: u32,
    new_sentence: bool,
}

impl Default for MessageCounts {
    fn default() -> Self {
        MessageCounts { words: 0, sentences: 0, new_sentence: true }
    }
}

impl MessageCounts {
    /// `TranslateClause`'s bookkeeping around one clause.  A clause of blanks is no sentence
    /// and hands its sentence start on to the next clause.
    fn clause(&mut self, source: &SourceClause, terminator: Terminator, codes: &[PhonemeCode], words: u32) -> ClauseCounters {
        let blank = source.text.chars().all(char::is_whitespace);
        let opens = self.new_sentence && !blank;
        self.sentences += u32::from(opens);
        let counters = ClauseCounters {
            start_char: source.start_char.max(0) as u32,
            start_word: self.words + 1,
            sentences: self.sentences,
            new_sentence: opens,
            end_char: source.end_char.max(0) as u32,
            end_ix: closing_pause_ix(source, codes),
        };
        self.words += words;
        self.new_sentence = terminator.is_sentence() || (blank && self.new_sentence);
        counters
    }
}

/// `sourceix` of the clause's closing pauses, which carry a mark after the last word.
/// `TranslateClause` leaves `source_index` one past the clause text's NUL.  A clause with no
/// word but an embedded command adds the empty `words[0]`, whose dummy phoneme (`sourceix` 0)
/// carries the command instead, so the closing pause is not asked.
fn closing_pause_ix(source: &SourceClause, codes: &[PhonemeCode]) -> u16 {
    let has_word = codes.iter().any(|c| !c.is_boundary && c.marker.is_none());
    if has_word || !source.text.contains('\u{1}') {
        ((source.text.len() + 1) & 0x7ff) as u16
    } else {
        0
    }
}

/// Character of the [`Token::ClauseBoundary`] that ends a clause the reader or the length
/// limit cut, and that the driver adds when a clause text has none of its own.  The tokenizer
/// never maps it from text, where it is an ordinary private-use character as in C.
pub(crate) const TAG_CLAUSE_END: char = '\u{E002}';

/// Character of the [`Token::ClauseBoundary`] that ends the clause at an embedded voice change,
/// which C ends with `CLAUSE_VOICE`: no pause, no intonation, no sentence.
pub(crate) const VOICE_CLAUSE_END: char = '\u{E003}';

fn single_clause(mut tokens: Vec<Token>) -> Vec<Token> {
    let last = tokens.iter().rposition(|t| !matches!(t, Token::Space));
    let closing = last.filter(|&i| matches!(tokens[i], Token::ClauseBoundary(_)));
    for (i, t) in tokens.iter_mut().enumerate() {
        match *t {
            Token::ClauseBoundary(_) if Some(i) == closing => {}
            Token::ClauseBoundary('.') => *t = Token::Space,
            Token::ClauseBoundary(c) => *t = Token::Punctuation(c),
            _ => {}
        }
    }
    if closing.is_none() {
        tokens.push(Token::ClauseBoundary(TAG_CLAUSE_END));
    }
    tokens
}

/// Index of the first sounding code of each word group, words being what the boundary codes
/// delimit.
fn word_group_starts(codes: &[PhonemeCode]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut open = true;
    for (i, c) in codes.iter().enumerate() {
        if c.is_boundary {
            open = true;
        } else if open && c.marker.is_none() {
            starts.push(i);
            open = false;
        }
    }
    starts
}

/// C gives a word whose `CalcWordLength` read no `charix` (the alias text of `<sub>` past the
/// reader's `charix_top`) a `sourceix` of 0: it starts no word in the phoneme list.
fn mark_sourceless_words(codes: &mut [PhonemeCode], cut: &[super::c_words::CutWord]) {
    let starts = word_group_starts(codes);
    if starts.len() != cut.len() {
        return;
    }
    for (&at, word) in starts.iter().zip(cut) {
        if word.sourceix == 0 && word.length == 0 && codes[at].source.is_none() {
            codes[at].source = Some(super::SourceIx::default());
        }
    }
}

fn variant_gender(gender: char) -> i32 {
    match gender {
        'M' => 1,
        'F' => 2,
        _ => 0,
    }
}

/// The codes with the word boundary before each sourceless word removed: C writes no space
/// where `sourceix` is 0, because that phoneme starts no word.
fn without_sourceless_gaps(codes: &[PhonemeCode]) -> Vec<PhonemeCode> {
    let sourceless = Some(super::SourceIx::default());
    let mut gap = vec![false; codes.len()];
    let is_gap = |k: &PhonemeCode| k.is_boundary && k.code == super::PHON_END_WORD;
    for at in (0..codes.len()).filter(|&i| codes[i].source == sourceless) {
        let run = codes[..at].iter().rev().take_while(|k| k.marker.is_some() || is_gap(k)).count();
        (at - run..at).filter(|&i| is_gap(&codes[i])).for_each(|i| gap[i] = true);
    }
    codes.iter().zip(gap).filter(|(_, g)| !g).map(|(c, _)| c.clone()).collect()
}

/// Speech settings the SSML parameter stack starts from (C `param_stack[0]`).
#[derive(Clone, Debug)]
pub struct SsmlSettings {
    pub rate_wpm: i32,
    /// `espeakSSML_BREAK_MUL` as a percentage.
    pub break_mul: i32,
    /// The `+variant` of the base voice as it was named (`m3`, `whisper`, `3`); empty for none.
    pub base_variant: String,
}

impl Default for SsmlSettings {
    fn default() -> Self {
        SsmlSettings { rate_wpm: 175, break_mul: 100, base_variant: String::new() }
    }
}

/// One clause after `TranslateClause`.
#[derive(Clone, Debug)]
pub struct TranslatedClause {
    pub codes: Vec<PhonemeCode>,
    pub terminator: Terminator,
    pub eof: bool,
    /// The voice id to switch to once this clause is spoken.
    pub voice_after: Option<String>,
    /// Index into [`SsmlDocument::voices`] of the voice that translated the clause.
    pub voice: usize,
    pub counters: ClauseCounters,
}

/// The clauses of a markup text and the voices that translated them; voice 0 is the base.
pub struct SsmlDocument<'a> {
    base: &'a Translator,
    others: Vec<(String, Translator)>,
    pub clauses: Vec<TranslatedClause>,
    names: NameData,
    /// The clause the reader was in at each URI callback, in call order.
    pub uri_clauses: Vec<usize>,
}

impl SsmlDocument<'_> {
    /// The `<mark>` name a marker's value addresses (`namedata + value`).
    pub fn mark_name(&self, value: i32) -> Option<&str> {
        self.names.get(usize::try_from(value).ok()?)
    }

    pub fn translator(&self, voice: usize) -> &Translator {
        match voice.checked_sub(1) {
            Some(ix) => &self.others[ix].1,
            None => self.base,
        }
    }

    /// The voice ids in the order of [`TranslatedClause::voice`]; the base voice is `None`.
    pub fn voice_id(&self, voice: usize) -> Option<&str> {
        voice.checked_sub(1).map(|ix| self.others[ix].0.as_str())
    }

    /// One `--ipa` line per clause, empty clauses included (C prints `"%s\n"` per clause).
    pub fn ipa_lines(&self, preserve_punctuation: bool) -> Result<Vec<String>> {
        self.clauses.iter().map(|c| self.translator(c.voice).clause_ipa(&c.codes, preserve_punctuation)).collect()
    }

    /// The clauses for synthesis.  A clause of another voice reads its phonemes from that
    /// voice's table, the way a `phonSWITCH` word does.
    pub fn clause_codes(&self) -> Vec<ClauseCodes> {
        self.clauses.iter().map(|c| self.synth_clause(c)).collect()
    }

    fn synth_clause(&self, clause: &TranslatedClause) -> ClauseCodes {
        let spoken: Vec<PhonemeCode> = clause
            .codes
            .iter()
            .filter(|c| !(c.code == 0 && c.is_boundary && c.marker.is_none()))
            .cloned()
            .collect();
        ClauseCodes {
            codes: spoken,
            terminator: clause.terminator,
            eof: clause.eof,
            voice_after: clause.voice_after.clone(),
            counters: Some(clause.counters),
        }
    }

    /// One `-x` line per clause.
    pub fn mnemonic_lines(&self, sep: Option<char>, tie: Option<char>) -> Result<Vec<String>> {
        self.clauses.iter().map(|c| self.translator(c.voice).clause_mnemonics(&c.codes, sep, tie)).collect()
    }
}

impl Translator {
    /// Read and translate a markup text clause by clause.
    pub fn translate_ssml(&self, text: &str, settings: SsmlSettings) -> Result<SsmlDocument<'_>> {
        self.translate_ssml_with_sounds(text, settings, &|_| None)
    }

    /// As [`translate_ssml`](Self::translate_ssml); `load_sound` is `LoadSoundFile2`: the
    /// sound-icon index of an `<audio src>` file, or `None` when it cannot be loaded.
    pub fn translate_ssml_with_sounds(
        &self,
        text: &str,
        settings: SsmlSettings,
        load_sound: &dyn Fn(&str) -> Option<i32>,
    ) -> Result<SsmlDocument<'_>> {
        self.translate_ssml_with_hooks(text, settings, load_sound, None)
    }

    /// As [`translate_ssml_with_sounds`](Self::translate_ssml_with_sounds); `uri_callback` is the
    /// `espeak_SetUriCallback` hook: `<audio src>` goes to it as `(uri, xmlbase)` and a result of 0
    /// accepts the audio as a PLAY event.
    pub fn translate_ssml_with_hooks(
        &self,
        text: &str,
        settings: SsmlSettings,
        load_sound: &dyn Fn(&str) -> Option<i32>,
        uri_callback: Option<&super::readclause::ssml_tag::UriCallback>,
    ) -> Result<SsmlDocument<'_>> {
        let voices = list_voices(&self.data_dir);
        let all_voices = list_catalog(&self.data_dir);
        let catalog = EntryCatalog(&all_voices);
        let pause_factors = |wpm: i32| pause_factors(wpm);
        let env = SsmlEnv {
            voices: &catalog,
            tone_language: crate::dictionary::rules::tone_language(&self.options.lang),
            n_outbuf: ReaderOptions::default().n_buf,
            pause_factors: &pause_factors,
            load_sound,
            uri_callback,
        };
        let variant = crate::voices::variant_file_name(&settings.base_variant);
        let mut state = SsmlState::new(self.speech_defaults(&settings), self.base_voice(&voices, &settings.base_variant), &variant);
        state.voice.current_voice_id = state.voice.voice_from_stack(&catalog, 1);
        let mut reader = ClauseReader::new(text, self.reader_options(), SsmlTags::new(state, env));
        let mut doc = SsmlDocument {
            base: self,
            others: Vec::new(),
            clauses: Vec::new(),
            names: NameData::default(),
            uri_clauses: Vec::new(),
        };
        let mut voice = 0;
        let mut carry = ClauseCarry::default();
        while !reader.at_end() {
            reader.tags_mut().state.reading_clause = doc.clauses.len();
            let clause = reader.next_clause(&ClauseLang(doc.translator(voice)));
            let translated = doc.translator(voice).translate_source_clause(clause, voice, &mut carry)?;
            if let Some(id) = &translated.voice_after {
                voice = doc.voice_for(id, &voices);
                if let Some(entry) = voice_file_entry(&all_voices, id) {
                    reader.tags_mut().state.voice.voice_loaded(language_names(entry), id);
                }
            }
            doc.clauses.push(translated);
        }
        doc.names = reader.tags_mut().state.names.clone();
        doc.uri_clauses = std::mem::take(&mut reader.tags_mut().state.uri_clauses);
        Ok(doc)
    }

    fn translate_source_clause(&self, clause: SourceClause, voice: usize, carry: &mut ClauseCarry) -> Result<TranslatedClause> {
        let terminator = match clause.tone_override {
            Some(tone) => Terminator::from_bits((clause.terminator.bits() & !0x7000) | (u32::from(tone) << 12)),
            None => clause.terminator,
        };
        let no_tonic = u32::from(terminator.intonation()) == INTONATION_NONE;
        let say_as = carry.say_as;
        let mut codes = self.translate_clause(&clause.text, no_tonic, carry, &clause.charix)?;
        let cut = self.cut_clause_words(&clause.text, &clause.charix, say_as.mode, say_as.digits());
        super::c_words::apply_cut_words(&mut codes, &cut);
        mark_sourceless_words(&mut codes, &cut);
        let abbreviated = clause.terminator.dot_after_last_word()
            && codes.iter().any(|c| matches!(c.marker, Some(CodeMarker::AbbreviationDot)));
        let terminator = if abbreviated { terminator.after_abbreviation() } else { terminator };
        let counters = carry.counts.clause(&clause, terminator, &codes, cut.len() as u32);
        Ok(TranslatedClause { codes, terminator, eof: clause.eof, voice_after: clause.voice_change, voice, counters })
    }

    fn speech_defaults(&self, settings: &SsmlSettings) -> [i32; N_SPEECH_PARAM] {
        let mut d = [0; N_SPEECH_PARAM];
        d[P_RATE] = settings.rate_wpm;
        d[2] = 100;
        d[3] = 50;
        d[4] = 50;
        d[P_PUNCTUATION] = i32::from(self.options.punct.is_some());
        d[P_CAPITALS] = i32::from(self.options.capitals);
        d[P_SSML_BREAK_MUL] = settings.break_mul;
        d
    }

    fn own_voice_entry<'v>(&self, voices: &'v [VoiceEntry]) -> Option<&'v VoiceEntry> {
        select_voice_by_name(voices, &self.options.lang)
            .or_else(|| voices.iter().find(|v| v.language.eq_ignore_ascii_case(&self.options.lang)))
    }

    /// `SetVoiceStack` after `espeak_ng_SetVoiceByName`: the name as given, the identifier of the
    /// voice file that `LoadVoice` loaded and its language. The variant file's `gender` line sets
    /// the gender `espeak_GetCurrentVoice` reports.
    fn base_voice(&self, voices: &[VoiceEntry], variant: &str) -> BaseVoice {
        let entry = self.own_voice_entry(voices);
        let languages = entry
            .map(|v| v.languages.iter().map(|(l, _)| l.clone()).collect())
            .unwrap_or_else(|| vec![self.options.lang.clone()]);
        let with_variant = |stem: &str, v: &str| if variant.is_empty() { stem.to_string() } else { format!("{stem}+{v}") };
        let file = crate::voices::load_variant(&self.data_dir, variant).filter(|_| !variant.is_empty());
        BaseVoice {
            identifier: entry.map(|v| with_variant(&v.file, &crate::voices::variant_file_name(variant))),
            name: with_variant(&self.options.lang, variant),
            languages,
            current_gender: file.map(|v| variant_gender(v.gender)),
            ..BaseVoice::default()
        }
    }

    pub(super) fn reader_options(&self) -> ReaderOptions {
        let punctlist = self.options.punct.clone().unwrap_or_default();
        ReaderOptions {
            ssml: true,
            punctuation: match &self.options.punct {
                Some(list) if list.is_empty() => 1,
                Some(_) => 2,
                None => 0,
            },
            punctlist,
            capitals: u32::from(self.options.capitals),
            ..ReaderOptions::default()
        }
    }

    fn clause_ipa(&self, codes: &[PhonemeCode], preserve_punctuation: bool) -> Result<String> {
        let loaded = self.loaded_data()?;
        let codes = super::lower_stress_before_emphasis(&without_sourceless_gaps(codes), &loaded.phdata);
        let ipa = super::phoneme_ipa::codes_to_ipa_opts(
            &codes,
            &loaded.phdata,
            &self.data_dir,
            &self.options.lang,
            preserve_punctuation,
            self.options.ipa_tie,
            self.options.ipa_separator,
        );
        Ok(ipa.trim_end_matches('\n').replace('\n', " "))
    }

    fn clause_mnemonics(&self, codes: &[PhonemeCode], sep: Option<char>, tie: Option<char>) -> Result<String> {
        let loaded = self.loaded_data()?;
        let codes = super::lower_stress_before_emphasis(&without_sourceless_gaps(codes), &loaded.phdata);
        let x = crate::phoneme_text::format_mnemonics(&codes, &loaded.phdata, sep, tie, &self.data_dir, &self.options.lang);
        Ok(x.trim_end_matches('\n').replace('\n', " "))
    }
}

impl<'a> SsmlDocument<'a> {
    fn voice_for(&mut self, id: &str, voices: &[VoiceEntry]) -> usize {
        if let Some(ix) = self.others.iter().position(|(v, _)| v == id) {
            return ix + 1;
        }
        let file = id.split('+').next().unwrap_or(id);
        let lang = voices.iter().find(|v| v.file == file).map(|v| v.language.clone());
        if lang.as_deref() == Some(self.base.options.lang.as_str()) || self.base_id_is(file, voices) {
            return 0;
        }
        let dir = &self.base.data_dir;
        let tr = lang.and_then(|l| Translator::new(&l, Some(dir)).ok());
        match tr {
            Some(tr) => {
                self.others.push((id.to_string(), tr));
                self.others.len()
            }
            None => 0,
        }
    }

    fn base_id_is(&self, file: &str, voices: &[VoiceEntry]) -> bool {
        self.base.own_voice_entry(voices).is_some_and(|v| v.file == file)
    }
}

/// The voice file a voice id (`file` or `file+variant`) names.
fn voice_file_entry<'v>(voices: &'v [VoiceEntry], id: &str) -> Option<&'v VoiceEntry> {
    let file = id.split('+').next().unwrap_or(id);
    voices.iter().find(|v| v.file == file)
}

/// `voice_languages` after `LoadVoice`: every `language` line except `variant`.
fn language_names(entry: &VoiceEntry) -> Vec<String> {
    entry.languages.iter().filter(|(l, _)| l != "variant").map(|(l, _)| l.clone()).collect()
}

fn pause_factors(wpm: i32) -> (i32, i32) {
    let mut speed = crate::synthesize::cgen::speed::Speed::default();
    speed.set(3, wpm, &crate::synthesize::cgen::voice::WVoice::default());
    (speed.pause_factor, speed.clause_pause_factor)
}

/// The language hooks `ReadClause` reads off the translator.
pub(super) struct ClauseLang<'a>(pub(super) &'a Translator);

fn translator_name(lang: &str) -> u32 {
    let base = super::primary_bcp47_subtag(lang);
    base.bytes().take(3).fold(0, |acc, b| (acc << 8) | u32::from(b))
}

impl ReaderLang for ClauseLang<'_> {
    fn translator_name(&self) -> u32 {
        translator_name(&self.0.options.lang)
    }

    fn ordinal_dot(&self) -> bool {
        self.0.options.number_grammar.ordinals.dot_marks_ordinal
    }

    fn ignore_or_replace(&self, _c: u32) -> CharMap {
        CharMap::Keep
    }

    fn lookup_special(&self, word: &str) -> Option<String> {
        let dict = self.0.load_dictionary()?;
        crate::dictionary::lookup::lookup(&dict, word, &Default::default())?;
        let loaded = self.0.loaded_data().ok()?;
        let found = super::word_to_phonemes(word, &dict, &loaded.phdata, &loaded.stress_opts, &self.0.options);
        let phonemes: Vec<u8> = found.phonemes.into_iter().filter(|&b| b != 0).collect();
        (!phonemes.is_empty()).then(|| self.0.phoneme_mode_text(&phonemes)).flatten()
    }

    /// C `LookupCharName`: with `only`, the character's own list entry and nothing else.
    fn lookup_char_name(&self, c: u32, only: bool) -> String {
        let Some(c) = char::from_u32(c) else { return String::new() };
        let named = match only {
            true => self.lookup_special(&c.to_string()),
            false => self.0.char_name_phonemes(c).and_then(|ph| self.0.phoneme_mode_text(&ph)),
        };
        named.unwrap_or_default()
    }
}

impl Translator {
    /// C `DecodeWithPhonemeMode`: phonemes as text the translator reads back verbatim.
    fn phoneme_mode_text(&self, phonemes: &[u8]) -> Option<String> {
        let loaded = self.loaded_data().ok()?;
        let codes: Vec<PhonemeCode> =
            phonemes.iter().filter(|&&b| b != 0).map(|&b| PhonemeCode { code: b, ..Default::default() }).collect();
        let text = crate::phoneme_text::format_mnemonics(&codes, &loaded.phdata, None, None, &self.data_dir, &self.options.lang);
        Some(format!("[[{}]]", text.trim()))
    }
}
