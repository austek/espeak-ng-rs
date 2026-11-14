//! What the reader needs from the rest of the engine: the language hooks `ReadClause` reads off
//! the translator, the SSML tag processor, and the option and flag state they share.

use super::Terminator;

/// `option_*` values `ReadClause` reads (translate.h) plus the clause buffer size `N_TR_SOURCE`.
#[derive(Clone, Debug)]
pub struct ReaderOptions {
    pub ssml: bool,
    pub linelength: i32,
    pub phoneme_input: bool,
    pub punctuation: u8,
    pub punctlist: Vec<char>,
    pub capitals: u32,
    pub end_character_position: i32,
    pub skip_characters: i32,
    pub n_buf: usize,
}

impl Default for ReaderOptions {
    fn default() -> Self {
        ReaderOptions {
            ssml: false,
            linelength: 0,
            phoneme_input: false,
            punctuation: 0,
            punctlist: Vec::new(),
            capitals: 0,
            end_character_position: 0,
            skip_characters: 0,
            n_buf: 800,
        }
    }
}

/// Reader statics that `ProcessSsmlTag` and the embedded commands also write.
#[derive(Clone, Debug, Default)]
pub struct ReaderFlags {
    pub ignore_text: bool,
    pub audio_text: bool,
    pub sayas_mode: u32,
    pub sayas_start: usize,
    pub silence: bool,
    pub punctuation: u8,
    pub punctlist: Vec<char>,
    pub capitals: u32,
    pub clear_skipping_text: bool,
}

/// C `ProcessSsmlTag`: appends the tag's embedded commands to `out`. `None` continues the clause,
/// `Some` ends it with that terminator.
pub trait TagProcessor {
    fn process_tag(&mut self, tag: &[char], out: &mut String, flags: &mut ReaderFlags) -> Option<Terminator>;
    fn current_voice_id(&self) -> String;
}

/// Tag processor for plain text; never called.
pub struct NoTags;

impl TagProcessor for NoTags {
    fn process_tag(&mut self, _: &[char], out: &mut String, _: &mut ReaderFlags) -> Option<Terminator> {
        out.push(' ');
        None
    }
    fn current_voice_id(&self) -> String {
        String::new()
    }
}

pub enum CharMap {
    Keep,
    Replace(u32),
    Ignore,
}

/// Language hooks C reads from the `Translator` inside `ReadClause`.
pub trait ReaderLang {
    fn translator_name(&self) -> u32 {
        (u32::from(b'e') << 8) + u32::from(b'n')
    }
    fn ordinal_dot(&self) -> bool {
        false
    }
    fn lowercase_sentence(&self) -> bool {
        false
    }
    fn announce_punct_flag2(&self) -> bool {
        false
    }
    fn ignore_or_replace(&self, _c: u32) -> CharMap {
        CharMap::Keep
    }
    fn lookup_special(&self, _word: &str) -> Option<String> {
        None
    }
    fn lookup_char_name(&self, _c: u32, _only: bool) -> String {
        String::new()
    }
    fn soundicon(&self, _c: u32) -> Option<i32> {
        None
    }
    fn embedded_speed(&self) -> i32 {
        175
    }
}

#[derive(Default)]
pub struct DefaultLang;

impl ReaderLang for DefaultLang {}
