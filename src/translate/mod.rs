//! Text → phoneme code → IPA string translation pipeline.
//!
//! Rust port of key portions of:
//! - `translate.c` (1807 lines)
//! - `translateword.c` (1201 lines)
//! - `readclause.c` (1023 lines)
//! - `numbers.c` (1873 lines)
//! - `tr_languages.c` (1704 lines)
//!
//! # Pipeline
//! ```text
//! &str  (raw text)
//!   │  tokenize()           → Vec<Token>
//!   │  word_to_phonemes()   → per-word phoneme codes (dictionary + rules)
//!   │  set_word_stress()    → stress placement
//!   │  phonemes_to_ipa()    → IPA string
//!   ▼
//! String  (IPA)
//! ```
//!
//! # Main entry point
//! [`Translator::text_to_ipa`] handles the full pipeline.
//! [`Translator::translate_to_codes`] stops before IPA rendering and
//! returns raw [`PhonemeCode`] values for the synthesizer.

pub mod ipa_table;
pub mod phoneme_ipa;
pub mod compat;
pub mod ssml;
mod accent;
pub(crate) mod letters;
mod ordinal;

use std::path::{Path, PathBuf};

/// Return the default espeak-ng data directory.
///
/// Resolution order:
/// 1. `ESPEAK_DATA_PATH` environment variable.
/// 2. A directory named `espeak-ng-data` next to the currently running
///    executable (useful when the crate is used as a standalone binary).
/// 3. `/usr/share/espeak-ng-data` (system installation).
pub fn default_data_dir() -> String {
    // 1. Explicit environment variable overrides everything.
    if let Ok(path) = std::env::var("ESPEAK_DATA_PATH") {
        return path;
    }

    // 2. espeak-ng-data/ relative to the binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let local = dir.join("espeak-ng-data");
            if local.join("en_dict").exists() {
                return local.to_string_lossy().into_owned();
            }
        }
    }

    // 3. espeak-ng-data/ relative to the current working directory.
    {
        let cwd_local = std::path::Path::new("espeak-ng-data");
        if cwd_local.join("en_dict").exists() {
            if let Ok(abs) = cwd_local.canonicalize() {
                return abs.to_string_lossy().into_owned();
            }
        }
    }

    // 4. System-wide installation.
    "/usr/share/espeak-ng-data".to_string()
}

/// Normalise a voice / language tag to the form espeak-ng uses internally
/// (lowercase ASCII, hyphen separators).
///
/// Accepts common BCP-47-style input such as `en-us`, `en_US`, or `EN-us`.
pub fn normalize_voice_tag(s: &str) -> String {
    s.trim().replace('_', "-").to_ascii_lowercase()
}

/// Split a voice name into its base language tag and optional `+variant`
/// modifier — e.g. `en-us+f3` → (`en-us`, `Some("f3")`), `en` → (`en`, `None`).
///
/// eSpeak variant voices (`+f3`, `+whisper`, …) live in
/// `espeak-ng-data/voices/!v/` and only modify the *acoustic* voice (pitch,
/// formants, …); the base tag selects the language data (dictionary, phoneme
/// table).  Splitting here lets `-v en+f3` resolve the `en` data instead of
/// failing to find a nonexistent `en+f3_dict`.  A trailing/empty variant
/// (`en+`) yields `None`.
pub fn split_voice_variant(voice: &str) -> (&str, Option<&str>) {
    match voice.split_once('+') {
        Some((base, variant)) if !base.is_empty() => {
            (base, (!variant.is_empty()).then_some(variant))
        }
        _ => (voice, None),
    }
}

/// First subtag of a normalised BCP-47 tag (`en-gb` → `en`).
pub(crate) fn primary_bcp47_subtag(tag: &str) -> &str {
    tag.split('-').find(|part| !part.is_empty()).unwrap_or(tag)
}

/// Pick which `<stem>_dict` file to load: try the longest matching prefix of
/// `voice_tag` that exists under `data_dir`, then shorter prefixes (locale fall back
/// to language-only, e.g. `en-us` → `en_dict`).
/// The path of a compiled `<stem>_dict`, whichever layout the data directory
/// uses.
///
/// Upstream #2261 moves the dictionaries into a `dicts/` subdirectory; both
/// layouts are accepted so a data refresh that adopts it doesn't break every
/// voice at once. The flat path wins when both exist.
pub fn dict_path(data_dir: &Path, stem: &str) -> Option<std::path::PathBuf> {
    let flat = data_dir.join(format!("{stem}_dict"));
    if flat.exists() {
        return Some(flat);
    }
    let nested = data_dir.join("dicts").join(format!("{stem}_dict"));
    nested.exists().then_some(nested)
}

pub fn resolve_dict_stem(data_dir: &Path, voice_tag: &str) -> Option<String> {
    let parts: Vec<&str> = voice_tag
        .split('-')
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    for len in (1..=parts.len()).rev() {
        let stem = parts[..len].join("-");
        if dict_path(data_dir, &stem).is_some() {
            return Some(stem);
        }
    }
    // No `<tag>_dict`: a voice may borrow another language's dictionary via a
    // `dictionary <name>` directive (Norwegian Bokmål `nb` → `no`).
    let (dictionary, _) = crate::voices::voice_data_overrides(data_dir, voice_tag);
    if let Some(dict) = dictionary {
        if dict_path(data_dir, &dict).is_some() {
            return Some(dict);
        }
    }
    None
}

/// The phoneme-table name for a voice tag: the language code itself, unless the
/// voice file redirects it via a `phonemes <name>` directive (e.g. `nb` → `no`).
/// Returns the code unchanged for the common case (no directive).
pub fn resolve_phoneme_table(data_dir: &Path, voice_tag: &str) -> String {
    let (_, phonemes) = crate::voices::voice_data_overrides(data_dir, voice_tag);
    phonemes.unwrap_or_else(|| voice_tag.to_string())
}

/// Name printed inside a (xx) language-switch marker. A marker back to the base voice (target == lang) names the active phoneme table; any other marker names target.
pub fn switch_marker_name<'a>(phdata: &'a PhonemeData, target: &'a str, lang: &str) -> &'a str {
    if target == lang {
        phdata.active_table_name().unwrap_or(target)
    } else {
        target
    }
}

/// Select `lang`'s phoneme table into `phdata`, trying in turn: the tag itself,
/// its `phonemes <name>` voice directive, then the **primary BCP-47 subtag**.
/// The last covers regional locales that ship only as a base table — `pt-br`→
/// `pt`, `fr-ca`→`fr`, `en-gb`→`en` — which previously errored with "phoneme
/// table '…' not found" (upstream #2248).
pub fn select_phoneme_table(
    phdata: &mut PhonemeData,
    data_dir: &Path,
    lang: &str,
) -> Result<()> {
    let resolved = resolve_phoneme_table(data_dir, lang);
    let base = primary_bcp47_subtag(lang);
    // The voice file's own `phonemes <name>` directive wins over the bare tag:
    // `pt` (Portuguese (Portugal)) declares `phonemes pt-pt`, while the table
    // literally named `pt` is the *Brazilian* one — trying the tag first gave
    // European Portuguese the Brazilian table (`dente` → `dˈẽŋtʃy`).
    for candidate in [resolved.as_str(), lang, base] {
        if phdata.select_table_by_name(candidate).is_ok() {
            return Ok(());
        }
    }
    // Re-run the base selection to surface a clear error if nothing matched.
    phdata.select_table_by_name(base).map(|_| ())
}

/// Resolve an SSML `<voice>` selector (a language tag *or* a human-readable
/// voice name) to a language tag backed by data, or `None` if it can't be
/// matched (so the caller falls back to the document language rather than
/// erroring).  Mirrors the CLI `-v` resolution.
fn resolve_voice_lang(selector: &str, data_dir: &Path) -> Option<String> {
    if resolve_dict_stem(data_dir, selector).is_some() {
        return Some(selector.to_string());
    }
    let voices = crate::voices::list_voices(data_dir);
    let query = crate::voices::VoiceQuery { name: Some(selector.to_string()), ..Default::default() };
    crate::voices::find_voice(&voices, &query)
        .and_then(|v| resolve_dict_stem(data_dir, &v.language))
}

use crate::error::{Error, Result};
use crate::phoneme::load::PhonemeData;
use crate::dictionary::file::Dictionary;
use crate::dictionary::lookup::{lookup, lookup_dict2_follow, lookup_with_follow, LookupCtx};
use crate::dictionary::rules::is_letter_wc;
use crate::dictionary::rules::translate_rules_phdata;
use crate::dictionary::{
    FLAG_PREFIX_REMOVED, FLAG_SUFX, FLAG_SUFX_E_ADDED, FLAG_SUFFIX_REMOVED, FLAG_SUFFIX_VOWEL,
    FLAG_SUFX_S, LETTERGP_B, LETTERGP_VOWEL2, N_WORD_PHONEMES, SUFX_A, SUFX_E, SUFX_I, SUFX_M, SUFX_P,
};
use crate::dictionary::stress::{set_word_stress, promote_strend_stress, change_word_stress,
                               apply_word_final_devoicing, apply_alt_stress_upgrade,
                               apply_special_attribute2, StressOpts, STRESS_UNBOUNDED};

use ipa_table::{
    en_ipa_override,
    phoneme_ipa_lang,
    IPA_STRESS_PRIMARY, IPA_STRESS_SECONDARY,
    PendingStress, PHON_STRESS_P, PHON_STRESS_P2, PHON_STRESS_TONIC,
    PHON_STRESS_2, PHON_STRESS_3,
    PHON_STRESS_U, PHON_STRESS_D, PHON_STRESS_PREV,
    is_pause_code,
};

// ---------------------------------------------------------------------------
// Clause type flags
// Mirrors CLAUSE_TYPE_XXX from translate.h
// ---------------------------------------------------------------------------

bitflags::bitflags! {
    /// Encodes punctuation pause length, intonation shape, and clause type
    /// in a single u32 – exactly as the C code packs them.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ClauseFlags: u32 {
        /// Pause duration field (bits 0–11, units of 10ms).
        const PAUSE_MASK           = 0x0000_0FFF;
        /// Intonation type field (bits 12–14).
        const INTONATION_MASK      = 0x0000_7000;
        /// Optional space after punctuation.
        const OPTIONAL_SPACE_AFTER = 0x0000_8000;
        /// Phrase type field (bits 16–19).
        const TYPE_MASK            = 0x000F_0000;
        /// Punctuation character can appear inside a word (Armenian).
        const PUNCT_IN_WORD        = 0x0010_0000;
        /// Speak the name of the punctuation character.
        const SPEAK_PUNCT_NAME     = 0x0020_0000;
        /// Dot after the last word.
        const DOT_AFTER_LAST_WORD  = 0x0040_0000;
        /// Multiply CLAUSE_PAUSE by 320ms instead of 10ms.
        const PAUSE_LONG           = 0x0080_0000;
    }
}

/// Intonation pattern for a clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intonation {
    /// Falling intonation (`.`).
    FullStop,
    /// Rising–falling intonation (`,`).
    Comma,
    /// Rising intonation (`?`).
    Question,
    /// Emphatic intonation (`!`).
    Exclamation,
    /// No intonation marker.
    None,
}

/// Phrase / sentence boundary type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseType {
    /// No boundary.
    None,
    /// End of input.
    Eof,
    /// Language/voice switch.
    VoiceChange,
    /// Clause boundary (comma-class punctuation).
    Clause,
    /// Sentence boundary (period-class punctuation).
    Sentence,
}

/// How a clause ends — the Rust form of the `terminator` out-parameter added by
/// upstream's `espeak_TextToPhonemesWithTerminator` (1.53.0).
///
/// Callers that feed a phonemizer usually need to know whether the text they
/// just converted ended a sentence, a question, or nothing at all, which the
/// plain phoneme string does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClauseTerminator {
    /// No terminating punctuation (end of input).
    None,
    /// `,` `;` `:` and the other comma-class separators.
    Comma,
    /// `.` — sentence end.
    Period,
    /// `?`
    Question,
    /// `!`
    Exclamation,
}

impl ClauseTerminator {
    /// The punctuation character, or `None` when the text simply ran out.
    pub fn as_char(self) -> Option<char> {
        match self {
            ClauseTerminator::None => None,
            ClauseTerminator::Comma => Some(','),
            ClauseTerminator::Period => Some('.'),
            ClauseTerminator::Question => Some('?'),
            ClauseTerminator::Exclamation => Some('!'),
        }
    }

    /// Whether this terminator ends a *sentence* (upstream `CLAUSE_TYPE_SENTENCE`).
    pub fn is_sentence(self) -> bool {
        matches!(
            self,
            ClauseTerminator::Period | ClauseTerminator::Question | ClauseTerminator::Exclamation
        )
    }

    /// Classify a clause-terminating character.
    pub fn from_char(c: char) -> Self {
        // The same classification C makes in `clause_type_from_codepoint`, so
        // `।` and `。` end a sentence while `、` and `…` only break a clause.
        clause_punctuation_class(c).map_or(ClauseTerminator::Comma, |(class, _)| class)
    }
}

/// The terminator of the final clause in `text` — the trailing clause
/// punctuation, ignoring whitespace and closing brackets/quotes.
fn clause_terminator_of(text: &str) -> ClauseTerminator {
    for c in text.chars().rev() {
        if c.is_whitespace() || matches!(c, '"' | '\'' | ')' | ']' | '}' | '\u{201d}' | '\u{2019}') {
            continue;
        }
        return match c {
            '.' | '?' | '!' | ',' | ';' | ':' | '\u{3002}' | '\u{ff1f}' | '\u{ff01}'
            | '\u{ff0c}' | '\u{2026}' => ClauseTerminator::from_char(c),
            _ => ClauseTerminator::None,
        };
    }
    ClauseTerminator::None
}

/// A clause read from the input text.
#[derive(Debug, Clone)]
pub struct Clause {
    /// The raw UTF-8 text of the clause.
    pub text: String,
    /// How the clause ends (intonation pattern).
    pub intonation: Intonation,
    /// What kind of boundary follows.
    pub clause_type: ClauseType,
    /// Pause after the clause in milliseconds.
    pub pause_ms: u32,
}

// ---------------------------------------------------------------------------
// Language options
// ---------------------------------------------------------------------------

/// Language-specific translation options.
#[derive(Debug, Clone)]
pub struct LangOptions {
    /// BCP-47 language tag, e.g. `"en"`, `"en-us"`, `"fr"`, `"de"`
    pub lang: String,
    /// Words per minute (default 175)
    pub rate: u32,
    /// Base pitch (0–100, default 50)
    pub pitch: u32,
    /// Word gap in units of 10ms (`-g`).
    pub word_gap: i32,
    /// Language's own inter-word pause phoneme (upstream `langopts.word_gap & 7`,
    /// indexing `pause_phonemes[]`): `1` = `_|` (no-link), used by cmn/yue/zh so a
    /// syllable's final vowel isn't merged with the next word's consonant.  `0`
    /// (the default) inserts nothing.
    pub lang_word_gap: u8,
    /// Stress rule index (STRESSPOSN_XXX from translate.h)
    pub stress_rule: u8,
    /// Capital-letter indication (`-k`): 0 = none, 1 = a short capital *sound*
    /// (the `phonCAPITAL` phoneme, doubled for all-caps words), 2 = announce the
    /// word "capital".
    pub capitals: u8,
    /// Announce punctuation by name (`--punct`): `None` = off (default),
    /// `Some(empty)` = all punctuation, `Some(chars)` = only those characters.
    pub punct: Option<Vec<char>>,
    /// Language-specific number parsing and rendering behavior.
    pub number_grammar: NumberGrammar,
    /// Active dict-condition bitmask from the voice's `dictrules` directive —
    /// selects `?n` variant dict/rule groups (e.g. Brazilian Portuguese `?2`).
    pub dict_condition: u32,
    /// Reduce unstressed vowels even when the pronunciation was given in the
    /// dictionary — upstream `LOPT_REDUCE & 1`, set only for Italian.
    ///
    /// C's `StressCondition` uses this to refuse `ChangeIfNotStressed` on a
    /// dictionary-given word ("don't change phonemes which are given for the
    /// word in the dictionary").  Wiring that gate to this port's notion of
    /// "came from the dictionary" made Russian *lose* its reduction while
    /// Spanish kept its allophone, so the flag is parsed and carried but the
    /// gate is not applied — see GAPS.md for the measurement.
    pub reduce_dictionary_vowels: bool,
    /// The dictionary's default entry kind is **replacement text**, not phonemes,
    /// so `FLAG_TEXTMODE`'s meaning is reversed (upstream `langopts.textmode`,
    /// set for cmn/yue/zh: "the meaning of FLAG_TEXTMODE is reversed (to save
    /// data when *_list file is compiled)").  Chinese entries map a character to
    /// its pinyin reading — `你` → `ni3` — and the flag marks the *exceptions*
    /// that really are phonemes.  Reading the bytes as phoneme codes made every
    /// Han character gibberish (`你` → `qr`).
    pub reversed_textmode: bool,
    /// `--tie=<char>`: the character to insert inside a multi-letter phoneme
    /// name in IPA output (`tʃ` → `t͡ʃ`).  `None` = none, as by default.
    pub ipa_tie: Option<char>,
    /// `--sep=<char>`: the character to write *between* phonemes in IPA output.
    /// Upstream shares one slot with [`Self::ipa_tie`], so only one is ever set.
    pub ipa_separator: Option<char>,
    /// `langopts.param[LOPT_BRACKET_PAUSE]` / `[LOPT_BRACKET_PAUSE_ANNOUNCED]`:
    /// how long a bracket or quote pauses.  C's defaults are 4 and 2, and its
    /// pre-pause loop spends 2 per `phonPAUSE`, so 4 means two pauses.  Read
    /// from the voice file's `brackets` / `bracketsAnnounced` (Hungarian sets
    /// both to 0 — `hu "a ( b"` has no pause at all).
    pub bracket_pause: (u8, u8),
    /// `langopts.max_initial_consonants` — how many consonants may precede the
    /// first vowel before a word counts as unpronounceable and is spelled out
    /// letter by letter ("BBC" → `b,i:b,i:s'i:`).
    pub max_initial_consonants: usize,
    /// `langopts.param[LOPT_UNPRONOUNCABLE]`: `1` disables the test entirely,
    /// `2` delegates it to the language's own rules (C's `Unpronouncable2` — a
    /// word the rules can't read, or that a `$unpron` rule marks, is spelled),
    /// and any other value is a letter code to discount when counting.
    pub unpronouncable: u32,
    /// This translator re-reads a word another language handed over.  C applied
    /// the *first* language's `.replace` table while reading the clause, so the
    /// table of the language that reads the word is not applied again.
    pub switched: bool,
    /// For a switched translator: the word it re-reads ends the first language's
    /// clause, so end-of-clause stress applies.
    pub switched_last: bool,
}

const BREAK_THOUSANDS: u32 = 0x4924_9248;
const BREAK_MYRIADS: u32 = 0x1111_1110;
const BREAK_LAKH: u32 = 0xaaaa_aaa8;
const BREAK_LAKH_BN: u32 = 0x2492_4aa8;
const BREAK_LAKH_DV: u32 = 0x0000_14a8;
const BREAK_LAKH_HI: u32 = 0x0001_4aa8;
const BREAK_LAKH_UR: u32 = 0x0000_52a8;
const BREAK_INDIVIDUAL: u32 = 0x0000_0018;

/// Language-specific number rendering rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberGrammar {
    /// Ordinal parsing behavior.
    pub ordinals: OrdinalGrammar,
    /// How tens and units are combined.
    pub tens: TensGrammar,
    /// Rules for hundreds.
    pub hundreds: HundredsGrammar,
    /// Rules for thousands.
    pub thousands: ThousandsGrammar,
    /// Digits attached to a word are **tone numbers**, not a separate number:
    /// the Chinese dictionaries store readings as pinyin + tone (`你` → `ni3`),
    /// and splitting that into "ni" + the number 3 made every Han character read
    /// as a word plus "three" (`你` → `qr` … `sˈan`).
    pub tone_numbers: bool,
    /// Read the fractional digits as **one number** rather than digit by digit —
    /// upstream `NUM_DFRACTION_2` (pl/cs/fi/tr/pt/…: "3,14" is "trzy przecinek
    /// **czternaście**") and `NUM_DFRACTION_4` (es/fr/lt/vi/…, same rule with a
    /// longer limit).  Leading zeros are still spoken one at a time, and a
    /// fraction longer than the limit falls back to digit-by-digit.  `0` = off.
    pub fraction_digits_as_number: u8,
    /// Read a fraction of any length as one number unless it starts with a zero
    /// (which falls back to single digits) — upstream `NUM_DFRACTION_1`
    /// (Italian: "3,14" is "tre virgola **quattordici**").
    pub fraction_whole_number: bool,
    /// Read a decimal fraction as a *fraction*: numerator + a "tenths / hundredths
    /// / thousandths" suffix, instead of digit-by-digit.  Upstream
    /// `NUM_DFRACTION_5` (ru, hu, ka), driven by the `_0Z<n>` / `_0Z<n>s`
    /// dictionary entries — `ru "0,25"` is "ноль и двадцать пять сотых", not
    /// "ноль и два пять".
    pub fraction_suffix: bool,
    /// The fraction numerator uses feminine forms of 1 and 2 (`_1f`, `_2f`) and
    /// takes the singular suffix when it ends in 1 (but not 11) — upstream
    /// `NUM2_FRACTION_FEMININE`: "одна сотая", "две сотых", "двадцать одна сотая".
    pub fraction_feminine: bool,
    /// Digit-group (thousands) separator, if the language groups large numbers —
    /// e.g. `Some(',')` for English so `1,000` reads as one number.  `None` (the
    /// default) leaves grouping characters as ordinary punctuation.
    pub group_separator: Option<char>,
    /// Decimal-point character — `'.'` in English, `','` in most of continental
    /// Europe (`3,14`).  Must differ from `group_separator`.
    pub decimal_separator: char,
    /// Vigesimal 70s/90s (France French): 70–79 = "soixante" + 10–19,
    /// 90–99 = "quatre-vingt" + 10–19 (there is no simple word for 70 or 90).
    pub vigesimal_70_90: bool,
    /// Whether a space between digit groups marks thousands (`1 234 567`) — the
    /// SI/ISO standard used across continental Europe.  Enabled for the
    /// comma-decimal languages (where a space is unambiguous); English keeps a
    /// space as an ordinary separator.
    pub space_group: bool,
    /// Generate Portuguese cardinals as text and pronounce them via the rules
    /// (the pt dictionary is missing most number keys — `1`/`5`/`100`/`1000` come
    /// out empty or wrong).  Set for the `pt` voice.
    pub portuguese_cardinals: bool,
    /// C's `NUM_1900`: read 1900–1999 as "nineteen hundred (and) …".  Only ten
    /// languages ask for it (af, da, en, fo, is, kl, nb/no, nl, sv); the rest
    /// say "one thousand nine hundred …".
    pub year_1900: bool,
    /// Split a word at a lowercase→uppercase boundary (`CamelCase` → "Camel
    /// Case", `iPhone` → "i Phone"), mirroring espeak's `TranslateClause`
    /// behaviour (a space is inserted before the uppercase letter).  Universal
    /// in espeak; only Irish (`ga`), whose lenition prefixes legitimately mix
    /// case, disables it — so this is `true` by default and `false` for `ga`.
    pub caps_word_split: bool,
    /// C's `LOPT_APOSTROPHE` (also the voice files' `apostrophe` directive):
    /// bit 0 keeps an apostrophe that *starts* a word, bit 1 one that *ends* it.
    /// Only Hebrew sets it (bit 1, for `בָּגָאז׳`).  Everywhere else an
    /// apostrophe belongs to the word only *between* two letters, which is what
    /// keeps `don't` whole while `hello'` is just "hello".
    pub apostrophe: u8,
    /// Treat a capital letter as a *stress marker* rather than a word break
    /// (espeak `LOPT_CAPS_IN_WORD`).  The first capital in a word inserts a
    /// primary-stress mark `ˈ` before it and the word stays whole (`bAstu` →
    /// "bˈastu").  Mutually exclusive with `caps_word_split`; set only for
    /// Lojban (`jbo`), where capitals denote the stressed syllable.
    pub caps_mark_stress: bool,
    /// Dutch (`nl`) exception to the upper→lower CamelCase split: a word-initial
    /// "IJ" digraph (`IJssel`) is not split.  Mirrors espeak's
    /// `translator_name == L('n','l')` guard.
    pub dutch_ij: bool,
    /// Italian tens-vowel elision: a tens word (trenta/quaranta/…) drops its
    /// final vowel before a vowel-initial unit (uno/otto) → "trentuno",
    /// "quarantotto".  The 20s already elide via dedicated `_21`/`_28` entries.
    pub elide_tens_vowel: bool,
    /// Slavic scale-word declension (gender + case agreement on "thousand",
    /// "million", …).  See `SlavicThousands`.
    pub thousands_variant: SlavicThousands,
    /// Longest run of 5 to 32 digits read as a cardinal; longer runs up to 32
    /// digits are spelled in groups (upstream `langopts.max_digits`).
    pub max_digits: u8,
    /// Bit `n` set: a digit group ends when `n` digits remain, for the
    /// digit-by-digit reading (upstream `langopts.break_numbers`).
    pub break_numbers: u32,
    /// C's `langopts.numbers` bitmask for the language.
    pub c_numbers: u32,
    /// C's `langopts.numbers2` bitmask for the language.
    pub c_numbers2: u32,
    /// Number symbols (vulgar fractions) of the text that the language's list
    /// names itself (`pt_list`: `¼ u~Nku'a*@-tU`); [`normalize_number_symbols`]
    /// leaves them for the dictionary lookup.  Filled per translation by
    /// [`with_dict_ordinal_suffixes`].
    pub listed_symbols: Vec<char>,
}

/// Slavic scale-word (thousand/million/…) declension.  The scale word takes a
/// different case by the count before it, and the count's "one"/"two" agree in
/// gender with the scale word — Russian "одна тысяча" (1000, fem), "две тысячи"
/// (2000, fem gen-sg), "пять тысяч" (5000, gen-pl), "один миллион" (masc).
/// (espeak's `M_Variant` / `NUM2_THOUSANDS_VAR*`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlavicThousands {
    /// No Slavic declension.
    #[default]
    None,
    /// Russian: scale word `_1MA{n}` for count≡1, `_0MA{n}` for 2–4, `_0M{n}`
    /// otherwise (mod 10, excluding the 11–19 teens); "thousand" is feminine, the
    /// higher scales masculine; the "one" is spoken (not omitted).
    Ru,
    /// Czech: `_0MA{n}` only for count exactly 2–4, else `_0M{n}`; no gender; the
    /// "one" is omitted ("tisíc", "dva tisíce", "pět tisíc").
    Cs,
    /// Polish: `_0MA{n}` for count ≡ 2–4 (mod 10, not the 11–19 teens), else
    /// `_0M{n}`; no gender; the "one" is omitted ("tysiąc", "dwa tysiące").
    Pl,
    /// Lithuanian: `_0MB{n}` (genitive plural "tūkstančių") for a teen count or
    /// count ≡ 0 mod 10, `_0MA{n}` ("tūkstantis") for count ≡ 1 mod 10, else
    /// `_0M{n}` ("tūkstančiai"); no gender; count 1 uses `_1M{n}`.
    Lt,
    /// Slovak: same case rule as Czech (`_0MA{n}` for count exactly 2–4), but
    /// "thousand" is feminine only for a bare single-digit count ("dve tisíc" at
    /// 2000, yet "dvadsať dva tisíc" at 22000 and "dva milióny" for millions).
    Sk,
    /// Croatian: `_1M{n}` (the count-1 word "tisuću") for count ≡ 1 mod 10,
    /// `_0MA{n}` ("tisuće") for count ≡ 2–4, else `_0M{n}` ("tisuća"); "thousand"
    /// is feminine (dvije) for any count, like Russian.
    Hr,
}

/// Ordinal marker recognition behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OrdinalGrammar {
    /// Suffix which marks ordinals even without a `_#suffix` dict entry.
    pub indicator: Option<String>,
    /// Whether `3.`-style ordinals are accepted.
    pub dot_marks_ordinal: bool,
    /// Germanic compound ordinals: a compound like 21 forms its ordinal as the
    /// whole *cardinal* + the ordinal suffix ("einundzwanzig" + "ste" =
    /// "einundzwanzigste"), not tens-ordinal + units-ordinal ("zwanzig erste").
    pub compound_cardinal_suffix: bool,
    /// Suffixes in the text that the dictionary defines as ordinal markers
    /// (`_#de` in Afrikaans, `_#ος` in Greek); filled per translation by
    /// [`with_dict_ordinal_suffixes`].
    pub dict_suffixes: Vec<String>,
    /// Hungarian: a dot before a word the dictionary flags `$alt` (a month
    /// name) is part of a date, not an ordinal; `declines_alt_words` turns the
    /// check on and [`with_dict_ordinal_suffixes`] fills `alt_words`.
    pub declines_alt_words: bool,
    pub alt_words: Vec<String>,
    /// Hyphenated suffixes (`-ig`) whose `_#` entry is `$alt2`: Hungarian
    /// applies them only after an `$alt` word.
    pub alt2_suffixes: Vec<String>,
}

/// Word-order rule for tens and units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TensGrammar {
    /// `thirty four`
    #[default]
    Standard,
    /// `treinta y cuatro`
    WithConjunction,
    /// `vier und dreißig`
    UnitsThenConjunction,
}

/// Hundreds-specific rendering behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HundredsGrammar {
    /// Whether to insert a conjunction between the hundreds and remainder,
    /// unconditionally (English "one hundred AND thirty-four").
    pub use_conjunction_with_remainder: bool,
    /// Insert the conjunction between hundreds and remainder only when the
    /// remainder is a *single word* — a unit, a single-word teen, or a round ten
    /// — but not a compound that already carries its own conjunction.  Afrikaans
    /// "honderd EN een" / "honderd EN twintig" but "honderd vier-en-dertig";
    /// Swahili "mia moja NA moja" but "mia moja thelethini na nne".
    pub conjunction_before_simple_remainder: bool,
    /// Put the conjunction in front of the hundreds of a group that follows a
    /// higher group — upstream `NUM_AND_HUNDRED`.
    pub conjunction_before_hundreds: bool,
    /// Put the conjunction in front of a scaled group without hundreds that
    /// follows a higher group — upstream `NUM_THOUSAND_AND`.
    pub conjunction_before_bare_group: bool,
    /// Whether to omit the explicit `one` before `hundred` (in the compose path,
    /// `_{n}` + `_0C`): French "cent", not "un cent".
    pub omit_one_prefix: bool,
    /// For a bare 1-hundred, skip the dedicated `_1C`/`_1C0` word (which bakes in
    /// the "one") and use the bare hundred word instead — `_0C0` for an exact
    /// hundred, else `_0C`.  Malayalam "100" = "നൂറ്" (`_0C0`), not "ഒരു നൂറ്"
    /// (`_1C0`); "200"+ keep their dedicated `_2C0`/`_2C` forms.  (espeak's
    /// `NUM2_OMIT_1_HUNDRED_ONLY`.)
    pub omit_one_hundred_word: bool,
}

/// Thousands-specific rendering behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThousandsGrammar {
    /// Whether to omit the explicit `one` before `thousand`.
    pub omit_one_prefix: bool,
}

impl NumberGrammar {
    fn for_lang(lang: &str) -> Self {
        let mut grammar = Self::default();
        let lang = primary_bcp47_subtag(lang);
        match lang {
            "en" => {
                grammar.hundreds.use_conjunction_with_remainder = true;
                grammar.group_separator = Some(','); // 1,000 → one thousand
            }
            "es" => {
                grammar.tens = TensGrammar::WithConjunction;
                grammar.hundreds.omit_one_prefix = true;
                grammar.thousands.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
            }
            "fr" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.vigesimal_70_90 = true; // soixante-dix, quatre-vingt-dix
            }
            "it" => {
                // "cento", not "uno cento" (1000 is already "mille").
                grammar.hundreds.omit_one_prefix = true;
                grammar.elide_tens_vowel = true; // "trentuno", "quarantotto"
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
            }
            "de" => {
                grammar.ordinals.dot_marks_ordinal = true;
                grammar.ordinals.compound_cardinal_suffix = true;
                grammar.tens = TensGrammar::UnitsThenConjunction;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                // German keeps the "ein": upstream reads 100 as "einhundert" and
                // 1000 as "ein tausend" (its `langopts.numbers` sets neither
                // `NUM_OMIT_1_HUNDRED` nor `NUM_OMIT_1_THOUSAND`).
            }
            // Chinese / Cantonese / Hakka: dictionary readings are pinyin with a
            // trailing tone digit, so digits glued to letters stay in the word.
            "cmn" | "yue" | "hak" | "zh" => {
                grammar.tone_numbers = true;
            }
            // (the reversed-textmode flag for these is set on LangOptions below)
            // Russian: "сто"/"тысяча", not "один сто"/"один тысяча".
            "ru" | "bg" | "uk" | "be" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.thousands.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                if lang == "ru" {
                    // Decimal fractions are read as fractions with feminine
                    // numerators — "0,25" = "ноль и двадцать пять сотых"
                    // (upstream NUM_DFRACTION_5 | NUM2_FRACTION_FEMININE).
                    grammar.fraction_suffix = true;
                    grammar.fraction_feminine = true;
                    // Russian scale words decline (gender + case): "одна тысяча",
                    // "две тысячи", "пять тысяч", "один миллион".  The "one" is
                    // spoken (unlike bg/uk/be), so undo the omit set above.
                    grammar.thousands_variant = SlavicThousands::Ru;
                    grammar.thousands.omit_one_prefix = false;
                }
            }
            // Turkish/Hungarian: "yüz"/"bin", "száz"/"ezer" — no "one" prefix.
            "tr" | "hu" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.thousands.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                // Hungarian also always names the fraction ("tizedes"), without
                // the feminine numerator forms Russian uses.
                grammar.fraction_suffix = lang == "hu";
            }
            // Slovak/Croatian/Serbian: "sto" not "jedensto" (1000 already "tisíc"/
            // "tisuću"/"hiljadu").
            "sr" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.decimal_separator = ',';
            }
            // Croatian: the scale word declines — "tisuću" (…1), "tisuće" (…2–4),
            // "tisuća" (5+) — and "thousand" is feminine ("dvije tisuće").
            "hr" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                grammar.thousands_variant = SlavicThousands::Hr;
            }
            // Slovak: the scale word declines for count 2–4 ("dva milióny"), and
            // "thousand" is feminine for a bare 2 ("dve tisíc").
            "sk" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                grammar.thousands_variant = SlavicThousands::Sk;
            }
            // Macedonian: "сто"/"илјада", not "еден сто"/"еден илјада".
            "mk" => {
                grammar.hundreds.omit_one_prefix = true;
                grammar.thousands.omit_one_prefix = true;
                grammar.decimal_separator = ',';
            }
            // Arabic/Persian/Urdu: comma groups thousands (the native `٬` is
            // normalised to `,`; the native `٫` decimal maps to `.`).
            "ar" | "fa" | "ur" => {
                grammar.group_separator = Some(',');
            }
            "nl" | "mt" => {
                grammar.ordinals.dot_marks_ordinal = true;
                grammar.ordinals.indicator = Some("e".to_string());
                // Dutch, like German, suffixes the whole cardinal for compound
                // ordinals ("eenentwintigste"); Maltese ordinals differ, so it's
                // Dutch-only.
                grammar.ordinals.compound_cardinal_suffix = lang == "nl";
                grammar.tens = TensGrammar::UnitsThenConjunction;
                grammar.hundreds.omit_one_prefix = true;
                grammar.thousands.omit_one_prefix = true;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
            }
            // Portuguese: comma decimal / period thousands like the rest of the
            // Romance set.  (`portuguese_cardinals` spelled the numbers out as
            // text because the dict looked key-less; it is not — pt's `_N`
            // entries are behind `?1`/`?2` conditions, which the port now
            // honours, so the generic machinery reads them.)
            // `NUM_AND_UNITS | NUM_HUNDRED_AND`: the conjunction "e" joins
            // tens→units ("vinte E um") and hundreds→remainder ("cento E um"),
            // and `NUM_DFRACTION_2` reads two fraction digits as one number.
            "pt" => {
                grammar.portuguese_cardinals = false;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.tens = TensGrammar::WithConjunction;
                grammar.hundreds.use_conjunction_with_remainder = true;
                grammar.fraction_digits_as_number = 2;
            }
            // Continental-European comma-decimal languages.
            // Units-first cardinals: Danish/Faroese "enogtyve", Slovenian
            // "enaindvajset" (units + "og"/"in" + tens), like German — and their
            // compound ordinals suffix the whole cardinal ("enogtyvende").
            "da" | "fo" | "kl" | "sl" => {
                grammar.ordinals.dot_marks_ordinal = true;
                grammar.tens = TensGrammar::UnitsThenConjunction;
                grammar.ordinals.compound_cardinal_suffix = true;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                // C `NUM_HUNDRED_AND` — "hundrede OG en", "tusind OG en".
                // Slovenian is the one of the four without it.
                grammar.hundreds.use_conjunction_with_remainder = lang != "sl";
                // C `NUM_OMIT_1_HUNDRED` for all but Danish.
                grammar.hundreds.omit_one_prefix = lang != "da";
            }
            // Tens-first, dot-ordinal, comma-decimal (Finnish, Estonian,
            // Lithuanian, Norwegian "tjueen", Greenlandic).
            "et" | "fi" | "nb" | "no" => {
                grammar.ordinals.dot_marks_ordinal = true;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                // Norwegian has C's `NUM_HUNDRED_AND` ("nitten hundre OG …");
                // Finnish and Estonian have no conjunction at all.
                grammar.hundreds.use_conjunction_with_remainder =
                    matches!(lang, "nb" | "no");
            }
            // Lithuanian: like the above, plus the scale word declines —
            // "tūkstantis" (1/…1), "tūkstančiai" (…2–9), "tūkstančių" (teens/×10).
            "lt" => {
                grammar.ordinals.dot_marks_ordinal = true;
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.thousands_variant = SlavicThousands::Lt;
            }
            // More comma-decimal languages (period/space thousands).  Only the
            // decimal + group separators are set here — the ordinal-dot marker
            // varies (Swedish "3:e", Romanian "al 3-lea"…), so it's left off.
            // Romanian & Icelandic: a conjunction joins tens and units
            // ("treizeci ȘI patru", "þrjátíu OG fjórir") via the dict `_0and`.
            "ro" | "is" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.tens = TensGrammar::WithConjunction;
                // Icelandic also joins hundreds→remainder, but only when the
                // remainder is a single word (`NUM_SINGLE_AND | NUM_HUNDRED_AND`):
                // "hundrað OG einn", but "nítján hundruð áttatíu og fjórir".
                grammar.hundreds.conjunction_before_simple_remainder = lang == "is";
            }
            // Albanian: the conjunction "e" joins both tens→unit and
            // hundreds→remainder ("njëzet E një", "njëqind E tridhjetë").
            "sq" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.tens = TensGrammar::WithConjunction;
                grammar.hundreds.use_conjunction_with_remainder = true;
            }
            // Armenian: "100" is "harjur", not "mek harjur".
            "hy" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.hundreds.omit_one_prefix = true;
            }
            // Kazakh: "100" is "жүз", not "бір жүз" (but "1000" keeps "бір мың").
            // Kazakh keeps the period decimal separator ("3,14" is two numbers).
            "kk" => {
                grammar.hundreds.omit_one_prefix = true;
            }
            // Azerbaijani: "100"/"1000" are "yüz"/"min", not "bir yüz"/"bir min".
            "az" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.hundreds.omit_one_prefix = true;
                grammar.thousands.omit_one_prefix = true;
            }
            // Afrikaans: Germanic units-first tens ("vier-en-dertig"), like Dutch,
            // but keeps "een honderd" / "een duisend" (no omit-one prefix) and puts
            // "en" before a single-word remainder ("een honderd en een/twintig").
            // (Afrikaans reads "3,14" as two numbers — its decimal separator is
            // the period, like English: `af "3.14"` → "drie punt een vier".)
            "af" => {
                grammar.tens = TensGrammar::UnitsThenConjunction;
                grammar.hundreds.conjunction_before_simple_remainder = true;
            }
            // Czech: the scale word declines for count exactly 2–4 ("dva tisíce",
            // "dva milióny"); the "one" word (`_1M1`) is spoken for 1000.
            "cs" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.thousands_variant = SlavicThousands::Cs;
            }
            // Polish: the scale word declines for count ≡ 2–4 mod 10 ("dwa
            // tysiące"), else the genitive-plural form ("pięć tysięcy").
            "pl" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
                grammar.thousands_variant = SlavicThousands::Pl;
            }
            // (Georgian is *not* here: it reads "3,14" as two numbers and uses the
            // period as its decimal separator, as upstream does.)
            "sv" | "el" | "lv" | "ca" | "eu"
            | "bs" | "id" | "vi" => {
                grammar.decimal_separator = ',';
                grammar.group_separator = Some('.');
            }
            // Welsh: "100" is "cant", not "un cant" (period decimal, unchanged).
            "cy" => {
                grammar.hundreds.omit_one_prefix = true;
            }
            // Amharic: "100" is "መቶ", not "አንድ መቶ" (but "1000" keeps "አንድ ሺ").
            "am" => {
                grammar.hundreds.omit_one_prefix = true;
            }
            // Swahili: "na" joins tens and units ("ishirini na moja"), and also
            // hundreds→remainder when the remainder is a single word ("mia moja
            // na moja/kumi/ishirini", but "mia moja thelethini na nne").
            "sw" => {
                grammar.tens = TensGrammar::WithConjunction;
                grammar.hundreds.conjunction_before_simple_remainder = true;
            }
            // Malayalam: "100" is the bare "നൂറ്" (not "ഒരു നൂറ്"); 200+ keep
            // their dedicated words.  "1000" is the bare "ആയിരം" (no "ഒന്ന്").
            "ml" => {
                grammar.hundreds.omit_one_hundred_word = true;
                grammar.thousands.omit_one_prefix = true;
            }
            // Tamil / Sinhala: "1000" is "ஆயிரம்" / "අහස", not "ஒன்று ஆயிரம்" /
            // "එක අහස" (the "one" is dropped before "thousand").
            "ta" | "si" => {
                grammar.thousands.omit_one_prefix = true;
            }
            _ => {}
        }
        // Space (SI/ISO) thousands grouping is unambiguous exactly where the
        // decimal point is a comma — all the continental-European languages.
        // English (comma-group/period-decimal) and Arabic-style (comma-group)
        // keep a space as an ordinary separator.
        grammar.space_group = grammar.decimal_separator == ',';
        // Irish keeps lenition prefixes ("tAthair", "nGael") whole — the only
        // language whose `UpperCaseInWord` suppresses the CamelCase split.
        grammar.caps_word_split = lang != "ga";
        // Lojban writes the stressed syllable with capitals (`LOPT_CAPS_IN_WORD`)
        // — a capital marks stress instead of splitting the word.
        grammar.caps_mark_stress = lang == "jbo";
        // Dutch keeps a word-initial "IJ" digraph intact under the upper→lower
        // split.
        grammar.dutch_ij = lang == "nl";
        grammar.fraction_digits_as_number = match primary_bcp47_subtag(lang) {
            // NUM_DFRACTION_2 — up to two digits read as a number.
            "et" | "fi" | "he" | "hr" | "bs" | "sr" | "mk" | "pl" | "pt" | "sk" | "cs"
            | "sl" | "smj" | "tr" | "az" => 2,
            // NUM_DFRACTION_4 — the same, up to five digits.
            "es" | "an" | "ca" | "ia" | "pap" | "fr" | "ht" | "lt" | "lv" | "ltg" | "sq"
            | "tt" | "vi" => 5,
            _ => 0,
        };
        grammar.fraction_whole_number = primary_bcp47_subtag(lang) == "it";
        // C `NUM_HUNDRED_AND` for the languages whose arm above doesn't set it:
        // a conjunction joins hundreds→remainder ("mi'a WA wahid", "sto I edno").
        // `NUM_SINGLE_AND` (Bulgarian, Hebrew) restricts it to a remainder that
        // is a single word, as it does for Icelandic.
        if matches!(lang, "ar" | "eu" | "fa" | "io" | "ku" | "om" | "tn") {
            grammar.hundreds.use_conjunction_with_remainder = true;
        }
        if matches!(lang, "bg" | "he") {
            grammar.hundreds.conjunction_before_simple_remainder = true;
        }
        grammar.hundreds.conjunction_before_hundreds = matches!(lang, "ar" | "ku");
        grammar.hundreds.conjunction_before_bare_group = lang == "ar";
        // C `NUM_AND_UNITS`: a conjunction joins tens→units ("si O chahar",
        // "shloshim VE arba").  Arabic also has `NUM_SWAP_TENS`, so its units
        // come first ("arba'a WA thalathun").
        if matches!(lang, "bg" | "fa" | "he" | "io" | "ka" | "ku" | "mk" | "tn") {
            grammar.tens = TensGrammar::WithConjunction;
        }
        if lang == "ar" {
            grammar.tens = TensGrammar::UnitsThenConjunction;
        }
        grammar.max_digits = match lang {
            "en" | "lv" | "ltg" => 33,
            "ka" | "ru" | "uk" => 32,
            "ne" => 22,
            "ko" => 20,
            _ => 14,
        };
        grammar.break_numbers = match lang {
            "bn" | "as" | "bpy" => BREAK_LAKH_BN,
            "hi" | "or" | "pa" | "gu" | "mr" | "si" => BREAK_LAKH_HI,
            "ne" => BREAK_LAKH,
            "ur" | "sd" => BREAK_LAKH_UR,
            "kn" | "ml" | "ta" | "te" => BREAK_LAKH_DV,
            "ko" => BREAK_MYRIADS,
            "shn" | "yue" => BREAK_INDIVIDUAL,
            _ => BREAK_THOUSANDS,
        };
        // C `LOPT_APOSTROPHE`: Hebrew keeps a word-final apostrophe.
        grammar.apostrophe = if lang == "he" { 2 } else { 0 };
        // C `NUM_1900`: 1900–1999 read as "nineteen hundred …".
        grammar.year_1900 = matches!(
            lang,
            "af" | "da" | "en" | "fo" | "is" | "kl" | "nb" | "nl" | "no" | "sv"
        );
        (grammar.c_numbers, grammar.c_numbers2) = ordinal::c_number_flags(lang);
        (grammar.group_separator, grammar.decimal_separator, grammar.space_group) =
            ordinal::c_separators(lang, grammar.c_numbers);
        grammar.ordinals.dot_marks_ordinal |= grammar.c_numbers & ordinal::NUM_ORDINAL_DOT != 0;
        grammar.ordinals.declines_alt_words = lang == "hu";
        grammar
    }
}

impl Default for NumberGrammar {
    fn default() -> Self {
        Self {
            ordinals: OrdinalGrammar::default(),
            tens: TensGrammar::Standard,
            hundreds: HundredsGrammar::default(),
            thousands: ThousandsGrammar::default(),
            tone_numbers: false,
            fraction_digits_as_number: 0,
            fraction_whole_number: false,
            fraction_suffix: false,
            fraction_feminine: false,
            group_separator: None,
            decimal_separator: '.',
            vigesimal_70_90: false,
            space_group: false,
            portuguese_cardinals: false,
            year_1900: false,
            apostrophe: 0,
            caps_word_split: true,
            caps_mark_stress: false,
            dutch_ij: false,
            elide_tens_vowel: false,
            thousands_variant: SlavicThousands::None,
            max_digits: 14,
            break_numbers: BREAK_THOUSANDS,
            c_numbers: 1,
            c_numbers2: 0,
            listed_symbols: Vec::new(),
        }
    }
}

impl Default for LangOptions {
    fn default() -> Self {
        LangOptions {
            lang:        "en".to_string(),
            rate:        175,
            pitch:       50,
            word_gap:    0,
            lang_word_gap: 0,
            stress_rule: 2, // STRESSPOSN_2R = penultimate
            capitals:    0,
            punct:       None,
            number_grammar: NumberGrammar::default(),
            dict_condition: 0,
            reduce_dictionary_vowels: false,
            reversed_textmode: false,
            bracket_pause: (4, 2),
            ipa_tie: None,
            ipa_separator: None,
            max_initial_consonants: 3,
            unpronouncable: 0,
            switched: false,
            switched_last: false,
        }
    }
}

/// Whether `--punct` is active and covers character `c`.  An empty set means
/// "all punctuation" (`--punct` with no argument); the SSML clause-break
/// sentinel is never announced.
fn punct_covers(opts: &LangOptions, c: char) -> bool {
    if c == ssml::SSML_BREAK {
        return false;
    }
    match &opts.punct {
        Some(chars) => chars.is_empty() || chars.contains(&c),
        None => false,
    }
}

impl LangOptions {
    pub fn for_lang(lang: &str) -> Self {
        let lang = normalize_voice_tag(lang);
        // cmn/yue/zh compile their dictionaries as replacement *text* (hanzi →
        // pinyin), so `FLAG_TEXTMODE` marks the phoneme entries instead
        // (upstream `langopts.textmode`).
        let reversed_textmode =
            matches!(primary_bcp47_subtag(&lang), "cmn" | "yue" | "zh");
        // `LOPT_UNPRONOUNCABLE`, transcribed from `tr_languages.c`.
        let unpronouncable = match primary_bcp47_subtag(&lang) {
            "am" | "ar" | "be" | "chr" | "el" | "grc" | "fa" | "ko" | "si" | "ur" | "sd"
            | "cmn" | "yue" | "zh" => 1,
            "de" | "en" | "es" | "an" | "ca" | "ia" | "pap" => 2,
            "ga" | "gd" => 3,
            "bg" => 0x432,
            "sl" => 0x76,
            _ => 's' as u32,
        };
        let max_initial_consonants = match primary_bcp47_subtag(&lang) {
            "az" | "kk" | "ku" | "tr" => 2,
            "sw" | "tn" => 4,
            "bs" | "cmn" | "cs" | "hr" | "sk" | "sr" | "yue" | "zh" => 5,
            "hy" => 6,
            "ka" | "pl" => 7,
            _ => 3,
        };
        // `langopts.word_gap & 7` — the language's own inter-word pause phoneme
        // (upstream `pause_phonemes[]`): `0x21` for Chinese and Vietnamese
        // (→ `_|`), `1` for Mongolian, `0x8` for German (no pause bits set).
        let lang_word_gap = match primary_bcp47_subtag(&lang) {
            "cmn" | "yue" | "zh" | "vi" => 1,
            "mn" => 1,
            _ => 0,
        };
        let reduce_dictionary_vowels = primary_bcp47_subtag(&lang) == "it";
        Self {
            reduce_dictionary_vowels,
            number_grammar: NumberGrammar::for_lang(&lang),
            reversed_textmode,
            bracket_pause: (4, 2),
            ipa_tie: None,
            ipa_separator: None,
            max_initial_consonants,
            unpronouncable,
            lang_word_gap,
            lang,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// CJK character detection
// ---------------------------------------------------------------------------

/// Returns `true` if `c` is a CJK ideographic character.
///
/// These characters should each form an individual word token,
/// matching the C espeak-ng behaviour for languages with `words 1`
/// (e.g. Chinese, Japanese Kanji, Korean Hanja).
fn is_cjk_ideograph(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
    // CJK Unified Ideographs Extension A
    || (0x3400..=0x4DBF).contains(&cp)
    // CJK Unified Ideographs Extension B-H
    || (0x20000..=0x323AF).contains(&cp)
    // CJK Compatibility Ideographs
    || (0xF900..=0xFAFF).contains(&cp)
    // CJK Radicals / Kangxi
    || (0x2F00..=0x2FDF).contains(&cp)
}

// ---------------------------------------------------------------------------
// Token types for the simple tokenizer
// ---------------------------------------------------------------------------

/// One token produced by [`tokenize`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A word (sequence of letters / digits / apostrophes).
    Word(String),
    /// A parsed number-like token.
    Number(NumberToken),
    /// One or more whitespace characters collapsed into a single separator.
    Space,
    /// A word break that prints no space: the parts either side are looked up
    /// separately but spoken as one word.  A hyphen between two letters makes
    /// one — upstream reads "well-known" as `w'Eln'oUn`, not `w'El n'oUn`.
    WordJoin,
    /// Sentence/clause boundary punctuation: `.`, `,`, `!`, `?`, `;`, `:`.
    ClauseBoundary(char),
    /// Any other punctuation character.
    Punctuation(char),
    /// An arithmetic *operator* the tokenizer synthesised from the input (the
    /// unary minus of `-5`).  It reads as the language's word for the operation
    /// rather than as the dictionary's name for the character, which is often
    /// the name of the sign itself ("minus sign").
    Operator(char),
    /// Inline phonemes given directly in `[[ … ]]` (espeak phoneme mnemonics).
    /// These bypass the dictionary and are emitted verbatim.
    InlinePhonemes(String),
    /// An inline embedded command (`\x01[±]<value><letter>`) — a rate, pitch or
    /// amplitude change that takes effect **from this point on**, not for the
    /// whole utterance.
    Embedded(EmbeddedCmd),
}

/// Parsed number token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberToken {
    Cardinal(String),
    /// Four digits written with a thousands separator (`1,234`, `1 234`) whose
    /// last three digits are not all zero.  C splits that at the separator into
    /// a thousands word and a word for the rest, each stressed on its own.
    Grouped(String),
    /// `grouped`: the integer part is four digits written with a thousands
    /// separator whose last three are not all zero (`1,234.56`).
    Decimal { integer: String, fractional: String, grouped: bool },
    /// A decimal separator that opens the word (`.5`): not a number to C, whose
    /// rules speak the separator and then every digit on its own.
    Bare { separator: char, digits: String },
    Ordinal(OrdinalNumber),
}

impl NumberToken {
    /// The number for `digits` written with a thousands separator.
    fn separated(digits: String) -> Self {
        let exact = digits.get(1..).is_some_and(|tail| tail.bytes().all(|b| b == b'0'));
        if digits.len() == MAX_UNSPLIT_DIGITS && !exact {
            NumberToken::Grouped(digits)
        } else {
            NumberToken::Cardinal(digits)
        }
    }

    fn parse(word: &str, grammar: &NumberGrammar) -> Option<Self> {
        if word.is_empty() {
            return None;
        }

        if let Some((integer, fractional)) = word.split_once('.') {
            let has_single_dot = word.bytes().filter(|&b| b == b'.').count() == 1;
            if has_single_dot
                && !integer.is_empty()
                && !fractional.is_empty()
                && integer.bytes().all(|b| b.is_ascii_digit())
                && fractional.bytes().all(|b| b.is_ascii_digit())
            {
                return Some(NumberToken::Decimal {
                    integer: integer.to_string(),
                    fractional: fractional.to_string(),
                    grouped: false,
                });
            }
        }

        let digit_end = word.bytes().position(|b| !b.is_ascii_digit()).unwrap_or(word.len());
        if digit_end == 0 {
            return None;
        }

        if digit_end == word.len() {
            return word
                .bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| NumberToken::Cardinal(word.to_string()));
        }

        let digits = &word[..digit_end];
        let suffix = &word[digit_end..];
        if suffix == "." && grammar.ordinals.dot_marks_ordinal {
            return Some(NumberToken::Ordinal(OrdinalNumber {
                digits: digits.to_string(),
                marker: OrdinalMarker::Dot,
            }));
        }

        Some(NumberToken::Ordinal(OrdinalNumber {
            digits: digits.to_string(),
            marker: OrdinalMarker::Suffix(suffix.to_lowercase()),
        }))
    }

    fn surface(&self) -> String {
        match self {
            NumberToken::Cardinal(digits) | NumberToken::Grouped(digits) => digits.clone(),
            NumberToken::Decimal { integer, fractional, .. } => format!("{integer}.{fractional}"),
            NumberToken::Bare { separator, digits } => format!("{separator}{digits}"),
            NumberToken::Ordinal(ordinal) => ordinal.surface(),
        }
    }
}

/// Parsed ordinal number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinalNumber {
    pub digits: String,
    pub marker: OrdinalMarker,
}

impl OrdinalNumber {
    fn surface(&self) -> String {
        match &self.marker {
            OrdinalMarker::Suffix(suffix) => format!("{}{}", self.digits, suffix),
            OrdinalMarker::Dot => format!("{}.", self.digits),
        }
    }
}

/// Marker that makes a numeric token ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinalMarker {
    Suffix(String),
    Dot,
}

/// Tokenize plain text into a sequence of words, spaces and punctuation.
///
/// This is a simplified version of `ReadClause()` from readclause.c.  It
/// handles plain ASCII / UTF-8 text without SSML.
pub fn tokenize(text: &str) -> Vec<Token> {
    tokenize_opts(text, &NumberGrammar::default())
}

/// True if `suffix` (already lower-cased) is a recognised written ordinal marker
/// for the number `digits` — the English `st`/`nd`/`rd`/`th`, the Iberian
/// masculine/feminine `º`/`ª`, or the language's own ordinal indicator (e.g.
/// Dutch `e`).  Anything else after a number (a plural `s`, a unit `km`) is
/// *not* an ordinal.
///
/// Any `st`/`nd`/`rd`/`th` is one, whether or not it fits the number: upstream
/// reads `2st` as "secst" (the `_2o` ordinal plus the `_#st` suffix word).
fn is_ordinal_suffix(suffix: &str, digits: &str, grammar: &NumberGrammar) -> bool {
    let _ = digits;
    if matches!(suffix, "st" | "nd" | "rd" | "th" | "º" | "ª") {
        return true; // a language without a `_#` entry splits it off again later
    }
    grammar.ordinals.indicator.as_deref() == Some(suffix)
        || grammar.ordinals.dict_suffixes.iter().any(|s| s == suffix)
}

/// Whether the dot after a number marks it as an ordinal, given the text `rest`
/// that follows the dot (C `CheckDotOrdinal`): the dot must be followed by a
/// space and a word that does not start with a capital — German nouns are
/// capitalised, so "am 3. Mai" is not an ordinal.  A dot that ends the clause
/// (end of text, or before a comma) is dropped by C before the number is read,
/// so the number stays a cardinal.
fn dot_marks_ordinal_before(
    mut rest: impl Iterator<Item = char> + Clone,
    declined: &[String],
) -> bool {
    match rest.next() {
        Some(first) if first.is_whitespace() => {
            let next = rest.clone().next();
            let lower_start = next.is_some_and(|next| next.is_alphabetic() && !next.is_uppercase());
            let word: String = rest.take_while(|c| c.is_alphabetic()).flat_map(char::to_lowercase).collect();
            lower_start && !declined.contains(&word)
        }
        _ => false,
    }
}

/// Whether the last word before the end of `tokens` is one of `alt_words`.
fn follows_alt_word(tokens: &[Token], alt_words: &[String]) -> bool {
    matches!(tokens, [.., Token::Word(word), Token::Space] if alt_words.contains(&word.to_lowercase()))
}

/// The letters glued to a number anywhere in `text`, lower-cased.
fn glued_number_suffixes(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_ascii_digit() {
            continue;
        }
        while chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
        }
        let mut suffix = String::new();
        let mut tail = chars.clone();
        if tail.peek() == Some(&'.') {
            tail.next();
        }
        if tail.peek() == Some(&'-') && tail.clone().nth(1).is_some_and(char::is_alphabetic) {
            suffix.push('-');
            tail.next();
            chars = tail;
        }
        while let Some(&next) = chars.peek() {
            if !(next.is_alphabetic() || next == 'º' || next == 'ª') {
                break;
            }
            suffix.extend(next.to_lowercase());
            chars.next();
        }
        if !suffix.is_empty() && suffix != "-" && !found.contains(&suffix) {
            found.push(suffix);
        }
    }
    found
}

/// `grammar` extended with the glued suffixes of `text` that the dictionary
/// defines as ordinal markers (`_#<suffix>`), which is how C decides, and with
/// the number symbols (`²`, `½`) it lists as words of its own.
fn with_dict_ordinal_suffixes(grammar: &NumberGrammar, text: &str, dict: &Dictionary) -> NumberGrammar {
    let mut extended = grammar.clone();
    let mut symbols: Vec<char> = text
        .chars()
        .filter(|c| !c.is_ascii() && normalize_number_symbols_except(&c.to_string(), &[]) != c.to_string())
        .collect();
    symbols.sort_unstable();
    symbols.dedup();
    extended.listed_symbols = symbols
        .into_iter()
        .filter(|c| lookup(dict, &c.to_string(), &LookupCtx::default()).is_some_and(|r| r.flags1.found()))
        .collect();
    extended.ordinals.dict_suffixes = glued_number_suffixes(text)
        .into_iter()
        .filter(|suffix| !lookup_num_phonemes(dict, &format!("_#{suffix}")).is_empty())
        .collect();
    if grammar.ordinals.declines_alt_words {
        extended.ordinals.alt_words = alt_flagged_words(text, dict);
        extended.ordinals.alt2_suffixes = extended
            .ordinals
            .dict_suffixes
            .iter()
            .filter(|suffix| {
                lookup(dict, &format!("_#{suffix}"), &LookupCtx { lookup_symbol: true, ..Default::default() })
                    .is_some_and(|r| r.flags1.0 & crate::dictionary::FLAG_ALT2_TRANS != 0)
            })
            .cloned()
            .collect();
    }
    extended
}

/// The words of `text`, lower-cased, whose dictionary entry carries `$alt`.
fn alt_flagged_words(text: &str, dict: &Dictionary) -> Vec<String> {
    let mut words: Vec<String> = text
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    words.sort_unstable();
    words.dedup();
    words
        .into_iter()
        .filter(|word| {
            lookup(dict, word, &LookupCtx::default())
                .is_some_and(|r| r.flags1.0 & crate::dictionary::FLAG_ALT_TRANS != 0)
        })
        .collect()
}

/// Spell a Portuguese cardinal group `0..=999` as text (European Portuguese).
/// Components are joined with "e" (`234` → "duzentos e trinta e quatro"); `100`
/// alone is "cem", within a larger number it's "cento".  Empty for `0`.
fn pt_group(n: u32) -> String {
    const U: [&str; 20] = [
        "zero", "um", "dois", "três", "quatro", "cinco", "seis", "sete", "oito", "nove",
        "dez", "onze", "doze", "treze", "catorze", "quinze",
        "dezasseis", "dezassete", "dezoito", "dezanove",
    ];
    const T: [&str; 10] = [
        "", "", "vinte", "trinta", "quarenta", "cinquenta",
        "sessenta", "setenta", "oitenta", "noventa",
    ];
    const H: [&str; 10] = [
        "", "cento", "duzentos", "trezentos", "quatrocentos", "quinhentos",
        "seiscentos", "setecentos", "oitocentos", "novecentos",
    ];
    let tens_units = |tu: u32| -> String {
        if tu < 20 {
            U[tu as usize].to_string()
        } else {
            let (t, u) = ((tu / 10) as usize, tu % 10);
            if u == 0 {
                T[t].to_string()
            } else {
                format!("{} e {}", T[t], U[u as usize])
            }
        }
    };
    if n == 0 {
        return String::new();
    }
    if n == 100 {
        return "cem".to_string(); // bare 100 is "cem", 101+ is "cento"
    }
    let mut parts: Vec<String> = Vec::new();
    if n / 100 > 0 {
        parts.push(H[(n / 100) as usize].to_string());
    }
    if n % 100 > 0 {
        parts.push(tens_units(n % 100));
    }
    parts.join(" e ")
}

/// Build the Portuguese cardinal word for `digits` in `0..=999_999_999` (the pt
/// dict lacks number keys).  `None` above that range (the caller keeps the
/// existing path).
fn portuguese_cardinal_word(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    if n > 999_999_999 {
        return None;
    }
    if n == 0 {
        return Some("zero".to_string());
    }
    let millions = (n / 1_000_000) as u32;
    let thousands = ((n / 1000) % 1000) as u32;
    let units = (n % 1000) as u32;

    // (group value, rendered text with its scale word) for each non-zero group.
    let mut pieces: Vec<(u32, String)> = Vec::new();
    if millions > 0 {
        pieces.push((
            millions,
            if millions == 1 { "um milhão".to_string() } else { format!("{} milhões", pt_group(millions)) },
        ));
    }
    if thousands > 0 {
        pieces.push((
            thousands,
            if thousands == 1 { "mil".to_string() } else { format!("{} mil", pt_group(thousands)) },
        ));
    }
    if units > 0 {
        pieces.push((units, pt_group(units)));
    }

    // Join with spaces, but "e" before the *final* group when its value is < 100
    // or a round hundred ("dois milhões e quinhentos mil", "mil e vinte"); other
    // group boundaries are bare ("um milhão duzentos e trinta e quatro mil …").
    let last = pieces.len() - 1;
    let mut out = String::new();
    for (i, (value, text)) in pieces.iter().enumerate() {
        if i > 0 {
            let use_e = i == last && (*value < 100 || *value % 100 == 0);
            out.push_str(if use_e { " e " } else { " " });
        }
        out.push_str(text);
    }
    Some(out)
}

/// Build the spoken Portuguese text for a cardinal or decimal number token
/// (`1,5` → "um vírgula cinco"; the fractional part is read digit by digit).
/// `None` for ordinals or values the cardinal speller doesn't cover.
fn portuguese_number_word(token: &NumberToken) -> Option<String> {
    match token {
        NumberToken::Cardinal(digits) | NumberToken::Grouped(digits) => portuguese_cardinal_word(digits),
        NumberToken::Decimal { integer, fractional, .. } => {
            let mut s = portuguese_cardinal_word(integer)?;
            s.push_str(" vírgula");
            // Portuguese reads the fraction as one number when it is short
            // enough (upstream `NUM_DFRACTION_2`): "3,14" is "três vírgula
            // catorze", not "… um quatro".  Leading zeros stay individual.
            let zeros = fractional.bytes().take_while(|&b| b == b'0').count();
            let rest = &fractional[zeros..];
            if !rest.is_empty() && rest.len() <= 2 {
                for _ in 0..zeros {
                    s.push(' ');
                    s.push_str(&portuguese_cardinal_word("0")?);
                }
                s.push(' ');
                s.push_str(&portuguese_cardinal_word(rest)?);
            } else {
                for d in fractional.chars() {
                    s.push(' ');
                    s.push_str(&portuguese_cardinal_word(&d.to_string())?);
                }
            }
            Some(s)
        }
        NumberToken::Ordinal(_) | NumberToken::Bare { .. } => None,
    }
}

/// Replace stand-alone Unicode number symbols that would otherwise be dropped
/// (they are neither letters nor ASCII digits) with a spoken ASCII form:
/// super/subscript digits → the digit, and vulgar fractions → `n/m`.  Spaces are
/// inserted so the substitution doesn't merge with an adjacent number
/// (`10³` → `10 3`, not `103`).  Returns the input unchanged (borrowed) when it
/// contains none of these characters.  Language-neutral — the resulting digits
/// and `/` are read by the normal number/symbol path in the target language.
/// Map the typographic apostrophes onto the plain one — C's
/// `TranslateClause` character loop ("'microsoft' quote or sexed closing single
/// quote, or prime - possibly used as apostrophe").  Without it `don’t` split
/// into "don" and a spoken "T", and Irish `D’fhuascail` into two words.
///
/// Applied to the **clause's own text** only, exactly as C does: a dictionary
/// `$textmode` replacement is re-translated without passing through that loop,
/// so Italian's `€ simbolo dell’euro` keeps its curly apostrophe and reads as
/// two words (`s'imbolo d'El 'EURo`), not one.
fn normalize_apostrophes(text: &str) -> std::borrow::Cow<'_, str> {
    const SMART: [char; 4] = ['\u{2019}', '\u{00B4}', '\u{2032}', '\u{0092}'];
    if !text.chars().any(|c| SMART.contains(&c)) {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(
        text.chars().map(|c| if SMART.contains(&c) { '\'' } else { c }).collect(),
    )
}

/// Dutch and Afrikaans write the article `een` as `'n` and `het` as `'t`.  C's
/// `TranslateChar` turns the apostrophe of a standalone `'n`/`'t` into a schwa,
/// so the dictionary's `ən`/`ət` entries read it; Afrikaans `'n` drops the `n`.
fn schwa_for_article_apostrophe<'a>(text: &'a str, lang: &str) -> std::borrow::Cow<'a, str> {
    let afrikaans = match primary_bcp47_subtag(lang) {
        "nl" => false,
        "af" => true,
        _ => return std::borrow::Cow::Borrowed(text),
    };
    let chars: Vec<char> = text.chars().collect();
    let standalone = |i: usize| {
        let next = chars.get(i + 1).copied();
        let after = chars.get(i + 2).copied();
        chars[i] == '\''
            && !(i > 0 && chars[i - 1].is_alphabetic())
            && matches!(next, Some('n' | 't'))
            && after.is_none_or(|a| a.is_whitespace() || matches!(a, '.' | ',' | '?' | '!' | ';' | ':' | '…'))
    };
    if !(0..chars.len()).any(standalone) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 2);
    let mut skip_n = false;
    for (i, &c) in chars.iter().enumerate() {
        if skip_n {
            skip_n = false;
        } else if standalone(i) {
            out.push('\u{0259}');
            skip_n = afrikaans && chars[i + 1] == 'n';
        } else {
            out.push(c);
        }
    }
    std::borrow::Cow::Owned(out)
}

#[cfg(test)]
fn normalize_number_symbols(text: &str) -> std::borrow::Cow<'_, str> {
    normalize_number_symbols_except(text, &[])
}

/// [`normalize_number_symbols`] leaving the characters in `keep` untouched.
fn normalize_number_symbols_except<'a>(text: &'a str, keep: &[char]) -> std::borrow::Cow<'a, str> {
    fn mapping(c: char) -> Option<&'static str> {
        Some(match c {
            // Superscript digits (⁰¹²³⁴⁵⁶⁷⁸⁹).
            '\u{2070}' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' => match c {
                '\u{2070}' => " 0", '⁴' => " 4", '⁵' => " 5", '⁶' => " 6",
                '⁷' => " 7", '⁸' => " 8", _ => " 9",
            },
            // Upstream reads them as the plain digits (`m²` is "m two").
            '¹' => " 1", '²' => " 2", '³' => " 3",
            // Subscript digits (₀₁₂₃₄₅₆₇₈₉ = U+2080..2089).
            '₀' => " 0", '₁' => " 1", '₂' => " 2", '₃' => " 3", '₄' => " 4",
            '₅' => " 5", '₆' => " 6", '₇' => " 7", '₈' => " 8", '₉' => " 9",
            // Arabic/Persian native separators → their Latin equivalents (kept
            // adjacent to the digits so `١٫٥`→`1.5`, `١٬٠٠٠`→`1,000`, `٥٪`→`5%`).
            '\u{066B}' => ".", // ARABIC DECIMAL SEPARATOR ٫
            '\u{066C}' => ",", // ARABIC THOUSANDS SEPARATOR ٬
            '\u{066A}' => "%", // ARABIC PERCENT SIGN ٪

            _ => return None,
        })
    }

    let changed = |c: char| !keep.contains(&c)
        && (mapping(c).is_some()
            || native_digit_to_ascii(c).is_some()
            || fullwidth_to_ascii(c).is_some()
            || stylized_to_ascii(c).is_some()
            || enclosed_number_value(c).is_some()
            || compat::compatibility_expansion(c).is_some());
    if !text.chars().any(changed) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if keep.contains(&c) {
            out.push(c);
        } else if let Some(s) = mapping(c) {
            out.push_str(s);
        } else if let Some(d) = native_digit_to_ascii(c) {
            // Consecutive native digits stay adjacent so they form one number.
            out.push(d);
        } else if let Some(a) = fullwidth_to_ascii(c) {
            out.push(a);
        } else if let Some(a) = stylized_to_ascii(c) {
            // Mathematical/stylized letters & digits stay adjacent (form one word).
            out.push(a);
        } else if let Some(n) = enclosed_number_value(c) {
            // A standalone enclosed/Roman numeral → its value, spaced so it
            // doesn't merge with an adjacent digit.
            out.push(' ');
            out.push_str(&n.to_string());
            out.push(' ');
        } else if let Some(e) = compat::compatibility_expansion(c) {
            // Unicode compatibility (NFKC) form — `㎏` → `kg`, `℃` → `°C`
            // (upstream #2523).  Last in the chain, so the mappings above, which
            // are tuned for speech, win where they apply.
            out.push_str(e);
        } else {
            out.push(c);
        }
    }
    std::borrow::Cow::Owned(out)
}

/// The numeric value of a circled/parenthesised digit (`①`=1, `⑩`=10, `⑴`=1) or a
/// Roman-numeral codepoint (`Ⅳ`=4, `Ⅻ`=12, `Ⅿ`=1000), which are otherwise dropped.
fn enclosed_number_value(c: char) -> Option<u32> {
    let cp = c as u32;
    Some(match cp {
        0x24EA => 0,                    // ⓪
        0x2460..=0x2473 => cp - 0x245F, // ①..⑳ = 1..20
        0x2474..=0x2487 => cp - 0x2473, // ⑴..⒇ = 1..20 (parenthesised)
        0x2160..=0x216B => cp - 0x215F, // Ⅰ..Ⅻ = 1..12
        0x2170..=0x217B => cp - 0x216F, // ⅰ..ⅻ = 1..12 (lowercase)
        0x216C | 0x217C => 50,          // Ⅼ ⅼ
        0x216D | 0x217D => 100,         // Ⅽ ⅽ
        0x216E | 0x217E => 500,         // Ⅾ ⅾ
        0x216F | 0x217F => 1000,        // Ⅿ ⅿ
        _ => return None,
    })
}

/// Map a stylized Latin letter or digit — the Mathematical Alphanumeric Symbols
/// (bold `𝐇`, italic `𝐻`, script `𝓗`, fraktur `𝕳`, double-struck `𝔻`,
/// sans-serif `𝖳`, monospace `𝙲`, bold digits `𝟏`), the circled letters
/// (`Ⓗ`/`ⓗ`), and the letter-like holes (`ℝ`, `ℋ`, `ℎ`) — to plain ASCII.  These
/// appear in styled social-media text and are otherwise dropped.
fn stylized_to_ascii(c: char) -> Option<char> {
    let cp = c as u32;
    // Mathematical alphanumeric Latin letters: 13 styles × 52 letters, laid out
    // as A–Z then a–z; reserved holes never occur, so a plain offset works.
    if (0x1D400..=0x1D6A3).contains(&cp) {
        let idx = (cp - 0x1D400) % 52;
        return Some(if idx < 26 {
            (b'A' + idx as u8) as char
        } else {
            (b'a' + (idx - 26) as u8) as char
        });
    }
    // Mathematical digits (5 styles × 10) → 0–9.
    if (0x1D7CE..=0x1D7FF).contains(&cp) {
        return char::from_digit((cp - 0x1D7CE) % 10, 10);
    }
    // Circled Latin letters (Ⓐ–Ⓩ, ⓐ–ⓩ).
    if (0x24B6..=0x24CF).contains(&cp) {
        return Some((b'A' + (cp - 0x24B6) as u8) as char);
    }
    if (0x24D0..=0x24E9).contains(&cp) {
        return Some((b'a' + (cp - 0x24D0) as u8) as char);
    }
    // Letters that were unified into the Letterlike Symbols block, leaving holes
    // in the math ranges (script/fraktur/double-struck).
    Some(match c {
        'ℎ' => 'h', 'ℬ' => 'B', 'ℰ' => 'E', 'ℱ' => 'F', 'ℋ' => 'H', 'ℐ' => 'I',
        'ℒ' => 'L', 'ℳ' => 'M', 'ℛ' => 'R', 'ℯ' => 'e', 'ℊ' => 'g', 'ℴ' => 'o',
        'ℭ' => 'C', 'ℌ' => 'H', 'ℑ' => 'I', 'ℜ' => 'R', 'ℨ' => 'Z',
        'ℂ' => 'C', 'ℍ' => 'H', 'ℕ' => 'N', 'ℙ' => 'P', 'ℚ' => 'Q', 'ℝ' => 'R', 'ℤ' => 'Z',
        _ => return None,
    })
}

/// Map a full-width (CJK-compatibility) form of an ASCII character to plain ASCII
/// — `５`→`5`, `Ａ`→`A`, `＄`→`$`, `％`→`%`, the ideographic space → `' '`.  These
/// appear in Japanese/Chinese text and are otherwise dropped.  `None` otherwise.
fn fullwidth_to_ascii(c: char) -> Option<char> {
    match c as u32 {
        // FF01 '！' .. FF5E '～' are ASCII 0x21 '!' .. 0x7E '~' + 0xFEE0.
        cp @ 0xFF01..=0xFF5E => char::from_u32(cp - 0xFEE0),
        0x3000 => Some(' '), // IDEOGRAPHIC SPACE
        _ => None,
    }
}

/// Map a non-ASCII decimal digit (Arabic-Indic `٥`, Devanagari `५`, Thai `๕`, …)
/// to its ASCII equivalent so the normal number path reads it in the target
/// language.  `None` for anything that isn't a decimal digit of a known script.
fn native_digit_to_ascii(c: char) -> Option<char> {
    let cp = c as u32;
    let base = match cp {
        0x0660..=0x0669 => 0x0660, // Arabic-Indic
        0x06F0..=0x06F9 => 0x06F0, // Extended Arabic-Indic (Persian/Urdu)
        0x0966..=0x096F => 0x0966, // Devanagari
        0x09E6..=0x09EF => 0x09E6, // Bengali
        0x0A66..=0x0A6F => 0x0A66, // Gurmukhi
        0x0AE6..=0x0AEF => 0x0AE6, // Gujarati
        0x0B66..=0x0B6F => 0x0B66, // Oriya
        0x0BE6..=0x0BEF => 0x0BE6, // Tamil
        0x0C66..=0x0C6F => 0x0C66, // Telugu
        0x0CE6..=0x0CEF => 0x0CE6, // Kannada
        0x0D66..=0x0D6F => 0x0D66, // Malayalam
        0x0E50..=0x0E59 => 0x0E50, // Thai
        0x0ED0..=0x0ED9 => 0x0ED0, // Lao
        0x0F20..=0x0F29 => 0x0F20, // Tibetan
        _ => return None,
    };
    char::from_digit(cp - base, 10)
}

/// Tokenize with language-specific options.
/// CTRL_EMBEDDED marker (0x01) that starts an inline command (`\x01+10P`).
pub const CTRL_EMBEDDED: char = '\u{01}';

/// The command letters accepted after `\x01[±]<value>` — pitch/speed/amplitude/
/// range/reverb/tone/… (C `EmbeddedCommand`'s `"PSARHTIVYMUBF"`), plus `Z`,
/// which carries an SSML pause length in milliseconds (see
/// [`ssml::BREAK_LETTER`]).  C has no equivalent — it packs the pause into the
/// clause terminator — and drops the letter, so the two never disagree about
/// text written for either.
const EMBED_LETTERS: &str = "PSARHTIVYMUBFZ";

/// A parsed inline embedded command `\x01[+|-]<value><letter>` (C
/// `EmbeddedCommand`).  `relative`: 0 = absolute, 1 = `+`, -1 = `-`.  A missing
/// value is `-1` (the caller substitutes the parameter default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedCmd {
    pub letter: char,
    pub value: i32,
    pub relative: i8,
}

/// Parse and strip inline embedded commands (`\x01…`) from `text`, returning the
/// command-free text and the commands in order.  Mirrors C's handling of
/// `CTRL_EMBEDDED`; a stray `\x01` (or one not followed by a valid letter) is
/// dropped so it can never be spoken as garbage.
pub fn parse_embedded_commands(text: &str) -> (String, Vec<EmbeddedCmd>) {
    let mut out = String::with_capacity(text.len());
    let mut cmds = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != CTRL_EMBEDDED {
            out.push(c);
            continue;
        }
        let relative = match chars.peek() {
            Some('+') => { chars.next(); 1 }
            Some('-') => { chars.next(); -1 }
            _ => 0,
        };
        let mut digits = String::new();
        while matches!(chars.peek(), Some(d) if d.is_ascii_digit()) {
            digits.push(chars.next().unwrap());
        }
        match chars.next() {
            Some(l) if EMBED_LETTERS.contains(l.to_ascii_uppercase()) => {
                cmds.push(EmbeddedCmd {
                    letter: l.to_ascii_uppercase(),
                    value: digits.parse().unwrap_or(-1),
                    relative,
                });
            }
            // Not a valid command (or `\x01` at end): the whole sequence —
            // including the trailing char — is consumed and dropped, matching
            // C's `EmbeddedCommand` (which advances past the letter regardless).
            _ => {}
        }
    }
    (out, cmds)
}

/// C's `N_TR_SOURCE`: the source bytes one clause can hold.
const CLAUSE_SOURCE_BYTES: usize = 800;

/// Clause ends at the last byte that fits (C `ReadClause`).
const CLAUSE_HARD_BYTES: usize = CLAUSE_SOURCE_BYTES - 4;
/// Past this size, the next non-alphanumeric byte ends the clause.
const CLAUSE_SOFT_BYTES: usize = CLAUSE_SOURCE_BYTES - 75;

/// Splits `text` at the hard clause limit, flagging each piece after which a clause ends. Updates `used`.
fn clause_pieces(text: &str, used: &mut usize) -> Vec<(String, bool)> {
    let mut pieces: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        current.push(c);
        *used += c.len_utf8();
        if *used >= CLAUSE_HARD_BYTES {
            pieces.push((std::mem::take(&mut current), true));
            *used = 0;
        }
    }
    if !current.is_empty() {
        pieces.push((current, false));
    }
    pieces
}

/// Adds the clause boundaries C's `ReadClause` inserts when its source buffer fills.
fn split_overlong_clauses(tokens: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut used = 0usize;
    let mut after_space = true;
    for token in tokens {
        match token {
            Token::ClauseBoundary(_) => {
                used = 0;
                after_space = true;
                out.push(token);
            }
            Token::Space => {
                if !after_space {
                    used += 1;
                }
                after_space = true;
                out.push(token);
                if used > CLAUSE_SOFT_BYTES {
                    out.push(Token::ClauseBoundary(ssml::SSML_BREAK));
                    used = 0;
                }
            }
            Token::Word(word) => {
                after_space = false;
                for (piece, ends) in clause_pieces(&word, &mut used) {
                    out.push(Token::Word(piece));
                    if ends {
                        out.push(Token::ClauseBoundary(ssml::SSML_BREAK));
                        after_space = true;
                    }
                }
            }
            Token::Number(NumberToken::Cardinal(digits)) => {
                after_space = false;
                for (piece, ends) in clause_pieces(&digits, &mut used) {
                    out.push(Token::Number(NumberToken::Cardinal(piece)));
                    if ends {
                        out.push(Token::ClauseBoundary(ssml::SSML_BREAK));
                        after_space = true;
                    }
                }
            }
            Token::Number(number) => {
                after_space = false;
                used += number.surface().len();
                out.push(Token::Number(number));
            }
            Token::Punctuation(c) | Token::Operator(c) => {
                after_space = false;
                used += c.len_utf8();
                out.push(token);
                if used > CLAUSE_SOFT_BYTES {
                    out.push(Token::ClauseBoundary(ssml::SSML_BREAK));
                    used = 0;
                    after_space = true;
                }
            }
            other => out.push(other),
        }
    }
    out
}

pub fn tokenize_opts(text: &str, grammar: &NumberGrammar) -> Vec<Token> {
    let text = normalize_number_symbols_except(text, &grammar.listed_symbols);
    let text = text.as_ref();
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == CTRL_EMBEDDED {
            // `\x01[±]<value><letter>` — keep it as a token so the change applies
            // from here on rather than to the whole utterance.
            let relative = match chars.peek() {
                Some('+') => { chars.next(); 1 }
                Some('-') => { chars.next(); -1 }
                _ => 0,
            };
            let mut digits = String::new();
            while matches!(chars.peek(), Some(d) if d.is_ascii_digit()) {
                digits.push(chars.next().unwrap());
            }
            match chars.next() {
                Some(l) if EMBED_LETTERS.contains(l.to_ascii_uppercase()) => {
                    tokens.push(Token::Embedded(EmbeddedCmd {
                        letter: l.to_ascii_uppercase(),
                        value: digits.parse().unwrap_or(-1),
                        relative,
                    }));
                }
                // A stray `\x01` is dropped rather than spoken as garbage.
                _ => {}
            }
            continue;
        }
        if c.is_whitespace() {
            // Collapse runs of whitespace into a single Space token.
            while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                chars.next();
            }
            tokens.push(Token::Space);
        } else if c == ssml::SSML_BREAK {
            // SSML clause boundary (<break/>, </p>, </s>) — silent boundary,
            // never printed even in preserve-punctuation mode.
            tokens.push(Token::ClauseBoundary(c));
        } else if c == '[' && chars.peek() == Some(&'[') {
            // Inline phonemes: [[ mnemonics ]] — read verbatim to the closing ]].
            chars.next(); // consume second '['
            let mut content = String::new();
            while let Some(nc) = chars.next() {
                if nc == ']' {
                    if chars.peek() == Some(&']') {
                        chars.next(); // consume the closing ']]'
                        break;
                    }
                    content.push(']');
                } else {
                    content.push(nc);
                }
            }
            tokens.push(Token::InlinePhonemes(content));
        } else if c == grammar.decimal_separator
            && chars.peek().map_or(false, |d| d.is_ascii_digit())
        {
            // A decimal point with no integer part.  Standalone (".5") it reads
            // "zero point five"; but immediately after a number it is a *continued*
            // decimal separator — a dotted "1.2.3" version → "one point two point
            // three" — and takes no "zero".
            let continued = matches!(tokens.last(), Some(Token::Number(_)));
            let mut fractional = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    fractional.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            // C speaks no second separator in a decimal-comma language
            // ("1,234,567"): the digits after it are a cardinal of their own.
            let after_decimal =
                c == ',' && matches!(tokens.last(), Some(Token::Number(NumberToken::Decimal { .. })));
            if after_decimal {
                tokens.push(Token::Space);
                tokens.push(Token::Number(NumberToken::Cardinal(fractional)));
                continue;
            }
            tokens.push(Token::Number(if continued {
                NumberToken::Decimal { integer: String::new(), fractional, grouped: false }
            } else {
                NumberToken::Bare { separator: c, digits: fractional }
            }));
        } else if !matches!(c, '.' | ',' | '!' | '?' | ';' | ':')
            && clause_punctuation_class(c).is_some_and(|(_, optional_space)| {
                // C only ends the clause when the character is followed by
                // space, a bracket or end of text — unless the character's
                // property says a space is optional, which is what gives
                // space-less scripts their sentence breaks (`।`, `。`).
                optional_space
                    || chars.peek().is_none_or(|&n| n.is_whitespace() || is_bracket(n))
            })
        {
            // Non-ASCII clause punctuation: the Devanagari danda, the CJK full
            // stop and comma, the Armenian/Syriac/Ethiopic/Mongolian/Tibetan
            // terminators, the ellipsis, the en/em dash.  Unspaced (`a–b`) they
            // are dropped instead, which is C's behaviour too.
            while chars.peek().map(|ch| ch.is_whitespace()).unwrap_or(false) {
                chars.next();
            }
            tokens.push(Token::ClauseBoundary(c));
        } else if matches!(c, '.' | ',' | '!' | '?' | ';' | ':') {
            // A '.' immediately followed by a letter or digit is not a sentence
            // end — espeak reads it as the "dot"/"point"/"punkt" symbol
            // ("cat.Dog" → "cat dot dog", "5.and" → "five dot and", and in a
            // comma-decimal locale de "3.14" → "drei punkt vierzehn").  A real
            // full stop is always followed by whitespace or end of text.  (An
            // English "3.14" never reaches here — its '.' is the decimal
            // separator, consumed by the number tokenizer above.)
            if c == '.' && matches!(chars.peek(), Some(n) if n.is_alphanumeric()) {
                tokens.push(Token::Punctuation('.'));
                continue;
            }
            // A word that follows a spoken dot keeps its own dot too: C drops a
            // dot only after a word that does not itself follow punctuation.
            if c == '.'
                && matches!(tokens.as_slice(), [.., Token::Punctuation('.'), Token::Word(_)])
                && dot_stays_in_clause(chars.clone())
            {
                tokens.push(Token::Punctuation('.'));
                continue;
            }
            // A ':' immediately followed by a digit is a time or ratio separator,
            // not a clause break — "10:30" (a time; the colon is silent, handled
            // by `apply_time_reading`) or "3:2" (a ratio; the colon reads as the
            // ":" symbol in English, silent elsewhere).  A ':' before whitespace/
            // end ("Note: this") stays a clause boundary below.
            if c == ':' && matches!(chars.peek(), Some(n) if n.is_ascii_digit()) {
                tokens.push(Token::Punctuation(':'));
                continue;
            }
            // Clause/sentence boundary punctuation
            // Absorb trailing whitespace after punctuation
            while chars.peek().map(|ch| ch.is_whitespace()).unwrap_or(false) {
                chars.next();
            }
            tokens.push(Token::ClauseBoundary(c));
        } else if c == '-'
            && minus_precedes_number(&chars)
            && matches!(
                tokens.last(),
                None | Some(Token::Space | Token::WordJoin) | Some(Token::ClauseBoundary(_))
            )
        {
            // A unary minus written with an ASCII hyphen directly before a
            // number ("-5") reads as the language's "minus" word.  Gated to a clause start or just-after-a-space — a
            // hyphen following a word or number ("COVID-19", "5-10") is a
            // separator/range, not a sign, and is left to the punctuation path.
            // Keep the hyphen as written and mark it an *operator*: C reads a
            // leading `-` by translating the character as a word — Finnish
            // `*_rules` has `__) - (  mi:nus`, Greek a `- m'ion` list entry —
            // and reads the *operation*, not the dictionary's name for the sign,
            // which in several languages is "minus sign" (uk "знак мінуса", he
            // "סימן מינוס").  The currency symbol and digits tokenise normally
            // on the following iterations.
            tokens.push(Token::Operator('-'));
        } else if c.is_ascii_digit() {
            let mut digits = String::new();
            digits.push(c);
            let mut has_dot = false;
            let mut separated = false;
            let mut fractional = String::new();
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() {
                    if has_dot {
                        fractional.push(next);
                    } else {
                        digits.push(next);
                    }
                    chars.next();
                } else if next == grammar.decimal_separator && !has_dot {
                    // Peek ahead to see if followed by a digit (a real decimal).
                    let mut lookahead = chars.clone();
                    lookahead.next(); // skip the decimal separator
                    if lookahead.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        has_dot = true;
                        chars.next();
                    } else {
                        break;
                    }
                } else if grammar.group_separator == Some(next) && !has_dot {
                    // Digit-group separator (e.g. `1,000`): only a genuine group —
                    // exactly three digits that are *not* themselves followed by a
                    // digit — is absorbed; otherwise the char is left as punctuation.
                    let mut lookahead = chars.clone();
                    lookahead.next(); // skip the separator
                    let d1 = lookahead.next();
                    let d2 = lookahead.next();
                    let d3 = lookahead.next();
                    let three_digits = [d1, d2, d3]
                        .iter()
                        .all(|c| c.map_or(false, |c| c.is_ascii_digit()));
                    let more_digits = lookahead.peek().map_or(false, |c| c.is_ascii_digit());
                    if three_digits && !more_digits {
                        chars.next(); // consume the separator; the 3 digits accumulate below
                        separated = true;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if has_dot {
                let grouped = separated && digits.len() == MAX_UNSPLIT_DIGITS;
                tokens.push(Token::Number(NumberToken::Decimal {
                    integer: digits,
                    fractional,
                    grouped,
                }));
                continue;
            }

            // Check for an ordinal suffix immediately after the digits ("1st",
            // "2nd", "3º", Dutch "1e").  Only a *recognised* ordinal suffix is
            // absorbed — other trailing letters (a plural "1990s", a unit "5km")
            // are left for the next token so the number is still spoken instead of
            // being swallowed into a bogus ordinal.
            let mut lookahead = chars.clone();
            let mut suffix = String::new();
            let mut consumed = 0usize;
            // A hyphen, after a dot where the language writes ordinals with one,
            // opens a suffix the dictionary lists with the hyphen (`_#-én`).
            let dotted = grammar.ordinals.dot_marks_ordinal && lookahead.peek() == Some(&'.');
            let mut hyphen = lookahead.clone();
            if dotted {
                hyphen.next();
            }
            if hyphen.peek() == Some(&'-') && hyphen.clone().nth(1).is_some_and(char::is_alphabetic) {
                consumed = usize::from(dotted) + 1;
                suffix.push('-');
                hyphen.next();
                lookahead = hyphen;
            }
            while let Some(&next) = lookahead.peek() {
                if next.is_alphabetic() || next == 'º' || next == 'ª' {
                    suffix.push(next);
                    lookahead.next();
                } else {
                    break;
                }
            }
            let suffix_lc = suffix.to_lowercase();
            let gated = grammar.ordinals.alt2_suffixes.contains(&suffix_lc)
                && !follows_alt_word(&tokens, &grammar.ordinals.alt_words);
            if !suffix.is_empty() && suffix != "-" && !gated && is_ordinal_suffix(&suffix_lc, &digits, grammar) {
                for _ in 0..consumed + suffix.chars().count() - usize::from(consumed > 0) {
                    chars.next(); // now consume the suffix from the real iterator
                }
                tokens.push(Token::Number(NumberToken::Ordinal(OrdinalNumber {
                    digits,
                    marker: OrdinalMarker::Suffix(suffix_lc),
                })));
                continue;
            }

            // NUM_ORDINAL_DOT: if enabled, a trailing dot after digits marks ordinal
            // (e.g. German "3." → "dritte"). Only when NOT followed by a digit.
            if grammar.ordinals.dot_marks_ordinal && chars.peek() == Some(&'.') {
                let mut lookahead = chars.clone();
                lookahead.next(); // skip '.'
                if dot_marks_ordinal_before(lookahead, &grammar.ordinals.alt_words) {
                    chars.next();
                    tokens.push(Token::Number(NumberToken::Ordinal(OrdinalNumber {
                        digits,
                        marker: OrdinalMarker::Dot,
                    })));
                    continue;
                }
            }

            tokens.push(Token::Number(if separated {
                NumberToken::separated(digits)
            } else {
                NumberToken::Cardinal(digits)
            }));
        } else if is_cjk_ideograph(c) {
            // Each ideograph is its own word.  Upstream *joins* a pair that
            // undergoes tone sandhi (你好 → `ni35X'Au214_|`, no gap and no
            // stress on the first syllable), which `apply_tone_sandhi` does on
            // the code list rather than here.
            tokens.push(Token::Word(c.to_string()));
            while let Some(&next) = chars.peek() {
                if is_cjk_ideograph(next) {
                    tokens.push(Token::Space);
                    tokens.push(Token::Word(next.to_string()));
                    chars.next();
                } else {
                    break;
                }
            }
        } else if c.is_alphabetic() || (c == '\'' && grammar.apostrophe & 1 != 0) {
            // Accumulate a word (letters, apostrophes, hyphens within words).
            let mut word = String::new();
            // Lojban (`caps_mark_stress`): a capital marks the stressed syllable,
            // so the first capital — even a leading one — gets a `ˈ` before it
            // and the letter is kept (lowercased downstream at lookup).
            let mut syllable_marked = false;
            if grammar.caps_mark_stress && c.is_uppercase() {
                word.push('\u{02c8}');
                syllable_marked = true;
            }
            word.push(c);
            // Track the last consumed character for the CamelCase splits (espeak
            // breaks at a lowercase→uppercase boundary, and at the last uppercase
            // of a run when it is followed by lowercase + another letter).
            let mut prev_char = c;
            let mut word_break = false;
            let mut hyphen_join = false;
            while let Some(&next) = chars.peek() {
                if is_cjk_ideograph(next) {
                    // Stop word accumulation at CJK boundary.
                    break;
                } else if grammar.caps_mark_stress && next.is_uppercase() {
                    // Lojban: keep the word whole; the first capital inserts a
                    // primary-stress mark, later capitals are just lowercased.
                    chars.next();
                    if !syllable_marked {
                        word.push('\u{02c8}');
                        syllable_marked = true;
                    }
                    word.push(next);
                    prev_char = next;
                } else if grammar.caps_word_split
                    && prev_char.is_lowercase()
                    && next.is_uppercase()
                {
                    // lowercase→uppercase: CamelCase word boundary.  Leave `next`
                    // for the outer loop to start the following word (`CamelCase`
                    // → "Camel" + "Case", `iPhone` → "i" + "Phone").  A `Space` is
                    // emitted below so the boundary is identical to a literal
                    // space (espeak sets `c = ' '`), spacing every output surface.
                    word_break = true;
                    break;
                } else if grammar.caps_word_split
                    && prev_char.is_uppercase()
                    && next.is_uppercase()
                    && caps_run_ends_here(&chars, next, &word, prev_char, grammar.dutch_ij)
                {
                    // upper→lower: split at the LAST uppercase of a run when it is
                    // followed by a lowercase letter + another letter, so an
                    // acronym glued to a capitalised word separates (`HTMLParser`
                    // → "HTML" + "Parser", `JABberwocky` → "JA" + "Bberwocky").
                    // `next` starts the new word.
                    word_break = true;
                    break;
                } else if grammar.tone_numbers && next.is_ascii_digit() {
                    // Tone digit belongs to the syllable (`ni3`), not to a number.
                    word.push(next);
                    chars.next();
                    prev_char = next;
                } else if next == '\''
                    && !(grammar.apostrophe & 2 != 0
                        || chars.clone().nth(1).is_some_and(|after| after.is_alphanumeric()))
                {
                    // C keeps an apostrophe in the word only *between* two
                    // letters (`don't`, `O'Brien`); elsewhere it becomes a space
                    // — so `hello'` is "hello", not a word the dictionary has
                    // never seen.  Leave it for the outer loop.
                    break;
                } else if next.is_alphabetic() || next == '\'' || is_indic_combining(next) {
                    // Indic combining marks (virama ्, nukta ़) aren't classed as
                    // alphabetic but are part of the word — keeping them lets the
                    // rules see a conjunct (`स्त`) instead of two broken words.
                    word.push(next);
                    chars.next();
                    prev_char = next;
                } else if next == '.'
                    && word.rsplit('.').next().is_some_and(|last| last.chars().count() == 1)
                    && word.chars().last().is_some_and(char::is_alphabetic)
                    && chars.clone().nth(1).is_some_and(|after| after.is_alphabetic())
                    && !chars.clone().nth(2).is_some_and(|after| after.is_alphabetic())
                {
                    // A '.' between single letters (`e.g`, `U.S.A`, `a.u.b`)
                    // keeps the letters in one word, as C does.
                    word.push('.');
                    chars.next();
                    prev_char = '.';
                } else if next == '.' && is_dotted_letters(&word) && dot_stays_in_clause(chars.clone().skip(1)) {
                    word.push('.');
                    chars.next();
                    break;
                } else if next == '-' {
                    // A hyphen between two letters is a word break, not part of
                    // the word (espeak `TranslateClause`: `well-known` → "well
                    // known", `X-ray` → "eks ray").  Consume and drop the hyphen
                    // and emit a `Space` so the two parts read as separate words;
                    // a trailing/standalone hyphen just ends the word.
                    let mut lookahead = chars.clone();
                    lookahead.next(); // skip '-'
                    if lookahead.peek().map(|c| c.is_alphabetic()).unwrap_or(false) {
                        chars.next(); // consume (drop) the hyphen
                        word_break = true;
                        hyphen_join = true;
                        break;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            tokens.push(Token::Word(word));
            if word_break {
                tokens.push(if hyphen_join { Token::WordJoin } else { Token::Space });
            }
        } else if is_emoji_char(c) && !is_clause_punctuation(c) {
            // Emoji read via the dict's emoji-name entries.  A whole sequence
            // (variation selector, skin-tone modifier, ZWJ-joined parts, or a
            // regional-indicator flag pair) stays one word so it matches the
            // multi-codepoint dictionary entry.
            //
            // A few codepoints in the emoji blocks are *clause punctuation* to C
            // (`❓` and `❗` are a question and an exclamation mark, `‼` a double
            // one): those end a clause instead of being read, so they fall
            // through to the punctuation arm below.  `⁉` carries both properties,
            // matches none of C's cases, and so is read by name — as it is here.
            let mut emoji = String::from(c);
            let is_regional = |ch: char| (0x1F1E6..=0x1F1FF).contains(&(ch as u32));
            if is_regional(c) {
                if matches!(chars.peek(), Some(&n) if is_regional(n)) {
                    emoji.push(chars.next().unwrap());
                }
            } else {
                while let Some(&m) = chars.peek() {
                    if is_emoji_tag(m) {
                        // Tag sequence (subdivision flag): keep every tag byte.
                        chars.next();
                        emoji.push(m);
                    } else if is_emoji_modifier(m) {
                        chars.next();
                        // Drop the emoji-presentation variation selector (U+FE0F) —
                        // the dictionary keys the base emoji without it — but keep
                        // the ZWJ and skin-tone, which change the emoji's identity.
                        if m != '\u{FE0F}' {
                            emoji.push(m);
                        }
                        // A ZWJ binds the following emoji into the same sequence.
                        if m == '\u{200D}' {
                            if matches!(chars.peek(), Some(&n) if is_emoji_char(n)) {
                                emoji.push(chars.next().unwrap());
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
            tokens.push(Token::Word(emoji));
        } else {
            tokens.push(Token::Punctuation(c));
        }
    }

    tokens
}

/// Whether an uppercase `next` is the last uppercase of a run that should be
/// split off under espeak's upper→lower CamelCase rule (`HTMLParser` → "HTML"
/// + "Parser").  It splits when `next` is followed by a lowercase letter and
/// then another alphabetic character.  The Dutch word-initial "IJ" digraph is
/// exempt (`dutch_ij`).  `chars` is positioned with `next` still at its front
/// (already `peek`ed), so the clone skips it before reading the two look-ahead
/// characters — mirroring C's `next_in` / `next2_in`.
fn caps_run_ends_here<I>(
    chars: &std::iter::Peekable<I>,
    next: char,
    word: &str,
    prev_char: char,
    dutch_ij: bool,
) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    // Dutch keeps "IJ" at word start whole (prev 'I', this 'J', 2nd letter).
    if dutch_ij && prev_char == 'I' && next == 'J' {
        let letters = word.chars().filter(|c| c.is_alphabetic()).count();
        if letters == 1 {
            return false;
        }
    }
    let mut la = chars.clone();
    la.next(); // skip `next` itself (the cached peek)
    let after = la.next();
    let after2 = la.next();
    after.map(|c| c.is_lowercase()).unwrap_or(false)
        && after2.map(|c| c.is_alphabetic()).unwrap_or(false)
}

/// Hindi word-final schwa deletion (simplified).  The inherent vowel `V` at the
/// end of a Devanagari word — after a consonant, and only when the word has
/// another vowel — is dropped (`भारत` "bharatə" → "bharat", `स्कूल` "skulə" →
/// "skul").  espeak does the full positional deletion in the `V` phoneme program
/// (`ChangePhoneme(NULL)`); this covers the dominant, most audible case.
fn delete_final_schwa(phonemes: &mut Vec<u8>, phdata: &PhonemeData) {
    use crate::phoneme::{PH_PAUSE, PH_VOWEL, PHON_END_WORD};
    let v = phdata.lookup_phoneme("V");
    if v == 0 {
        return;
    }
    let had_null = phonemes.last() == Some(&0);
    if had_null {
        phonemes.pop();
    }
    let is_marker = |c: u8| (1..=8).contains(&c) || c == 26 || c == PHON_END_WORD;
    if phonemes.last() == Some(&v) {
        let body = &phonemes[..phonemes.len() - 1];
        let prev_is_consonant = body
            .iter()
            .rev()
            .find(|&&c| !is_marker(c))
            .and_then(|&c| phdata.get(c))
            .map(|p| p.typ != PH_VOWEL && p.typ != PH_PAUSE)
            .unwrap_or(false);
        // Keep the schwa only when it's the word's sole vowel (a lone consonant
        // `क`); if any earlier vowel remains — including another inherent V — the
        // final one is deletable (`घर` "ghɔrə" → "ghɔr").
        let has_other_vowel = body
            .iter()
            .any(|&c| phdata.get(c).map(|p| p.typ == PH_VOWEL).unwrap_or(false));
        if prev_is_consonant && has_other_vowel {
            phonemes.pop(); // drop the final inherent-vowel V
            if matches!(phonemes.last(), Some(&c) if is_marker(c)) {
                phonemes.pop(); // and the stress marker attached to it
            }
        }
    }
    if had_null {
        phonemes.push(0);
    }
}

/// An Indic combining mark (virama / nukta) that continues a word but isn't
/// classified as alphabetic, so the plain word scanner would split on it.
fn is_indic_combining(c: char) -> bool {
    matches!(c as u32,
        0x093C | 0x094D   // Devanagari nukta, virama
        | 0x09BC | 0x09CD // Bengali
        | 0x0A3C | 0x0A4D // Gurmukhi
        | 0x0ABC | 0x0ACD // Gujarati
        | 0x0B3C | 0x0B4D // Oriya
        | 0x0BCD          // Tamil
        | 0x0C3C | 0x0C4D // Telugu
        | 0x0CBC | 0x0CCD // Kannada
        | 0x0D3B | 0x0D3C | 0x0D4D // Malayalam
        | 0x0DCA          // Sinhala
        // ZWJ / ZWNJ: a joiner between letters is part of the word, not a break.
        // Sinhala writes conjuncts as consonant + virama + ZWJ + consonant
        // (`ක්‍ර` = kra); breaking there read the parts as separate letters
        // (`k ɹˈə` instead of `kɹˈɐ`) — upstream fixed the same bug in 1.53.0.
        | 0x200C | 0x200D
    )
}

/// A base emoji / pictograph codepoint (emoticons, symbols & pictographs,
/// transport, dingbats, misc symbols, stars/arrows, regional-indicator flags).
/// These are read via the dictionary's emoji-name entries.
fn is_emoji_char(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF   // emoji, pictographs, supplemental & extended-A
                            // (includes 1F1E6..1F1FF regional-indicator flags)
        | 0x2600..=0x27BF   // miscellaneous symbols + dingbats
        | 0x2B00..=0x2BFF   // stars, arrows
    )
}

/// A codepoint that modifies/joins an emoji rather than starting a new one:
/// the emoji variation selector, skin-tone modifiers, and the ZWJ.
fn is_emoji_modifier(c: char) -> bool {
    matches!(c as u32, 0xFE0F | 0x1F3FB..=0x1F3FF | 0x200D)
}

/// A skin-tone modifier (Fitzpatrick type 1-2 … 6).
fn is_skin_tone(c: char) -> bool {
    matches!(c as u32, 0x1F3FB..=0x1F3FF)
}

/// A TAG character (U+E0020–U+E007F).  Subdivision flags are a black-flag base
/// followed by tag letters and a cancel tag — 🏴 + `gbeng` + CANCEL is England.
/// The dictionary keys the whole sequence, so the tags must stay in the token
/// (upstream 1.53.0 fixed these being read as "black flag").
fn is_emoji_tag(c: char) -> bool {
    matches!(c as u32, 0xE0020..=0xE007F)
}

// ---------------------------------------------------------------------------
// Letter-bits table for English
// ---------------------------------------------------------------------------

/// English letter-bits table (256 bytes, indexed by ASCII/Latin-1 byte).
///
/// Bit layout:
///   bit 0 = vowel (A/E/I/O/U and their variants)
///   bit 2 = consonant
///   bit 7 = vowel2 (stressable vowel)
///
/// This is a simplified version.  The C code builds this from the language
/// definition files (`tr_languages.c`).  We hard-code the basic Latin letters.
/// Build the English letter_bits table matching InitTranslator() in tr_languages.c
/// for Latin-script English (letter_bits_offset = 0).
///
/// Groups (bit positions):
///   0 = A  vowels (aeiou)
///   1 = B  hard consonants, excluding h,r,w  (bcdfgjklmnpqstvxz)
///   2 = C  all consonants                    (bcdfghjklmnpqrstvwxz)
///   3 = H  'soft' consonants                 (hlmnr)
///   4 = F  voiceless consonants              (cfhkpqstx)
///   5 = G  voiced                            (bdgjlmnrvwyz)
///   6 = Y  front vowels                      (eiy)
///   7 = vowels including y                   (aeiouy)
pub fn english_letter_bits() -> [u8; 256] {
    let mut bits = [0u8; 256];

    let set = |bits: &mut [u8; 256], group: u8, letters: &[u8]| {
        for &c in letters {
            bits[c as usize] |= 1 << group;
            // Also uppercase
            if c.is_ascii_lowercase() {
                bits[(c - 32) as usize] |= 1 << group;
            }
        }
    };

    set(&mut bits, 0, b"aeiou");
    set(&mut bits, 1, b"bcdfgjklmnpqstvxz");
    set(&mut bits, 2, b"bcdfghjklmnpqrstvwxz");
    set(&mut bits, 3, b"hlmnr");
    set(&mut bits, 4, b"cfhkpqstx");
    set(&mut bits, 5, b"bdgjlmnrvwyz");
    set(&mut bits, 6, b"eiy");
    set(&mut bits, 7, b"aeiouy");

    bits
}

// ---------------------------------------------------------------------------
// Phoneme-byte → IPA rendering
// ---------------------------------------------------------------------------

/// Render a sequence of raw phoneme codes into an IPA string.
///
/// `phdata` provides the mnemonic and type for each phoneme code.
/// Stress codes set a "pending stress" that is prepended before the next vowel.
///
/// This mirrors the `GetTranslatedPhonemeString()` rendering in dictionary.c,
/// simplified for direct use from the raw phoneme byte stream (no phoneme list).
pub fn phonemes_to_ipa(
    phoneme_bytes: &[u8],
    phdata: &PhonemeData,
    pending_stress_in: PendingStress,
    word_sep: bool,          // prepend a space before the first vowel?
) -> (String, PendingStress) {
    phonemes_to_ipa_lang(phoneme_bytes, phdata, pending_stress_in, word_sep, true)
}

/// Like [`phonemes_to_ipa`] but with an explicit `use_en_overrides` flag.
///
/// Set `use_en_overrides = false` for non-English languages to skip the
/// English-specific schwa / r rendering.
pub fn phonemes_to_ipa_lang(
    phoneme_bytes: &[u8],
    phdata: &PhonemeData,
    pending_stress_in: PendingStress,
    word_sep: bool,
    use_en_overrides: bool,
) -> (String, PendingStress) {
    phonemes_to_ipa_full(
        phoneme_bytes, phdata, pending_stress_in, word_sep, use_en_overrides,
        LiaisonCtx::default(),
    )
}

/// Full phoneme-to-IPA renderer.
/// Codes that are *not* a spoken sound and so are transparent when a French
/// liaison phoneme looks ahead for the next real phoneme: stress markers
/// (2..=8, tonic 26), pause codes, and the `END_WORD` separator (liaison can
/// cross a word boundary, e.g. "deux enfants").  The `0` terminator is handled
/// by the caller's `take_while`, so it is not listed here.
fn liaison_skip(c: u8) -> bool {
    (2..=8).contains(&c) || c == PHON_STRESS_TONIC || c == PHON_END_WORD || is_pause_code(c)
}

/// Whether the liaison phoneme `liaison` (`z2`, `t2`, `z3` …) is pronounced
/// before `next`: a vowel always, and per `ph_french` also `j` (z2, t2) and
/// `w/` (z2, z3).
fn liaison_surfaces_before(
    liaison: &crate::phoneme::table::PhonemeTab,
    next: &crate::phoneme::table::PhonemeTab,
) -> bool {
    let b = liaison.mnemonic.to_le_bytes();
    next.typ == 2 /* phVOWEL */
        || match (b[0], b[1], next.mnemonic_str().as_str()) {
            (b'z' | b't', b'2', "j") | (b'z', _, "w/") => true,
            _ => false,
        }
}

/// Context a token's neighbours provide for resolving French liaison phonemes at
/// the *end* of the token (see `ph_french`: `z2`/`z3`/`t2`/`t3`).
#[derive(Clone, Copy, Default)]
pub struct LiaisonCtx {
    /// French: resolve liaison phonemes by the following sound.  Off for every
    /// other language ("X2"/"X3" are real consonants there, e.g. Hungarian "öt").
    pub resolve: bool,
    /// The next spoken token in this clause starts with a vowel — a token-final
    /// liaison surfaces (`z`/`t`), as in "deux enfants", "six ans".
    pub next_starts_vowel: bool,
    /// A pause follows (clause boundary or end of text).  Selects the *citation*
    /// form of a `3`-liaison — `z3`→[s] ("six"→"sis"), `t3`→[t] ("huit"→"yit") —
    /// whereas a `2`-liaison (and `z3` before a consonant-initial word) is silent.
    pub next_is_pause: bool,
    /// The language reduces unstressed vowels even in dictionary-given words
    /// (`LOPT_REDUCE & 1`; Italian only).  Carried for the `StressCondition`
    /// gate described on [`LangOptions::reduce_dictionary_vowels`], which is not
    /// applied yet.
    pub reduce_dict_vowels: bool,
    /// This word's phonemes came from the dictionary (or a number lookup), not
    /// from the letter-to-sound rules — Turkish/Bashkir/Tatar suppress vowel
    /// reduction then (`isTranslationGiven`).
    pub translation_given: bool,
    /// First real phoneme of the next spoken word (`0` at a pause / end of text).
    ///
    /// Context-dependent phonemes test their *neighbours*, and some of those
    /// conditions legitimately reach across the word boundary: Portuguese coda
    /// `s#` is `IF nextPh(isVowel) THEN z ELIF nextPh(isVoiced) THEN Z ELSE S`,
    /// so "três vírgula" is `tɾˈeʒ …` but "três" alone is `tɾˈeʃ`.  Without this
    /// the token-final phoneme saw nothing after it and always took the
    /// voiceless branch.  (`nextPhW` conditions still stop at the boundary — the
    /// neighbour is flagged as a word start.)
    pub next_phoneme: u8,
}

/// `liaison`: French liaison resolution (see `LiaisonCtx`).  When enabled, a
/// liaison phoneme (a 2-char consonant mnemonic ending in '2' or '3', e.g. `z2`,
/// `t2`, `z3`) is resolved by the following sound: kept before a vowel, dropped
/// before a consonant.  The scan looks past internal `END_WORD` separators, so it
/// works both across word boundaries ("deux enfants" keeps the /z/, "deux chats"
/// drops it) and *within* a single composed token — the number "200" =
/// "deux·cent" drops the /z/ because "cent" is consonant-initial, "2000" =
/// "deux·mille" likewise.  At the end of the token, `LiaisonCtx.next_starts_vowel`
/// / `next_is_pause` decide: a `3`-liaison surfaces its citation form before a
/// pause ("six"→"sis"), while a `2`-liaison is silent.  Only enabled for French;
/// other languages use `X2`/`X3` for real consonants (Hungarian "öt" = ö + `t2`),
/// which must never be dropped.
/// Resolve a context-dependent ("virtual") phoneme through its own program.
///
/// Phonemes whose mnemonic ends in `#` — `d#`, `t#`, `z#`, … — are defined in
/// each language's phoneme source as a program of conditions plus
/// `ChangePhoneme(x)` instructions, and the definitions differ per language:
///
/// * `ph_english`: `d#` → `IF prevPh(isVoiced) THEN d ELSE t`
/// * `ph_portugal`: `d#` → `virtual` + unconditional `ChangePhoneme(d)`
/// * `ph_pt_brazil`: branches to the affricate before a close front vowel
///
/// Hardcoding English's voicing rule (as this function's predecessor did) gave
/// Portuguese `dente` → `tˈẽŋtʰy` instead of `dˈẽntɨ` once the refreshed pt
/// rules started emitting `d#`.  Running the phoneme's actual program via
/// [`interpret_phoneme`] (upstream `InterpretPhoneme`'s `ChangePhoneme` path)
/// keeps every language's own definition.
///
/// Returns the replacement code, or `None` when the program specifies no change.
fn resolve_virtual_phoneme(
    code: u8,
    idx: usize,
    phoneme_bytes: &[u8],
    prev_phcode: u8,
    phdata: &PhonemeData,
    next_word_phoneme: u8,
    stress_level: u8,
    translation_given: bool,
) -> Option<u8> {
    phoneme_program_effects(
        code, idx, phoneme_bytes, prev_phcode, phdata, next_word_phoneme, stress_level,
        translation_given,
    )
        .change_phoneme_code
        .filter(|&c| c != 0 && c != code)
}

/// Run a phoneme's program with neighbour context and return everything it asks
/// for: the `ChangePhoneme` target *and* the list-editing instructions
/// (`InsertPhoneme`, `AppendPhoneme`, `IfNextVowelAppend`, `ChangeNextPhoneme`).
///
/// C keeps these in `phdata->pd_param[]` and applies them in `MakePhonemeList`;
/// they were no-ops here, which is why English lost its linking `r-`
/// ("sofa area" → `sˈəʊfə ˈeəɹiə` instead of `sˈəʊfəɹ ˈeəɹiə`) and the `;` glide
/// after `aI` ("my apple").
fn phoneme_program_effects(
    code: u8,
    idx: usize,
    phoneme_bytes: &[u8],
    prev_phcode: u8,
    phdata: &PhonemeData,
    next_word_phoneme: u8,
    stress_level: u8,
    translation_given: bool,
) -> crate::synthesize::bytecode::PhonemeExtract {
    let empty = crate::synthesize::bytecode::PhonemeExtract::default();
    let Some(ph) = phdata.get(code) else { return empty };
    let program = ph.program;
    if program == 0 {
        return empty;
    }
    // Next two real phonemes, skipping stress markers, pauses and END_WORD.
    let mut following = phoneme_bytes[idx + 1..]
        .iter()
        .copied()
        .take_while(|&c| c != 0)
        .filter(|&c| !liaison_skip(c));
    // Fall back to the next *word*'s first phoneme when this token has nothing
    // more, flagging it as a word start so `nextPhW` conditions still stop at
    // the boundary while plain `nextPh` ones see through it (as in C, where the
    // phoneme list is continuous and `sourceix` marks the boundary).
    let in_token_next = following.next();
    let crosses_word = in_token_next.is_none();
    let next = in_token_next.unwrap_or(next_word_phoneme);
    let nb = crate::synthesize::bytecode::Neighbours {
        prev: prev_phcode,
        this: code,
        next,
        next2: following.next().unwrap_or(0),
        // Stress conditions in the program must see this phoneme's real stress;
        // leaving it at 0 made every vowel look "diminished" and fired changes
        // that shouldn't (Turkish "yüz" → `jˈøz`, Hindi schwa → `nə`).
        stress: stress_level,
        translation_given,
        next_wordstart: crosses_word,
        next2_wordstart: crosses_word,
        ..Default::default()
    };
    crate::synthesize::bytecode::interpret_phoneme_ctl(
        program,
        &phdata.phonindex,
        &nb,
        |c| phdata.get(c).cloned(),
        true, // PhonemeList stage: first ChangePhoneme wins
    )
}

pub fn phonemes_to_ipa_full(
    phoneme_bytes: &[u8],
    phdata: &PhonemeData,
    pending_stress_in: PendingStress,
    word_sep: bool,
    use_en_overrides: bool,
    liaison: LiaisonCtx,
) -> (String, PendingStress) {
    let mut out = String::new();
    let mut stress = pending_stress_in;
    let mut need_space = word_sep;
    let mut prev_phcode: u8 = 0; // track previous real phoneme for d#/z# logic
    const PH_VOICED_FLAG: u32 = 1 << 4; // phFLAGBIT_VOICED

    for (idx, &code) in phoneme_bytes.iter().enumerate() {
        if code == 0 { break; }

        // ── Stress codes ─────────────────────────────────────────────────
        match code {
            PHON_STRESS_P | PHON_STRESS_P2 | PHON_STRESS_TONIC => {
                stress = PendingStress::Primary;
                continue;
            }
            PHON_STRESS_2 | PHON_STRESS_3 => {
                stress = PendingStress::Secondary;
                continue;
            }
            PHON_STRESS_U | PHON_STRESS_D | PHON_STRESS_PREV => {
                // Clear any pending stress for this word
                stress = PendingStress::None;
                continue;
            }
            _ => {}
        }

        // ── Pause / boundary codes ────────────────────────────────────────
        // Code 17 is a miscellaneous control code in most tables but
        // `phonDEFAULTTONE` in a tone language, where it is a real tone phoneme
        // — Vietnamese's toneless syllables take it, and skipping it as a pause
        // dropped the tone from the IPA (`sˈin` instead of `sˈi1n`).
        let is_tone = is_tone_phoneme(code, phdata);
        if is_pause_code(code) && !is_tone {
            // END_WORD (||, code 15) marks a word boundary within a single token.
            // This is used in number phoneme sequences to create spaces between
            // components (e.g., "forty-two" → "fˈɔːti tˈuː").
            // Other pause codes (9, 10, 11) are silently skipped.
            if code == 15 { // PHON_END_WORD
                need_space = true;
                // Reset stress so next word gets fresh stress
                stress = PendingStress::None;
            }
            continue;
        }

        // ── Real phoneme ─────────────────────────────────────────────────
        // English `--ipa` output matches the C dictionary path: keep phoneme codes
        // from rules without synthesis-stage stress reductions (e.g. /V/ → schwa),
        // or "unseen" renders as ə… instead of ʌ….
        // Other languages (e.g. Spanish, Russian) still need ChangeIf resolution
        // for stressed vowel allophones.
        // Stress-conditioned allophones (`ChangeIfNotStressed`).  C additionally
        // refuses to apply these to dictionary-given words unless the language
        // sets `LOPT_REDUCE & 1` (`synthdata.c` `StressCondition`), but wiring
        // that to this port's notion of "came from the dictionary" made Russian
        // *lose* its reduction while Spanish kept its allophone, so the gate is
        // not applied here — see GAPS.md.
        // NOTE: the *stressed* branch resolves a citation phoneme to its
        // stressed form, which several languages need (Czech `milió-` renders
        // `mˈiliˌoːni`, not `mˈiliʲˌoːni`).  It also over-applies for Russian,
        // where the stressed allophone depends on the neighbours — `--ipa` shows
        // `prʲivʲˈɛt` where `-x` and the synthesizer (which run the
        // context-aware pass) correctly give `prʲivʲˈet`.  Skipping it here
        // regressed the languages above, so it stays; see GAPS §2.
        let code = if use_en_overrides {
            code
        } else {
            let is_primary = stress == PendingStress::Primary;
            phdata.resolve_stressed_phoneme(code, is_primary)
        };

        if let Some(ph) = phdata.get(code) {
            let is_vowel = ph.typ == 2; // phVOWEL
            let is_stress_type = ph.typ == 1; // phSTRESS

            if is_stress_type {
                // Stress-type phoneme in phontab (e.g. code 6 has type=1).
                // Decode as in DecodePhonemes: `std_length <= 4` with no program
                // is a stress MARKER, not an acoustic phoneme.
                if ph.program == 0 {
                    // Stress markers (and the emphasis marks `''`/`'!`, which
                    // have no program either) are not spoken.
                    if ph.std_length <= 4 {
                        match ph.std_length {
                            4 => { stress = PendingStress::Primary; }
                            2 | 3 => { stress = PendingStress::Secondary; }
                            _ => {}
                        }
                    }
                    continue;
                }
                // Everything else of this type is a **tone phoneme** — Mandarin's
                // `55`/`35`/`214`/`51`/`11` are phSTRESS entries carrying their own
                // program (C attaches them to the syllable as `tone_ph` and
                // `WritePhMnemonic`s them after the vowel).  Dropping them made
                // every Chinese syllable toneless (`你` → `nˈi`, upstream
                // `nˈi214`).
                if need_space {
                    out.push(' ');
                    need_space = false;
                }
                let ipa = phdata
                    .phoneme_ipa_string(ph.program)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| crate::translate::ipa_table::phoneme_ipa_lang(
                        code, ph.mnemonic, false, use_en_overrides,
                    ));
                // An empty IPA name would silently drop the tone: Vietnamese's
                // default tone (`phonDEFAULTTONE`) has none, and upstream prints
                // its mnemonic (`s'i1n`).
                let ipa = if ipa.is_empty() { ph.mnemonic_display() } else { ipa };
                out.push_str(&ipa);
                prev_phcode = code;
                continue;
            }

            // Resolve French liaison phonemes by the following sound.  A "liaison
            // phoneme" has a 2-char mnemonic ending in '2' or '3' (e.g. n2, z2, t2,
            // z3) and is a consonant; it surfaces only before a vowel.  We scan
            // forward to the next real phoneme (skipping stress/pause markers and
            // internal END_WORD separators): keep before a vowel, drop before a
            // consonant.  If nothing real follows in this token it is token-final:
            // kept before a vowel-initial next token; a `3`-liaison additionally
            // takes its citation form before a pause (`z3`→[s], `t3`→[t]); a
            // `2`-liaison (and `z3` before a consonant) is silent.
            if liaison.resolve {
                let mnemonic = ph.mnemonic;
                // Little-endian u32 mnemonic bytes [b0,b1,b2,b3].
                let b0 = (mnemonic & 0xff) as u8;
                let b1 = ((mnemonic >> 8) & 0xff) as u8;
                let b2 = ((mnemonic >> 16) & 0xff) as u8;
                // liaison "level": 2 = silent at pause, 3 = citation form at pause.
                let level = ((b1 == b'2' || b1 == b'3') && b2 == 0 && !is_vowel)
                    .then(|| b1 - b'0');
                if let Some(level) = level {
                    let next_is_vowel = phoneme_bytes[idx + 1..]
                        .iter()
                        .take_while(|&&c| c != 0)
                        .find(|&&c| !liaison_skip(c))
                        .and_then(|&c| phdata.get(c))
                        .map(|next| liaison_surfaces_before(ph, next));
                    match next_is_vowel {
                        // A real sound follows in-token: keep before a vowel, else drop.
                        Some(true) => {}
                        Some(false) => continue,
                        // Token-final: the neighbouring token / pause decides.
                        None if liaison.next_starts_vowel => {}
                        None if level == 3 && liaison.next_is_pause => {
                            // Citation form before a pause: z3→[s], t3→[t]
                            // (ph_french `ChangePhoneme`).  These are the only two
                            // `3`-liaisons; the citation is the voiceless base.
                            let citation = match b0 {
                                b'z' => "s",
                                b't' => "t",
                                _ => "",
                            };
                            if !citation.is_empty() {
                                if need_space {
                                    out.push(' ');
                                    need_space = false;
                                }
                                out.push_str(citation);
                                prev_phcode = code;
                            }
                            continue;
                        }
                        None => continue, // `2`-liaison at pause, or `3` before a consonant word
                    }
                }
            }

            // Output space separator between words if needed
            if need_space {
                out.push(' ');
                need_space = false;
            }

            let _word_final = phoneme_bytes[idx+1..].iter()
                .all(|&c| c == 0 || c <= 8 || c == 15);

            // This phoneme's own stress level, captured before the pending
            // stress is consumed below — the phoneme's program tests it.
            let this_stress_level: u8 = if is_vowel {
                match stress {
                    PendingStress::Primary => 4,
                    PendingStress::Secondary => 3,
                    PendingStress::None => 1,
                }
            } else {
                1
            };

            // Emit pending stress before vowels
            if is_vowel {
                match stress {
                    PendingStress::Primary   => { out.push_str(IPA_STRESS_PRIMARY); }
                    PendingStress::Secondary => { out.push_str(IPA_STRESS_SECONDARY); }
                    PendingStress::None      => {}
                }
                stress = PendingStress::None;
            }

            // Special handling for phonemes that change based on previous phoneme voice:
            // d# → 'd' if prev is voiced, else 't'
            // z# → 'z' if prev is voiced, else 's'
            // These are common in English past tense and plural suffixes.
            let b1 = ((ph.mnemonic >> 8) & 0xff) as u8;
            // Any phoneme may resolve to another through its own program, not
            // just the `#`-suffixed "virtual" ones: English `@2` (the schwa used
            // only for "the") is `IF nextPh(isVowel) THEN I2 ELSE @`, which is
            // what makes "the apple" `ðɪ ˈapəl`, and `@5` does the same for "to".
            {
                if let Some(changed) = resolve_virtual_phoneme(
                    code, idx, phoneme_bytes, prev_phcode, phdata, liaison.next_phoneme,
                    this_stress_level, liaison.translation_given,
                ) {
                    // `ChangePhoneme(NULL)` (code 1) deletes the phoneme —
                    // C: "NULL phoneme, discard".  Russian's palatalisation
                    // marker uses it, and printing its mnemonic put a stray `_`
                    // inside words ("пять" → `pʲˈɑ_tʲ`).
                    if changed == 1 {
                        prev_phcode = code;
                        continue;
                    }
                    if let Some(changed_ph) = phdata.get(changed) {
                        let ipa = phdata
                            .phoneme_ipa_string(changed_ph.program)
                            .unwrap_or_else(|| crate::translate::ipa_table::phoneme_ipa_lang(
                                changed,
                                changed_ph.mnemonic,
                                changed_ph.typ == 2, /* phVOWEL */
                                use_en_overrides,
                            ));
                        if !ipa.is_empty() {
                            out.push_str(&ipa);
                            prev_phcode = changed;
                            continue;
                        }
                    }
                }

                // The English voicing fallback below only applies to the `#`
                // class (`d#`, `z#`); anything else falls through to normal
                // rendering when its program specified no change.
                let b0 = if b1 == b'#' { (ph.mnemonic & 0xff) as u8 } else { 0 };
                // Check if previous real phoneme is voiced
                let prev_voiced = if let Some(prev_ph) = phdata.get(prev_phcode) {
                    prev_ph.typ == 2 /* phVOWEL */ ||
                    prev_ph.typ == 3 /* phLIQUID */ ||
                    (prev_ph.phflags & PH_VOICED_FLAG) != 0
                } else { false };

                let ipa_char = if b0 == b'd' {
                    if prev_voiced { "d" } else { "t" }
                } else if b0 == b'z' {
                    if prev_voiced { "z" } else { "s" }
                } else {
                    // Other X# phonemes: fall through to normal rendering
                    ""
                };

                if !ipa_char.is_empty() {
                    out.push_str(ipa_char);
                    prev_phcode = code;
                    continue;
                }
            }

            // Look up IPA, most authoritative first:
            //   1. the ACTIVE phoneme table's own `i_IPA_NAME` bytecode,
            //   2. the curated English overrides (accent-neutral ones always,
            //      en-GB's vowel qualities only when the `en` table is active),
            //   3. the ipa1 mnemonic fallback.
            //
            // Order matters: the overrides were derived from `espeak-ng -v en`,
            // so letting them outrank phondata made every English voice speak
            // en-GB — `en-us` has its own table whose phondata says `3` is `ɚ`,
            // not `ə`. For the `en` table this order is a no-op: every override
            // code's phondata is either identical to the override or absent.
            let ipa = if let Some(ipa_str) = phdata.phoneme_ipa_string(ph.program) {
                ipa_str
            } else if let Some(ipa) = use_en_overrides
                .then(|| en_ipa_override(code, phdata.active_table_name()))
                .flatten()
            {
                ipa.to_string()
            } else {
                phoneme_ipa_lang(code, ph.mnemonic, is_vowel, false)
            };
            // (English linking `r` used to be approximated here by inspecting the
            // mnemonic for a `3`; the phoneme's own `IfNextVowelAppend(r-)`
            // program now does it properly, below.)
            out.push_str(&ipa);

            // List-editing instructions from the phoneme's own program: the
            // English linking `r-` (`IfNextVowelAppend`) and the `;` glide after
            // `aI` (`AppendPhoneme`).  C applies these in `MakePhonemeList`.
            let effects = phoneme_program_effects(
                code, idx, phoneme_bytes, prev_phcode, phdata, liaison.next_phoneme,
                this_stress_level, liaison.translation_given,
            );
            let mut appended: Option<u8> = effects.append_phoneme;
            if appended.is_none() {
                if let Some(c) = effects.append_if_next_vowel {
                    let next_real = phoneme_bytes[idx + 1..]
                        .iter()
                        .copied()
                        .take_while(|&b| b != 0)
                        .find(|&b| !liaison_skip(b))
                        .unwrap_or(liaison.next_phoneme);
                    if matches!(phdata.get(next_real), Some(n) if n.typ == 2 /* phVOWEL */) {
                        appended = Some(c);
                    }
                }
            }
            if let Some(c) = appended {
                // Only render an appended phoneme that has an IPA name of its
                // own: the linking `r-` is `ɹ`, while the `;` glide after `aI`
                // is a synthesis-only phoneme that upstream doesn't show in IPA.
                if let Some(extra) = phdata.get(c).and_then(|aph| phdata.phoneme_ipa_string(aph.program)) {
                    out.push_str(&extra);
                }
            }
            prev_phcode = code;
        }
        // Unknown code → skip silently
    }

    (out, stress)
}

// ---------------------------------------------------------------------------
// Word-level translation
// ---------------------------------------------------------------------------

/// Result of translating a single word.
pub struct WordResult {
    /// Phoneme codes with stress markers.
    pub phonemes: Vec<u8>,
    /// Raw dictionary flags (0 if not found in dictionary).
    pub dict_flags: u32,
    /// The pronunciation came from the dictionary *list* rather than from the
    /// letter-to-sound rules — C's `FLAG_FOUND`, which becomes `SFLAG_DICTIONARY`
    /// on every phoneme of the word and stops the stress-conditioned phoneme
    /// changes (`StressCondition`).  A word can carry dictionary *flags* without
    /// this (a flags-only `$` entry still leaves the rules to pronounce it), so
    /// it is tracked separately from `dict_flags`.
    pub found_in_list: bool,
    /// Set when the word came back unstressed (`defer_stress`) with standard
    /// prefixes peeled off its front, so an outer prefix can treat them as part
    /// of its own prefix run (C accumulates every peeled prefix in one
    /// `prefix_phonemes` before stressing the innermost stem).
    pub prefix: Option<PeeledPrefix>,
    /// Following words a multi-word list entry (`(de een)`) covers (C's
    /// `flag - 80`); the caller leaves them out.
    pub skip_words: usize,
    /// Phonemes of an ending flagged `SUFX_T`, held back from a deferred-stress
    /// result so the caller adds them after the stress is placed.
    pub late_suffix: Vec<u8>,
}

/// The peeled-prefix run at the front of a deferred [`WordResult`].
#[derive(Clone, Copy, Debug)]
pub struct PeeledPrefix {
    /// Bytes of `phonemes` that belong to the prefixes; the stem follows.
    pub len: usize,
    /// C's `prefix_flags`: a dictionary entry already fixed the stress.
    pub flagged: bool,
}

fn append_raw_phonemes(dst: &mut Vec<u8>, src: &[u8]) {
    for &b in src {
        if b == 0 {
            break;
        }
        dst.push(b);
    }
}

fn append_late_suffix(phonemes: &mut Vec<u8>, late_suffix: &[u8]) {
    if late_suffix.is_empty() {
        return;
    }
    let end = phonemes.iter().position(|&b| b == 0).unwrap_or(phonemes.len());
    phonemes.truncate(end);
    phonemes.extend_from_slice(late_suffix);
    phonemes.push(0);
}

fn combine_rules_result(result: &crate::dictionary::rules::RulesResult) -> Vec<u8> {
    let mut combined = Vec::new();
    append_raw_phonemes(&mut combined, &result.phonemes);
    append_raw_phonemes(&mut combined, &result.end_phonemes);
    combined
}

fn english_suffix_needs_e(stem: &str, dict: &Dictionary) -> bool {
    const ADD_E_EXCEPTIONS: &[&str] = &["ion"];
    const ADD_E_ADDITIONS: &[&str] = &["c", "rs", "ir", "ur", "ath", "ns", "u", "spong", "rang", "larg"];

    let chars: Vec<char> = stem.chars().collect();
    if chars.len() < 2 {
        return false;
    }

    let penultimate = chars[chars.len() - 2] as u32;
    let last = chars[chars.len() - 1] as u32;
    if is_letter_wc(&dict.letter_bits, penultimate, dict.letter_bits_offset, LETTERGP_VOWEL2)
        && is_letter_wc(&dict.letter_bits, last, dict.letter_bits_offset, LETTERGP_B)
    {
        return !ADD_E_EXCEPTIONS.iter().any(|suffix| stem.ends_with(suffix));
    }

    ADD_E_ADDITIONS.iter().any(|suffix| stem.ends_with(suffix))
}

/// C's `nl` branch of `RemoveEnding`: does taking a `SUFX_E` suffix off this
/// stem restore a doubled vowel?  True when the stem ends in an ASCII vowel plus
/// a consonant and the letter before the vowel is not a vowel — `grad` (from
/// `graden`) doubles to `graad`, while `grond` (from `gronden`) does not.
fn dutch_suffix_doubles_vowel(stem: &str, dict: &Dictionary) -> bool {
    let chars: Vec<char> = stem.chars().collect();
    if chars.len() < 2 {
        return false;
    }
    let (last, vowel) = (chars[chars.len() - 1], chars[chars.len() - 2]);
    // Before a two-letter stem C reads the space that precedes the word.
    let before = chars.len().checked_sub(3).map(|i| chars[i]);
    if !last.is_ascii() || !vowel.is_ascii() {
        return false;
    }
    let is_vowel = |c: char| {
        is_letter_wc(&dict.letter_bits, c as u32, dict.letter_bits_offset, LETTERGP_VOWEL2)
    };
    is_vowel(vowel)
        && is_letter_wc(&dict.letter_bits, last as u32, dict.letter_bits_offset,
                        crate::dictionary::LETTERGP_C)
        && !before.is_some_and(is_vowel)
}

/// C's `confirm_prefix`: a word whose rules matched a prefix keeps it only if
/// the prefix is still recognised once the word's own suffix is removed, and
/// then the prefix is the one matched on the shortened word.  Otherwise the
/// match with prefixes ignored (`FLAG_NO_PREFIX`) stands.
#[allow(clippy::too_many_arguments)]
fn confirm_prefix(
    result: crate::dictionary::rules::RulesResult,
    word: &str,
    word_buf: &[u8],
    word_start: usize,
    word_flags: u32,
    dict_flags: u32,
    dict: &Dictionary,
    lang_opts: &LangOptions,
    phdata: &PhonemeData,
) -> crate::dictionary::rules::RulesResult {
    const SUFX_B: u32 = 0x20000;
    if result.end_type & SUFX_P == 0 || result.end_type & SUFX_B != 0 {
        return result;
    }
    let run = |buf: &[u8], start: usize, flags: u32| {
        let (mut vowels, mut stressed) = (0i32, 0i32);
        crate::dictionary::rules::translate_rules_ext(
            dict,
            buf,
            start,
            flags,
            dict_flags,
            &dict.letter_bits,
            lang_opts.dict_condition,
            &mut vowels,
            &mut stressed,
            Some(phdata),
            true,
        )
    };
    let suffix_only = run(word_buf, word_start, word_flags | crate::dictionary::FLAG_NO_PREFIX);
    if suffix_only.end_type == 0 {
        return result;
    }
    let Some((stem, _, _)) = remove_standard_suffix(word, suffix_only.end_type, dict) else {
        return result;
    };
    let mut stem_buf = vec![b' '];
    stem_buf.extend_from_slice(stem.as_bytes());
    stem_buf.extend_from_slice(&[b' ', 0]);
    let without_suffix = run(&stem_buf, 1, word_flags);
    if without_suffix.end_type & SUFX_P == 0 { suffix_only } else { without_suffix }
}

/// C looks the word up with its prefix kept but its suffix removed (`unser`
/// for `unsereren`) before it re-translates the stem alone.  Returns the list
/// entry's phonemes with the suffix's added, and the entry's flags.
#[allow(clippy::too_many_arguments)]
fn prefix_with_stem_listed(
    word: &str,
    stem: &str,
    stem_word_flags: u32,
    rules_flags: u32,
    dict: &Dictionary,
    lang_opts: &LangOptions,
    phdata: &PhonemeData,
    expect: PosExpect,
) -> Option<(Vec<u8>, u32)> {
    let mut buf = vec![b' '];
    buf.extend_from_slice(stem.as_bytes());
    buf.extend_from_slice(&[b' ', 0]);
    let (mut vowels, mut stressed) = (0i32, 0i32);
    let rules = translate_rules_phdata(
        dict, &buf, 1, stem_word_flags, rules_flags, &dict.letter_bits,
        lang_opts.dict_condition, &mut vowels, &mut stressed, Some(phdata),
    );
    if rules.end_type == 0 || rules.end_type & SUFX_P != 0 {
        return None;
    }
    let (whole, end_flags, _) = remove_standard_suffix(word, rules.end_type, dict)?;
    let ctx = LookupCtx {
        lookup_symbol: false,
        end_flags,
        expect_verb: expect.verb,
        expect_noun: expect.noun,
        expect_past: expect.past,
        dict_condition: lang_opts.dict_condition,
        ..Default::default()
    };
    let hit = lookup(dict, &whole, &ctx).filter(|h| !h.phonemes.is_empty())?;
    let mut phonemes = Vec::new();
    append_raw_phonemes(&mut phonemes, &hit.phonemes);
    append_raw_phonemes(&mut phonemes, &rules.end_phonemes);
    phonemes.push(0);
    Some((phonemes, hit.flags1.0))
}

fn remove_standard_prefix(word: &str, end_type: u32) -> Option<(String, u32)> {
    if end_type & SUFX_P == 0 {
        return None;
    }
    let n_chars = (end_type & 0x3f) as usize;
    if n_chars == 0 {
        return None;
    }
    let cut_end = word
        .char_indices()
        .nth(n_chars.saturating_sub(1))
        .map(|(idx, c)| idx + c.len_utf8());
    let Some(start_after) = cut_end else {
        return None;
    };
    if start_after > word.len() {
        return None;
    }
    Some((word[start_after..].to_string(), FLAG_PREFIX_REMOVED))
}

fn remove_standard_suffix(word: &str, end_type: u32, dict: &Dictionary) -> Option<(String, u32, u32)> {
    let suffix_len_chars = (end_type & 0x3f) as usize;
    if suffix_len_chars == 0 {
        return None;
    }

    let mut chars: Vec<char> = word.chars().collect();
    if suffix_len_chars > chars.len() {
        return None;
    }

    let suffix_start = chars.len() - suffix_len_chars;
    let ending: String = chars[suffix_start..].iter().collect();
    chars.truncate(suffix_start);

    if (end_type & SUFX_I) != 0 && chars.last() == Some(&'i') {
        *chars.last_mut().unwrap() = 'y';
    }

    let mut stem: String = chars.iter().collect();
    let mut end_flags = (end_type & 0xfff0) | FLAG_SUFX;

    if (end_type & SUFX_E) != 0 {
        match dict.lang.as_str() {
            "en" if english_suffix_needs_e(&stem, dict) => {
                stem.push('e');
                end_flags |= FLAG_SUFX_E_ADDED;
            }
            // Dutch spells a long vowel doubled in a closed syllable, so taking
            // the suffix off has to put the second vowel back: `graden` − `en`
            // is the stem `graad`, not `grad`, which is why C reads `Qr'a:d@n`
            // and not `Qr'Ad@n`.  C's own test (`RemoveEnding`, the `nl` branch)
            // is: the stem ends ASCII vowel + consonant and the letter before
            // the vowel is not itself a vowel.
            "nl" if dutch_suffix_doubles_vowel(&stem, dict) => {
                let mut chars: Vec<char> = stem.chars().collect();
                let last = chars.len() - 1;
                chars.insert(last, chars[last - 1]);
                stem = chars.into_iter().collect();
            }
            _ => {}
        }
    }

    if ending == "s" || ending == "es" {
        end_flags |= FLAG_SUFX_S;
    }
    if ending.starts_with('\'') {
        end_flags &= !FLAG_SUFX;
    }

    let mut stem_word_flags = 0;
    if (end_flags & FLAG_SUFX) != 0 {
        stem_word_flags |= FLAG_SUFFIX_REMOVED;
    }
    if (end_type & SUFX_A) != 0 {
        stem_word_flags |= FLAG_SUFFIX_VOWEL;
    }

    Some((stem, end_flags, stem_word_flags))
}

// ---------------------------------------------------------------------------
// Number-to-phonemes (English)
// ---------------------------------------------------------------------------

/// Look up a number word from the dictionary (e.g. "_0", "_1", "_0C", "_0M1").
fn lookup_num_phonemes(dict: &Dictionary, key: &str) -> Vec<u8> {
    // The number words are ordinary list entries and can be `?n`-conditional:
    // European `pt` says `k&t'o*zy` for `_14`, Brazilian `pt-BR` `kat'o*zy`.
    let ctx = LookupCtx {
        lookup_symbol: true,
        dict_condition: dict.dict_condition,
        ..Default::default()
    };
    if let Some(r) = lookup(dict, key, &ctx) {
        if !r.phonemes.is_empty() {
            return r.phonemes;
        }
    }
    Vec::new()
}

const PHON_END_WORD: u8 = 15;

/// Whether the `.` at `ix` joins a number to the number or word right after it
/// (`3.14`, `3.x`); a dot before a space never reaches the number in C.
fn point_after_number(tokens: &[Token], ix: usize) -> bool {
    ix > 0
        && matches!(tokens[ix - 1], Token::Number(_))
        && matches!(tokens.get(ix + 1), Some(Token::Number(_) | Token::Word(_)))
}


/// Byte-oriented pronunciation builder that understands END_WORD separators.
#[derive(Debug, Clone, Default)]
struct Pronunciation {
    bytes: Vec<u8>,
    /// Offsets in `bytes` where C's separate words end, each stressed on its own.
    word_ends: Vec<usize>,
    /// Per word: C's number code starts it with an end-of-word marker (its
    /// tens and units, no hundreds, come first), see [`starts_with_word_break`].
    word_breaks: Vec<bool>,
    /// C's `speak_missing_thousands`: after a scale word the language lacks,
    /// the counter makes a later all-zero group name its scale (see
    /// [`missing_scale_word`]).
    missing_thousands: i8,
}

impl Pronunciation {
    fn push_lookup_word(&mut self, src: &[u8]) {
        self.start_word();
        self.bytes.extend_from_slice(trim_lookup(src));
    }

    /// Drop the trailing phoneme (a real phoneme, not an END_WORD/stress
    /// marker) — used for vowel elision in compound numbers, e.g. Italian
    /// "trenta"+"uno" → "trent"+"uno" = "trentuno".
    fn drop_final_phoneme(&mut self) {
        if matches!(self.bytes.last(), Some(&b) if b > 8 && b != PHON_END_WORD) {
            self.bytes.pop();
        }
    }

    fn insert_at(&mut self, at: usize, src: &[u8]) {
        self.bytes.splice(at..at, src.iter().copied());
    }

    fn append_lookup_suffix(&mut self, src: &[u8]) {
        self.bytes.extend_from_slice(trim_lookup(src));
    }

    /// Append C's phoneme string, whose words are separated by END_WORD.
    fn push_c_words(&mut self, raw: &[u8]) {
        for word in raw.split(|&b| b == PHON_END_WORD).filter(|word| !word.is_empty()) {
            self.push_lookup_word(word);
        }
    }

    fn push_pronunciation(&mut self, other: &Pronunciation) {
        let len = other.trimmed_len();
        if len == 0 {
            return;
        }
        self.start_word();
        self.bytes.extend_from_slice(&other.bytes[..len]);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bytes.last().copied() != Some(PHON_END_WORD) {
            self.bytes.push(PHON_END_WORD);
        }
        self.bytes.push(0);
        self.bytes
    }

    fn trimmed_len(&self) -> usize {
        self.bytes
            .iter()
            .rposition(|&b| b != PHON_END_WORD)
            .map_or(0, |idx| idx + 1)
    }

    fn start_word(&mut self) {
        if !self.bytes.is_empty() && self.bytes.last().copied() != Some(PHON_END_WORD) {
            self.bytes.push(PHON_END_WORD);
        }
    }
}

fn trim_lookup(src: &[u8]) -> &[u8] {
    let len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    &src[..len]
}

fn num_key(raw: impl std::fmt::Display) -> String {
    format!("_{raw}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScaleGroup {
    value: u32,
    scale: Option<u8>,
}

/// Longest digit run spoken as a number; espeak-ng breaks anything longer than
/// this into individual digits.
const MAX_GROUPED_DIGITS: usize = 32;

/// Longest digit run C keeps as one word; longer runs (up to `MAX_GROUPED_DIGITS`)
/// become one word per scale group.
const MAX_UNSPLIT_DIGITS: usize = 4;

/// Three-digit groups, most significant first; scale `k` is 10^(3k). `digits` must be ASCII digits.
fn split_scale_groups(digits: &str) -> Vec<ScaleGroup> {
    let count = digits.len().div_ceil(3);
    let groups: Vec<ScaleGroup> = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .enumerate()
        .map(|(index, chunk)| {
            let scale = count - 1 - index;
            ScaleGroup {
                value: chunk.iter().fold(0, |acc, &b| acc * 10 + u32::from(b.wrapping_sub(b'0'))),
                scale: if scale == 0 { None } else { u8::try_from(scale).ok() },
            }
        })
        .collect();
    groups
}

fn append_scale_word(
    dst: &mut Pronunciation,
    group_value: u32,
    scale: u8,
    dict: &Dictionary,
    grammar: &NumberGrammar,
    exact: bool,
    and_first: bool,
) {
    // Slavic scale-word declension: the scale word takes a case by the count
    // before it (`M_Variant`), and — in Russian — the count's trailing "one"/"two"
    // agrees in gender.
    if grammar.thousands_variant != SlavicThousands::None {
        let count = group_value;
        // A complete "{count} {scale}" word (Czech/Polish `_1M1` = "tisíc"/
        // "tysiąc" — the "one" is baked in, no separate multiplier).
        let dedicated = lookup_num_phonemes(dict, &format!("_{count}M{scale}"));
        if !dedicated.is_empty() {
            dst.push_lookup_word(&dedicated);
            return;
        }
        let teen = (11..=19).contains(&(count % 100));
        let d = count % 10;
        // Per-language `M_Variant`: the scale-word key prefix.
        let prefix = match grammar.thousands_variant {
            SlavicThousands::None => unreachable!(),
            SlavicThousands::Ru if !teen && d == 1 => "1MA",
            SlavicThousands::Ru if !teen && (2..=4).contains(&d) => "0MA",
            SlavicThousands::Cs | SlavicThousands::Sk if (2..=4).contains(&count) => "0MA",
            SlavicThousands::Pl if !teen && (2..=4).contains(&d) => "0MA",
            SlavicThousands::Lt if teen || d == 0 => "0MB",
            SlavicThousands::Lt if d == 1 => "0MA",
            SlavicThousands::Hr if !teen && d == 1 => "1M",
            SlavicThousands::Hr if !teen && (2..=4).contains(&d) => "0MA",
            _ => "0M",
        };
        let mut scale_word = lookup_num_phonemes(dict, &format!("_{prefix}{scale}"));
        if scale_word.is_empty() {
            scale_word = lookup_num_phonemes(dict, &format!("_0M{scale}"));
        }
        // Gender of the count's trailing "one"/"two".  "thousand" (scale 1) is
        // feminine in Russian (одна/две, for any count) and Slovak (дve, but only
        // for a bare single-digit count); the higher scales are masculine.
        let feminine = scale == 1
            && match grammar.thousands_variant {
                SlavicThousands::Ru | SlavicThousands::Hr => true,
                SlavicThousands::Sk => count < 10,
                _ => false,
            };
        if scale_word.is_empty() {
            let (word, counted) = missing_scale_word(dict, count, scale, dst);
            if counted {
                dst.push_lookup_word(&word);
                return;
            }
            scale_word = word;
        }
        dst.push_pronunciation(&num3_phonemes_g(dict, count, false, grammar, feminine, false));
        dst.push_lookup_word(&scale_word);
        return;
    }

    // For an *exact* scale group (no hundreds/tens/units below it), some languages
    // use a variant scale word `_0M{n}x`: Tamil/Malayalam "ஆயிரம்"/"ആയിരം" for a
    // round N000, vs "ஆயிரத்தி"/"ആയിരത്തി" (`_0M{n}`) before more digits.  (Also
    // Kannada, Telugu, Sinhala, Persian, Georgian; mirrors espeak's
    // `LookupThousands`.)
    let scale_word = {
        let variant = if exact {
            lookup_num_phonemes(dict, &format!("_0M{scale}x"))
        } else {
            Vec::new()
        };
        if !variant.is_empty() {
            variant
        } else {
            lookup_num_phonemes(dict, &format!("_0M{scale}"))
        }
    };

    let dedicated = lookup_num_phonemes(dict, &format!("_{group_value}M{scale}"));
    if !dedicated.is_empty() {
        dst.push_lookup_word(&dedicated);
        return;
    }

    let mut scale_word = scale_word;
    if scale_word.is_empty() {
        let (word, counted) = missing_scale_word(dict, group_value, scale, dst);
        if counted {
            dst.push_lookup_word(&word);
            return;
        }
        scale_word = word;
    }

    if scale == 1 && group_value == 1 && grammar.thousands.omit_one_prefix {
        dst.push_lookup_word(&scale_word);
        return;
    }

    if and_first {
        let and = lookup_num_phonemes(dict, "_0and");
        if !and.is_empty() {
            dst.push_lookup_word(&and);
        }
    }
    dst.push_pronunciation(&num3_phonemes_before_scale(dict, group_value, grammar));
    dst.push_lookup_word(&scale_word);
}

fn append_cardinal_group(
    dst: &mut Pronunciation,
    group: ScaleGroup,
    dict: &Dictionary,
    grammar: &NumberGrammar,
    exact: bool,
    preceded: bool,
) {
    if group.value == 0 {
        if let Some(scale) = group.scale.filter(|_| dst.missing_thousands == 1) {
            let above = lookup_num_phonemes(dict, &format!("_0M{}", scale + 1));
            if above.is_empty() {
                dst.push_lookup_word(&lookup_num_phonemes(dict, &format!("_0M{scale}")));
            }
        }
        dst.missing_thousands -= 1;
        return;
    }

    dst.start_word();
    let group_start = dst.bytes.len();
    append_group_body(dst, group, dict, grammar, exact, preceded);
    if preceded && grammar.hundreds.conjunction_before_hundreds && group.value >= 100 {
        let and = lookup_num_phonemes(dict, "_0and");
        dst.insert_at(group_start, trim_lookup(&and));
    }
    dst.missing_thousands -= 1;
}

/// C `LookupThousands` for a scale the language has no word for: the next
/// scale down speaks when that exists too ("thousand" repeated, "billion"
/// shown as "thousand million"), and `speak_missing_thousands` makes the
/// all-zero groups after it name their scale.  The flag is true when the entry
/// already names the count (`_1M1`), so the count is not spoken again.
fn missing_scale_word(
    dict: &Dictionary,
    value: u32,
    scale: u8,
    dst: &mut Pronunciation,
) -> (Vec<u8>, bool) {
    let mut word = Vec::new();
    if scale > 3 && lookup_num_phonemes(dict, &format!("_0M{}", scale - 1)).is_empty() {
        word = lookup_num_phonemes(dict, "_0M2");
        dst.missing_thousands = 3;
    }
    if word.is_empty() {
        let counted = lookup_num_phonemes(dict, &format!("_{value}M1"));
        dst.missing_thousands = 2;
        if !counted.is_empty() {
            return (counted, true);
        }
        word = lookup_num_phonemes(dict, "_0M1");
    }
    (word, false)
}

fn append_group_body(
    dst: &mut Pronunciation,
    group: ScaleGroup,
    dict: &Dictionary,
    grammar: &NumberGrammar,
    exact: bool,
    preceded: bool,
) {
    if let Some(scale) = group.scale {
        let and_first =
            preceded && grammar.hundreds.conjunction_before_bare_group && group.value < 100;
        append_scale_word(dst, group.value, scale, dict, grammar, exact, and_first);
    } else {
        // The last group: no scale word follows, so the plain digit form.
        //
        // C's `LookupNum3` takes the conjunction when the group *follows* a
        // higher scale as well as when it has hundreds of its own — its
        // condition is `(value > 100) || (control & 1 && thousandplex == 0)`.
        // So "1001" is "one thousand AND one" / "mil E um", and only a group
        // below a hundred needs it here (one with hundreds already gets it
        // inside `num3_phonemes`).
        let wants_and = group.value < 100
            && (grammar.hundreds.use_conjunction_with_remainder
                || (grammar.hundreds.conjunction_before_simple_remainder
                    && (group.value % 10 == 0
                        || !lookup_num_phonemes(dict, &num_key(group.value)).is_empty())));
        if preceded && wants_and {
            let and = lookup_num_phonemes(dict, "_0and");
            if !and.is_empty() {
                dst.push_lookup_word(&and);
            }
        }
        dst.push_pronunciation(&num3_phonemes(dict, group.value, false, grammar));
    }
}

fn append_ordinal_scale(
    dst: &mut Pronunciation,
    group_value: u32,
    scale: u8,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) {
    let dedicated = lookup_num_phonemes(dict, &format!("_{group_value}M{scale}o"));
    if !dedicated.is_empty() {
        dst.push_lookup_word(&dedicated);
        return;
    }

    let ord_scale = lookup_num_phonemes(dict, &format!("_0M{scale}o"));
    if !ord_scale.is_empty() {
        if !(scale == 1 && group_value == 1 && grammar.thousands.omit_one_prefix) {
            dst.push_pronunciation(&num3_phonemes_before_scale(dict, group_value, grammar));
        }
        dst.push_lookup_word(&ord_scale);
        return;
    }

    // Only a scale with nothing below it is ordinal, so C's `LookupThousands`
    // takes the exact variant (`_0M{n}x`) here as it does for a cardinal.
    append_scale_word(dst, group_value, scale, dict, grammar, true, false);
}

/// Convert a number value (0-999) to phonemes.
/// Mirrors C's LookupNum3 with per-language number flags.
/// Phonemes for a unit digit, using the feminine form (`_1f`/`_2f`) for 1 and 2
/// when `feminine` — Slavic scale-word agreement ("одна", "две").
fn unit_ph(dict: &Dictionary, n: u32, feminine: bool) -> Vec<u8> {
    if feminine && (n == 1 || n == 2) {
        let f = lookup_num_phonemes(dict, &format!("_{n}f"));
        if !f.is_empty() {
            return f;
        }
    }
    lookup_num_phonemes(dict, &num_key(n))
}

/// A digit's *combining* form — C's `_<n>a`, used when a scale word follows
/// ("hundred", "thousand", …).  German's `_1a  _'aIn` is what makes 1000
/// "ein tausend" rather than "eins tausend"; falls back to the plain `_<n>`.
fn unit_before_scale(dict: &Dictionary, n: u32) -> Vec<u8> {
    let a = lookup_num_phonemes(dict, &format!("_{n}a"));
    if a.is_empty() { lookup_num_phonemes(dict, &num_key(n)) } else { a }
}

fn num3_phonemes(
    dict: &Dictionary,
    value: u32,
    suppress_null: bool,
    grammar: &NumberGrammar,
) -> Pronunciation {
    num3_phonemes_g(dict, value, suppress_null, grammar, false, false)
}

/// [`num3_phonemes`] for a group that a *scale word* follows, so a bare digit
/// takes the combining `_<n>a` form — German 1000 is "ein tausend", not "eins
/// tausend".  Only the languages whose list defines the variant are affected.
fn num3_phonemes_before_scale(
    dict: &Dictionary,
    value: u32,
    grammar: &NumberGrammar,
) -> Pronunciation {
    num3_phonemes_g(dict, value, false, grammar, false, true)
}

/// Like [`num3_phonemes`], but with `feminine` gender for a trailing 1/2 unit
/// (Slavic thousands agreement), and with `combining` the unit under twenty
/// takes its `_<n>a` form because a scale word follows.
fn num3_phonemes_g(
    dict: &Dictionary,
    value: u32,
    suppress_null: bool,
    grammar: &NumberGrammar,
    feminine: bool,
    combining: bool,
) -> Pronunciation {
    let hundreds = value / 100;
    let tensunits = value % 100;

    let mut hundreds_part = Pronunciation::default();
    let mut tens_part = Pronunciation::default();
    let mut suppress_null = suppress_null;

    if hundreds > 0 {
        // Malayalam-style: for a bare 1-hundred, skip the dedicated `_1C`/`_1C0`
        // (which bakes in "one") and use the bare hundred word.
        let skip_dedicated = hundreds == 1 && grammar.hundreds.omit_one_hundred_word;
        // A dedicated "N hundred" word.  For an *exact* hundred (no remainder)
        // prefer `_{n}C0`, which some languages use as a distinct standalone form
        // vs. the `_{n}C` combining form spoken before more digits (Tamil "நூறு"
        // alone vs "நூற்று" in 101; also Kannada/Telugu).  A remainder skips the
        // exact form and takes `_{n}C` directly.  (Mirrors espeak's LookupNum3.)
        let exact = if tensunits == 0 && !skip_dedicated {
            lookup_num_phonemes(dict, &format!("_{}C0", hundreds))
        } else {
            Vec::new()
        };
        let compound = if skip_dedicated {
            Vec::new()
        } else {
            lookup_num_phonemes(dict, &format!("_{}C", hundreds))
        };
        if !exact.is_empty() {
            hundreds_part.push_lookup_word(&exact);
        } else if !compound.is_empty() {
            hundreds_part.push_lookup_word(&compound);
        } else {
            if !(hundreds == 1 && (grammar.hundreds.omit_one_prefix || skip_dedicated)) {
                // A hundred word follows, so the digit takes its combining form.
                hundreds_part.push_lookup_word(&unit_before_scale(dict, hundreds));
            }
            // Bare hundred word.  For the Malayalam bare-1-hundred, an exact
            // hundred uses the dedicated `_0C0` form ("നൂറ്"); everyone else keeps
            // `_0C`, so no other language is affected.
            let bare = if skip_dedicated && tensunits == 0 {
                let e = lookup_num_phonemes(dict, "_0C0");
                if e.is_empty() { lookup_num_phonemes(dict, "_0C") } else { e }
            } else {
                lookup_num_phonemes(dict, "_0C")
            };
            hundreds_part.append_lookup_suffix(&bare);
        }
        suppress_null = true;
    }

    if tensunits != 0 || !suppress_null {
        if tensunits < 20 {
            // For a bare 1/2 (the whole number is just the unit) use the feminine
            // form when required — "одна"/"две" before a feminine scale word.
            let whole = if combining && !feminine && tensunits > 0 {
                unit_before_scale(dict, tensunits)
            } else {
                unit_ph(dict, tensunits, feminine)
            };
            // Resolve the word for "ten": a dedicated `_10`, else the `_1X`
            // ten-prefix (Vietnamese "mười" has only `_1X`).
            let ten_word = || {
                let t = lookup_num_phonemes(dict, "_10");
                if t.is_empty() { lookup_num_phonemes(dict, "_1X") } else { t }
            };
            if !whole.is_empty() {
                // A dedicated teen word ("fifteen", "quinze").
                tens_part.push_lookup_word(&whole);
            } else if tensunits == 10 {
                // Exactly ten with no dedicated `_10` word — use the ten-prefix
                // (Vietnamese `_1X` = "mười"); previously this produced nothing.
                let ten = ten_word();
                if !ten.is_empty() {
                    tens_part.push_lookup_word(&ten);
                }
            } else if tensunits > 10 {
                // Compositional teens: languages like Turkish ("on beş") and
                // Turkmen ("on bäş") have "ten" (`_10`/`_1X`) but no teen words —
                // the dict lookup above is empty, so compose ten + unit.  (Without
                // this, `tr`/`tk`/`hy`/`hu` produced nothing for 11–19.)
                let ten = lookup_num_phonemes(dict, "_1X");
                let ten = if ten.is_empty() { lookup_num_phonemes(dict, "_10") } else { ten };
                if !ten.is_empty() {
                    tens_part.push_lookup_word(&ten);
                    // Some languages join the compositional teen with a
                    // conjunction ("kumi NA tano" = 15 in Swahili), the same one
                    // used between tens and units.  Turkish/Turkmen ("on beş")
                    // do not — they are `Standard`, so this stays gated.
                    if grammar.tens == TensGrammar::WithConjunction {
                        tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, "_0and"));
                    }
                    tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, &num_key(tensunits - 10)));
                }
            }
        } else {
            let ph_full = lookup_num_phonemes(dict, &num_key(tensunits));
            if !ph_full.is_empty() {
                tens_part.push_lookup_word(&ph_full);
            } else if grammar.vigesimal_70_90 && (70..80).contains(&tensunits) {
                // French 70–79 = "soixante" (60) + (10–19).
                tens_part.push_lookup_word(&lookup_num_phonemes(dict, "_6X"));
                tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, &num_key(tensunits - 60)));
            } else if grammar.vigesimal_70_90 && (90..100).contains(&tensunits) {
                // French 90–99 = "quatre-vingt" (80) + (10–19).
                tens_part.push_lookup_word(&lookup_num_phonemes(dict, "_8X"));
                tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, &num_key(tensunits - 80)));
            } else {
                let tens = tensunits / 10;
                let units = tensunits % 10;

                match grammar.tens {
                    TensGrammar::UnitsThenConjunction if units != 0 => {
                        // C's `NUM_SWAP_TENS` takes the combining `_<n>a` unit
                        // (German `_1a` = "ein"), else the plain `_<n>`.
                        tens_part.push_lookup_word(&unit_before_scale(dict, units));
                        tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, "_0and"));
                        tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, &format!("_{tens}X")));
                    }
                    TensGrammar::UnitsThenConjunction => {
                        tens_part.push_lookup_word(&lookup_num_phonemes(dict, &format!("_{tens}X")));
                    }
                    TensGrammar::WithConjunction => {
                        tens_part.push_lookup_word(&lookup_num_phonemes(dict, &format!("_{tens}X")));
                        if units != 0 {
                            tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, "_0and"));
                            tens_part.append_lookup_suffix(&lookup_num_phonemes(dict, &num_key(units)));
                        }
                    }
                    TensGrammar::Standard => {
                        tens_part.push_lookup_word(&lookup_num_phonemes(dict, &format!("_{tens}X")));
                        if units != 0 {
                            // Italian: the tens word elides its final vowel before
                            // a vowel-initial unit (uno=1, otto=8) → "trentuno".
                            if grammar.elide_tens_vowel && (units == 1 || units == 8) {
                                tens_part.drop_final_phoneme();
                            }
                            // A trailing 1/2 agrees in gender ("двадцать одна").
                            tens_part.append_lookup_suffix(&unit_ph(dict, units, feminine));
                        }
                    }
                }
            }
        }
    }

    if hundreds > 0 && tensunits > 0 && grammar.hundreds.use_conjunction_with_remainder {
        hundreds_part.append_lookup_suffix(&lookup_num_phonemes(dict, "_0and"));
    } else if hundreds > 0 && tensunits > 0 && grammar.hundreds.conjunction_before_simple_remainder {
        // Only when the remainder is a single word (round ten, or a number with
        // a direct dictionary entry) — a compound remainder already has its own
        // conjunction, so a second one would be wrong ("honderd vier-en-dertig").
        let simple = tensunits % 10 == 0
            || !lookup_num_phonemes(dict, &num_key(tensunits)).is_empty();
        if simple {
            hundreds_part.append_lookup_suffix(&lookup_num_phonemes(dict, "_0and"));
        }
    }

    ordinal::limit_single_stress(grammar.c_numbers, &mut tens_part.bytes);
    let mut result = Pronunciation::default();
    result.push_pronunciation(&hundreds_part);
    result.push_pronunciation(&tens_part);
    result
}

fn number_token_to_phonemes(
    token: &NumberToken,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) -> Option<Vec<u8>> {
    match token {
        NumberToken::Cardinal(digits) | NumberToken::Grouped(digits) => {
            Some(cardinal_pronunciation(digits, dict, grammar)?.finish())
        }
        NumberToken::Decimal { integer, fractional, .. } => {
            // An empty integer is a continued decimal ("1.2.3" → "…point three"):
            // render just the point and the fractional digits, no leading cardinal.
            let mut pronunciation = if integer.is_empty() {
                Pronunciation::default()
            } else {
                cardinal_pronunciation(integer, dict, grammar)?
            };
            push_decimal_part(&mut pronunciation, !integer.is_empty(), fractional, dict, grammar);
            Some(pronunciation.finish())
        }
        NumberToken::Ordinal(_) | NumberToken::Bare { .. } => None,
    }
}

/// The decimal point (unless the number continues one) and the fractional digits.
fn push_decimal_part(
    pronunciation: &mut Pronunciation,
    with_point: bool,
    fractional: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) {
    let decimal_point = lookup_num_phonemes(dict, "_dpt");
    if !decimal_point.is_empty() && with_point {
        pronunciation.push_lookup_word(&decimal_point);
    }
    if grammar.fraction_suffix && push_fraction(pronunciation, fractional, dict, grammar) {
        return;
    }
    let consumed = if grammar.fraction_digits_as_number > 0 || grammar.fraction_whole_number {
        push_fraction_as_number(pronunciation, fractional, dict, grammar)
    } else {
        0
    };
    for digit in fractional.bytes().skip(consumed) {
        pronunciation.push_lookup_word(&lookup_num_phonemes(dict, &num_key(digit - b'0')));
    }
}

/// Read a decimal fraction as a fraction: the digits as one number, followed by
/// the scale word for their count — `ru "0,25"` → "двадцать пять **сотых**".
///
/// Mirrors upstream `NUM_DFRACTION_5` (`numbers.c`), and is entirely data-driven:
/// the suffix comes from the language's own `_0Z<n>` (plural) / `_0Z<n>s`
/// (singular) dictionary entries, so a language without them falls back to
/// reading the digits one by one.  With `fraction_feminine` (Russian) the
/// numerator's final 1/2 take the feminine `_1f`/`_2f` forms and a numerator
/// ending in 1 — but not 11 — takes the singular suffix ("двадцать одна сотая").
///
/// Returns `false` if the language has no suffix entry for this digit count, so
/// the caller can fall back to digit-by-digit.
fn push_fraction(
    pronunciation: &mut Pronunciation,
    fractional: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) -> bool {
    let count = fractional.len();
    if count == 0 || !fractional.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let Ok(value) = fractional.parse::<u64>() else { return false };

    let feminine_units = grammar.fraction_feminine && matches!(value % 10, 1 | 2)
        && !matches!(value % 100, 11 | 12);
    let singular = grammar.fraction_feminine && value % 10 == 1 && value % 100 != 11;

    let suffix = {
        let plural = lookup_num_phonemes(dict, &format!("_0Z{count}"));
        let chosen = if singular {
            let s = lookup_num_phonemes(dict, &format!("_0Z{count}s"));
            if s.is_empty() { plural } else { s }
        } else {
            plural
        };
        if chosen.is_empty() {
            return false;
        }
        chosen
    };

    // Numerator.  With a feminine final unit, speak the tens (if any) from the
    // normal cardinal machinery and append the feminine `_1f`/`_2f`.
    if feminine_units {
        let tens = value - (value % 10);
        if tens > 0 {
            let Some(p) = cardinal_pronunciation(&tens.to_string(), dict, grammar) else {
                return false;
            };
            // `push_pronunciation` drops the trailing word marker, so the
            // feminine unit below joins the tens into one numeral.
            if tens >= 100 {
                pronunciation.append_lookup_suffix(&p.bytes[..p.trimmed_len()]);
            } else {
                pronunciation.push_pronunciation(&p);
            }
        }
        let fem = lookup_num_phonemes(dict, if value % 10 == 1 { "_1f" } else { "_2f" });
        if fem.is_empty() {
            return false;
        }
        // Part of the same numeral, as every other tens+units pair is.
        if tens >= 100 && tens % 100 == 0 {
            pronunciation.start_word();
            pronunciation.append_lookup_suffix(&fem);
        } else if tens > 0 {
            pronunciation.append_lookup_suffix(&fem);
        } else {
            pronunciation.push_lookup_word(&fem);
        }
    } else {
        let Some(p) = cardinal_pronunciation(&value.to_string(), dict, grammar) else {
            return false;
        };
        // C's LookupNum3 opens with a word break only below 100.
        if value >= 100 {
            pronunciation.append_lookup_suffix(&p.bytes[..p.trimmed_len()]);
            if value % 100 == 0 {
                pronunciation.start_word();
            }
        } else {
            pronunciation.push_pronunciation(&p);
        }
    }

    // C `strcat(ph_buf, buf1)`: the scale word joins the numerator *inside* one
    // word, which is what makes `ru "1,2"` one unit — `V#d;'in_ _:'i_:
    // dv;'e_d;Is;'AtYx`, not two words with a gap between them.
    pronunciation.append_lookup_suffix(&suffix);
    true
}

/// Read the fractional digits as a single number (upstream `NUM_DFRACTION_2` /
/// `NUM_DFRACTION_4`): "3,14" is "three point **fourteen**" in Polish, Czech,
/// Finnish, Turkish, Portuguese, Spanish, French and friends — not "one four".
///
/// Leading zeros are still spoken individually ("0,05" → "nula päť"), and a
/// fraction longer than the language's limit (2 or 5 digits) falls back to
/// digit-by-digit, exactly as C does.
///
/// Returns how many leading fractional digits were spoken; the caller reads the
/// rest one by one.
fn push_fraction_as_number(
    pronunciation: &mut Pronunciation,
    fractional: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) -> usize {
    if fractional.is_empty() || !fractional.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    if grammar.fraction_whole_number {
        return push_fraction_whole(pronunciation, fractional, dict, grammar);
    }
    let zeros = fractional.bytes().take_while(|&b| b == b'0').count();
    for _ in 0..zeros {
        pronunciation.append_lookup_suffix(&lookup_num_phonemes(dict, &num_key(0)));
    }
    let rest = &fractional[zeros..];
    if rest.is_empty() || rest.len() > grammar.fraction_digits_as_number as usize {
        return zeros;
    }
    match fraction_number_bytes(rest, dict, grammar) {
        Some(number) => {
            pronunciation.append_lookup_suffix(&number);
            fractional.len()
        }
        None => zeros,
    }
}

/// C `LookupNum3` for a fraction numerator: it opens with an end-of-word marker
/// when there are no hundreds, and nothing separates it from the text before.
fn fraction_number_bytes(digits: &str, dict: &Dictionary, grammar: &NumberGrammar) -> Option<Vec<u8>> {
    let numeric = digits.parse::<u64>().ok()?;
    let number = cardinal_pronunciation(&numeric.to_string(), dict, grammar)?;
    let mut out = Vec::new();
    if numeric < 100 {
        out.push(PHON_END_WORD);
    }
    out.extend_from_slice(&number.bytes[..number.trimmed_len()]);
    Some(out)
}

/// `NUM_DFRACTION_1`: the whole fraction as one number, with the `_0Z<n>` scale
/// word when it starts with a zero.
fn push_fraction_whole(
    pronunciation: &mut Pronunciation,
    fractional: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) -> usize {
    // C reads the fraction with `atoi`, so a longer one overflows into digits.
    if fractional.len() > 9 {
        return 0;
    }
    let scale = if fractional.starts_with('0') {
        trim_lookup(&lookup_num_phonemes(dict, &format!("_0Z{}", fractional.len()))).to_vec()
    } else {
        Vec::new()
    };
    if fractional.starts_with('0') && scale.is_empty() {
        return 0;
    }
    let Some(mut number) = fraction_number_bytes(fractional, dict, grammar) else { return 0 };
    number.extend_from_slice(&scale);
    pronunciation.append_lookup_suffix(&number);
    fractional.len()
}

fn cardinal_pronunciation(
    digits: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) -> Option<Pronunciation> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    // C routes runs over 32 digits to the individual-digit path before any leading-zero handling.
    if digits.len() > MAX_GROUPED_DIGITS {
        let mut result = Pronunciation::default();
        push_individual_digits(&mut result, digits, dict);
        return Some(result);
    }

    // A leading zero marks a code, not a quantity (C `TranslateNumber`): a short
    // one (≤3 digits) speaks its leading zeros then the number ("007" → "zero
    // zero seven"); a longer one is spoken digit-by-digit ("0800" → "zero eight
    // zero zero").  A lone "0" and decimals (a separate token) are unaffected.
    if digits.len() > 1 && digits.starts_with('0') {
        let mut result = Pronunciation::default();
        if digits.len() > 3 {
            // The rules speak each digit from the list and add a no-link pause
            // after every second one, as for a long run (`push_individual_digits`).
            push_individual_digits(&mut result, digits, dict);
            return Some(result);
        }
        // Speak the leading zeros (at most all but the last digit), then read the
        // remaining value with the normal cardinal machinery.
        let n_leading = digits
            .bytes()
            .take(digits.len() - 1)
            .take_while(|&b| b == b'0')
            .count();
        for _ in 0..n_leading {
            result.append_lookup_suffix(&lookup_num_phonemes(dict, "_0"));
        }
        if digits.bytes().all(|b| b == b'0') {
            // "000" → the leading zeros plus a final "zero" (an all-zero run).
            result.push_lookup_word(&lookup_num_phonemes(dict, "_0"));
        } else {
            push_cardinal_groups(&mut result, digits, dict, grammar);
        }
        return Some(result);
    }

    if digits == "0" {
        let mut pronunciation = Pronunciation::default();
        pronunciation.push_lookup_word(&lookup_num_phonemes(dict, "_0"));
        return Some(pronunciation);
    }

    // C `NUM_1900`: 1900–1999 read as "nineteen hundred (and) …" — `hundreds`
    // is 19, which the three-digit path already handles (`_19` + `_0C` + the
    // usual conjunction).  Only the ten languages that set the flag do this;
    // the port used to read *every* round hundred from 1100 up this way, in
    // every language, so `en 1500` was "fifteen hundred" where upstream says
    // "one thousand five hundred", `de 1100` "elfhundert" for "ein tausend ein
    // hundert" — and `en 1984` missed the year reading entirely.
    if grammar.year_1900 && digits.len() == 4 {
        if let Some(year) = digits.parse::<u32>().ok().filter(|y| (1900..=1999).contains(y)) {
            return Some(num3_phonemes(dict, year, false, grammar));
        }
    }

    let mut result = Pronunciation::default();
    push_cardinal_groups(&mut result, digits, dict, grammar);
    Some(result)
}

fn push_cardinal_groups(
    result: &mut Pronunciation,
    digits: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
) {
    push_cardinal_group_prefix(result, digits, dict, grammar, usize::MAX);
}

/// A thousands-separated number as C reads it: one word per group.
fn grouped_pronunciation(digits: &str, dict: &Dictionary, grammar: &NumberGrammar) -> Pronunciation {
    let mut result = Pronunciation::default();
    push_groups(&mut result, digits, dict, grammar, usize::MAX, true);
    result
}

fn push_cardinal_group_prefix(
    result: &mut Pronunciation,
    digits: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
    count: usize,
) -> bool {
    push_groups(result, digits, dict, grammar, count, digits.len() > MAX_UNSPLIT_DIGITS)
}

/// The first `count` groups of `digits` as cardinal words; returns whether any
/// of them was non-zero.  With `split` each group is a word of its own.
fn push_groups(
    result: &mut Pronunciation,
    digits: &str,
    dict: &Dictionary,
    grammar: &NumberGrammar,
    count: usize,
    split: bool,
) -> bool {
    let mut spoken = false;
    for group in split_scale_groups(digits).into_iter().take(count) {
        let exact = scale_group_is_exact(digits, group.scale);
        let before = result.bytes.len();
        append_cardinal_group(result, group, dict, grammar, exact, spoken);
        // C ends each group word that is not exact with PHON_PAUSE_NOLINK, which stops regressive voicing across groups.
        if split && !exact {
            // An all-zero group is still a word of its own: empty, then the pause.
            if result.bytes.len() == before {
                result.start_word();
            }
            result.bytes.push(crate::phoneme::PHON_PAUSE_NOLINK);
        }
        if split {
            result.word_ends.push(result.bytes.len());
            result.word_breaks.push(group.value < 100);
        }
        spoken |= group.value != 0;
    }
    spoken
}

fn push_individual_digits(result: &mut Pronunciation, digits: &str, dict: &Dictionary) {
    let word = individual_digit_word(digits, dict);
    if !word.is_empty() {
        result.push_lookup_word(&word);
    }
}

/// The digits' names run together, a no-link pause after every second one.
fn individual_digit_word(digits: &str, dict: &Dictionary) -> Vec<u8> {
    let words: Vec<Vec<u8>> = (0..10u8)
        .map(|d| lookup_num_phonemes(dict, &num_key(d)))
        .collect();
    let mut word: Vec<u8> = Vec::new();
    let spoken = digits
        .bytes()
        .filter_map(|d| words.get(usize::from(d.wrapping_sub(b'0'))));
    for (ix, digit) in spoken.enumerate() {
        let mut piece = trim_lookup(digit).to_vec();
        if ix % 2 == 1 {
            piece.push(crate::phoneme::PHON_PAUSE_NOLINK);
        }
        // C's `AppendPhonemes` drops a piece that would reach the word buffer
        // size, so a long digit run is cut at a whole digit.
        if word.len() + piece.len() < N_WORD_PHONEMES {
            word.extend_from_slice(&piece);
        }
    }
    word
}

/// Whether C splits the run into digit groups: 5 to 32 digits that exceed
/// max_digits or start with 0. Longer runs take the bounded individual-digit path.
fn reads_digit_by_digit(digits: &str, grammar: &NumberGrammar) -> bool {
    let n = digits.len();
    (5..=MAX_GROUPED_DIGITS).contains(&n)
        && (n > usize::from(grammar.max_digits) || digits.starts_with('0'))
}

fn digit_groups<'a>(digits: &'a str, grammar: &NumberGrammar) -> Vec<&'a str> {
    let n = digits.len();
    let mut starts: Vec<usize> = (1..n).filter(|j| grammar.break_numbers & (1 << (n - j)) != 0).collect();
    starts.insert(0, 0);
    starts.push(n);
    starts.windows(2).map(|w| &digits[w[0]..w[1]]).collect()
}

/// The unstressed phonemes the translation rules give `word`.
fn rules_phonemes(word: &str, dict: &Dictionary, lang_opts: &LangOptions) -> Vec<u8> {
    let mut buf = Vec::with_capacity(word.len() + 3);
    buf.push(b' ');
    buf.extend_from_slice(word.as_bytes());
    buf.extend_from_slice(&[b' ', 0]);
    let (mut vowels, mut stressed) = (0i32, 0i32);
    crate::dictionary::rules::translate_rules(
        dict,
        &buf,
        1,
        0,
        0,
        &dict.letter_bits,
        lang_opts.dict_condition,
        &mut vowels,
        &mut stressed,
    )
    .phonemes
}

/// How C speaks the dot it glues to a digit group that ends in the thousands
/// separator: the list entry for ".", else the rules' reading of it.
fn symbol_phonemes(symbol: &str, dict: &Dictionary, lang_opts: &LangOptions) -> Vec<u8> {
    let listed = lookup_num_phonemes(dict, symbol);
    let spoken = if listed.is_empty() { rules_phonemes(symbol, dict, lang_opts) } else { listed };
    spoken.into_iter().take_while(|&b| b != 0).collect()
}

/// Each group of digits is one word, stressed on its own, with the digit
/// names run together inside it.
fn spelled_digit_groups(
    digits: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    grammar: &NumberGrammar,
    speak_symbol: &dyn Fn(&str) -> Vec<u8>,
) -> Vec<u8> {
    let mut out = Vec::new();
    let groups = digit_groups(digits, grammar);
    // C inserts the thousands separator after every group but the last; the
    // rules then speak a dot glued to the group's digits.
    let point = if grammar.group_separator == Some('.') && groups.len() > 1 {
        speak_symbol(".")
    } else {
        Vec::new()
    };
    for (ix, group) in groups.iter().enumerate() {
        let mut word = Pronunciation::default();
        for (ix, d) in group.bytes().enumerate() {
            let mut piece = trim_lookup(&lookup_num_phonemes(dict, &num_key(d - b'0'))).to_vec();
            // As `push_individual_digits`: every second digit of a group is
            // followed by a no-link pause.
            if ix % 2 == 1 {
                piece.push(crate::phoneme::PHON_PAUSE_NOLINK);
            }
            word.append_lookup_suffix(&piece);
        }
        let mut phonemes = word.finish();
        let pointed = ix + 1 < groups.len() && !point.is_empty();
        if pointed {
            NumberTail { raw: point.clone(), fused: true }.append_to(&mut phonemes);
        }
        set_word_stress(&mut phonemes, phdata, stress_opts, Some(0), -1, 0);
        out.extend(phonemes.iter().copied().take_while(|&b| b != 0));
        if pointed {
            out.push(PHON_END_WORD);
        }
    }
    out.push(0);
    out
}

/// Whether a scale group is *exact* — the whole number has no non-zero digit
/// below this scale (so `1000` is exact at the thousands scale, `1234` is not).
/// A scale-less (units) group is always "exact".  Used to select the `_0M{n}x`
/// variant scale word.
fn scale_group_is_exact(digits: &str, scale: Option<u8>) -> bool {
    match scale {
        Some(s) => {
            let trailing_zeros = digits.len() - digits.trim_end_matches('0').len();
            trailing_zeros >= 3 * usize::from(s)
        }
        None => true,
    }
}

/// The pronunciation of the last one or two digits of `digits` that a
/// multi-word dictionary entry gives together with the word glued to them
/// (fr `(21 ème)`, `(1 er)`), and how many digits it covers.  C
/// `TranslateNumber_1` tries two digits first, then one digit unless it is `0`.
fn lookup_digit_follow(dict: &Dictionary, digits: &str, word: &str) -> Option<(Vec<u8>, usize)> {
    let follow = format!("{} ", word.to_lowercase());
    let ctx = LookupCtx {
        lookup_symbol: true,
        dict_condition: dict.dict_condition,
        ..Default::default()
    };
    let attempt = |n: usize| {
        let key = digits.get(digits.len().checked_sub(n)?..)?;
        lookup_dict2_follow(dict, key.as_bytes(), &ctx, follow.as_bytes())
            .filter(|found| found.skipwords > 0 && !found.phonemes.is_empty())
            .map(|found| (trim_lookup(&found.phonemes).to_vec(), n))
    };
    let single_is_zero = digits.ends_with('0');
    attempt(2).or_else(|| if single_is_zero { None } else { attempt(1) })
}

/// Try to interpret a word as an ordinal number (e.g. "2nd", "1st", "3º").
///
/// Splits the word into a leading digit string and a trailing non-digit suffix,
/// then looks up `_#<suffix>` in the dictionary. If found, the word is an ordinal.
///
/// For the last (units) digit, looks up `_<digit>o` for irregular ordinals
/// (e.g. `_1o` → "first", `_2o` → "second"). Falls back to cardinal + `_ord`
/// suffix for regular ordinals (e.g. "four" + "th").
///
/// This mirrors C espeak-ng's ordinal handling in numbers.c.
fn try_ordinal_number(
    ordinal: &OrdinalNumber,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    grammar: &NumberGrammar,
) -> Option<WordResult> {
    // Only a `_#<suffix>` entry gives the number a suffix word; a dot or the
    // language's own indicator (Dutch `e`) marks the ordinal without one.
    let (suffix_ph, suffix_ph_alt) = match &ordinal.marker {
        OrdinalMarker::Suffix(suffix) => (
            trim_lookup(&lookup_num_phonemes(dict, &format!("_#{suffix}"))).to_vec(),
            trim_lookup(&lookup_num_phonemes(dict, &format!("_x#{suffix}"))).to_vec(),
        ),
        OrdinalMarker::Dot => (Vec::new(), Vec::new()),
    };
    let is_ordinal = match &ordinal.marker {
        OrdinalMarker::Suffix(suffix) => {
            !suffix_ph.is_empty() || grammar.ordinals.indicator.as_deref() == Some(suffix.as_str())
        }
        OrdinalMarker::Dot => grammar.ordinals.dot_marks_ordinal,
    };
    let follow = match &ordinal.marker {
        OrdinalMarker::Suffix(suffix) if !is_ordinal => lookup_digit_follow(dict, &ordinal.digits, suffix),
        _ => None,
    };
    if !is_ordinal && follow.is_none() {
        return None;
    }
    let (digit_lookup, digit_lookup_len) = follow.unwrap_or_default();
    let numbers = ordinal::OrdinalNumbers {
        dict,
        phdata,
        numbers: grammar.c_numbers,
        numbers2: grammar.c_numbers2,
        ph_ordinal2: suffix_ph,
        ph_ordinal2x: suffix_ph_alt,
        ordinal: is_ordinal,
        digit_lookup,
        digit_lookup_len,
    };

    let all_digits = ordinal.digits.as_str();
    if all_digits.is_empty()
        || all_digits.len() > MAX_GROUPED_DIGITS
        || !all_digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let mut pronunciation = Pronunciation::default();
    // A short leading-zero run is spoken as zeros first, as for a cardinal.
    let spoken_zeros = if numbers.digit_lookup.is_empty() || all_digits.len() > 3 {
        0
    } else {
        all_digits.bytes().take(all_digits.len() - 1).take_while(|&b| b == b'0').count()
    };
    for _ in 0..spoken_zeros {
        pronunciation.append_lookup_suffix(&lookup_num_phonemes(dict, "_0"));
    }
    let digits = &all_digits[spoken_zeros..];
    let year = digits.parse::<u32>().ok().filter(|year| {
        grammar.year_1900 && digits.len() == 4 && (1900..=1999).contains(year)
    });
    if let Some(year) = year {
        pronunciation.push_c_words(&numbers.num3(year, false, false));
        let mut phonemes = pronunciation.finish();
        set_word_stress(&mut phonemes, phdata, stress_opts, Some(0), -1, STRESS_UNBOUNDED);
        return Some(WordResult { phonemes, dict_flags: 0, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() });
    }
    let groups = split_scale_groups(digits);
    // C makes every group of a long number a word of its own: the higher ones are
    // cardinals with their scale word, and only the last group takes the ordinal.
    if digits.len() > MAX_UNSPLIT_DIGITS {
        let last = groups[groups.len() - 1];
        let spoken = push_cardinal_group_prefix(&mut pronunciation, digits, dict, grammar, groups.len() - 1);
        pronunciation.push_c_words(&numbers.num3(last.value, spoken, spoken));
        pronunciation.word_ends.push(pronunciation.bytes.len());
        pronunciation.word_breaks.push(last.value < 100);
        let phonemes = stress_each_word(pronunciation, phdata, stress_opts, STRESS_UNBOUNDED);
        return Some(WordResult { phonemes, dict_flags: 0, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() });
    }
    // For a zero value there is no non-zero group; fall back to the *last* group
    // (the units, scale-less) so "0th" renders "zeroth" instead of dropping the
    // number or attaching a spurious scale word.
    let last_nonzero = groups
        .iter()
        .rposition(|group| group.value != 0)
        .unwrap_or(groups.len().saturating_sub(1));

    for &group in &groups[..last_nonzero] {
        // These groups all precede the last non-zero group, so a lower non-zero
        // digit always follows — never an exact scale group.
        append_cardinal_group(&mut pronunciation, group, dict, grammar, false, false);
    }

    let final_group = groups[last_nonzero];
    if final_group.scale.is_some() && !numbers.digit_lookup.is_empty() {
        return None;
    }
    if let Some(scale) = final_group.scale {
        append_ordinal_scale(&mut pronunciation, final_group.value, scale, dict, grammar);
        pronunciation.push_c_words(&numbers.ph_ordinal2);
    } else {
        let previous = groups[..last_nonzero].iter().any(|group| group.value != 0);
        pronunciation.push_c_words(&numbers.num3(final_group.value, previous, previous));
    }

    let mut phonemes = pronunciation.finish();
    if starts_with_word_break(digits) {
        phonemes.insert(0, PHON_END_WORD);
    }
    set_word_stress(&mut phonemes, phdata, stress_opts, Some(0), -1, STRESS_UNBOUNDED);
    Some(WordResult { phonemes, dict_flags: 0, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() })
}

/// Whether C's `LookupNum3` leaves an end-of-word marker in front of the
/// number's first sound: it writes `hundreds + marker + tens/units`, so a
/// leading group below 100 (1, 2, 5 or 6… digits, no leading zero) starts with
/// the marker, and `SetWordStress` puts its vowel break before it.
fn starts_with_word_break(digits: &str) -> bool {
    !digits.starts_with('0') && (digits.len() <= 2 || (digits.len() > 4 && digits.len() % 3 != 0))
}

/// Stress every word of a grouped number separately, as C does when it
/// translates each group of digits as a word of its own.
fn stress_each_word(
    pronunciation: Pronunciation,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    control: i32,
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut start = 0;
    for (n, end) in pronunciation.word_ends.iter().copied().enumerate() {
        let slice = &pronunciation.bytes[start..end];
        let body = slice.strip_prefix(&[PHON_END_WORD]).unwrap_or(slice);
        let mut word = Vec::with_capacity(body.len() + 2);
        if pronunciation.word_breaks.get(n).copied().unwrap_or(false) {
            word.push(PHON_END_WORD);
        }
        word.extend_from_slice(body);
        word.push(0);
        set_word_stress(&mut word, phdata, stress_opts, Some(0), -1, control);
        out.extend(word.iter().copied().take_while(|&b| b != 0));
        out.push(PHON_END_WORD);
        start = end;
    }
    out.push(0);
    out
}

fn translate_number_token(
    token: &NumberToken,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    grammar: &NumberGrammar,
    speak_symbol: &dyn Fn(&str) -> Vec<u8>,
) -> Option<WordResult> {
    translate_number_token_with_tail(token, dict, phdata, stress_opts, grammar, speak_symbol, None)
}

/// A point's raw phonemes that C's `TranslateNumber_1` appends to the number
/// before it, ahead of word stress.  `fused` joins them to the last digit word
/// (the decimal point of a continued decimal) instead of starting a new word.
struct NumberTail {
    raw: Vec<u8>,
    fused: bool,
}

impl NumberTail {
    fn append_to(&self, phonemes: &mut Vec<u8>) {
        if phonemes.last() == Some(&0) {
            phonemes.pop();
        }
        if self.fused && phonemes.last() == Some(&PHON_END_WORD) {
            phonemes.pop();
        }
        phonemes.extend(&self.raw);
        phonemes.push(0);
    }
}

/// The tail the number at `ix` takes from what follows it: the `_.` entry for a
/// non-decimal point before another number, the `_dpt` entry for a continued
/// decimal (`1.2.3`).
fn number_tail(tokens: &[Token], ix: usize, dict: &Dictionary) -> Option<NumberTail> {
    let raw_of = |key: &str| {
        let raw: Vec<u8> = lookup_num_phonemes(dict, key).into_iter().take_while(|&b| b != 0).collect();
        (!raw.is_empty()).then_some(raw)
    };
    match tokens.get(ix + 1)? {
        Token::Punctuation('.') if point_after_number(tokens, ix + 1) => {
            raw_of("_.").map(|raw| NumberTail { raw, fused: false })
        }
        Token::Number(NumberToken::Decimal { integer, .. }) if integer.is_empty() => {
            raw_of("_dpt").map(|raw| NumberTail { raw, fused: true })
        }
        _ => None,
    }
}

fn translate_number_token_with_tail(
    token: &NumberToken,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    grammar: &NumberGrammar,
    speak_symbol: &dyn Fn(&str) -> Vec<u8>,
    tail: Option<NumberTail>,
) -> Option<WordResult> {
    match token {
        NumberToken::Bare { separator, digits } => {
            let mut phonemes = speak_symbol(&format!("{separator}{digits}"));
            phonemes.extend(individual_digit_word(digits, dict));
            phonemes.push(0);
            set_word_stress(&mut phonemes, phdata, stress_opts, Some(0), -1, 0);
            Some(WordResult { phonemes, dict_flags: 0, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() })
        }
        NumberToken::Ordinal(ordinal) => try_ordinal_number(ordinal, dict, phdata, stress_opts, grammar)
            // The number words come from `_1`…`_20`/`_100` list entries, so C's
            // `TranslateInteger` sets `FLAG_FOUND` on the whole token and the
            // stress-conditioned changes don't apply to them.  (The languages
            // whose numbers this port spells out as *text* instead — Portuguese
            // — go through the rules and keep `found_in_list = false`.)
            .map(|wr| WordResult { found_in_list: true, ..wr }),
        NumberToken::Decimal { integer, fractional, grouped }
            if (*grouped || (MAX_UNSPLIT_DIGITS + 1..=MAX_GROUPED_DIGITS).contains(&integer.len()))
                && !integer.starts_with('0') =>
        {
            // C splits the integer at the separator into words of their own; the
            // last one carries the decimal point and the fraction.
            let mut pronunciation = grouped_pronunciation(integer, dict, grammar);
            pronunciation.word_ends.pop();
            let kept = pronunciation.trimmed_len();
            pronunciation.bytes.truncate(kept);
            push_decimal_part(&mut pronunciation, true, fractional, dict, grammar);
            pronunciation.word_ends.push(pronunciation.bytes.len());
            let mut phonemes = stress_each_word(pronunciation, phdata, stress_opts, STRESS_UNBOUNDED);
            if let Some(tail) = tail.as_ref().filter(|t| t.fused) {
                tail.append_to(&mut phonemes);
            }
            Some(WordResult { phonemes, dict_flags: 0, found_in_list: true, prefix: None, skip_words: 0, late_suffix: Vec::new() })
        }
        NumberToken::Cardinal(digits) if reads_digit_by_digit(digits, grammar) => {
            let phonemes = spelled_digit_groups(digits, dict, phdata, stress_opts, grammar, speak_symbol);
            Some(WordResult { phonemes, dict_flags: 0, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() })
        }
        _ => {
            // Over-long runs are bounded like C's word buffer; composed number words are not.
            let control = match token {
                NumberToken::Cardinal(digits) if digits.len() > MAX_GROUPED_DIGITS => 0,
                _ => STRESS_UNBOUNDED,
            };
            let grouped = match token {
                NumberToken::Cardinal(digits) if digits.len() > MAX_UNSPLIT_DIGITS => {
                    cardinal_pronunciation(digits, dict, grammar).filter(|p| p.word_ends.len() > 1)
                }
                NumberToken::Grouped(digits) => Some(grouped_pronunciation(digits, dict, grammar)),
                _ => None,
            };
            if let Some(pronunciation) = grouped {
                let phonemes = stress_each_word(pronunciation, phdata, stress_opts, control);
                return Some(WordResult { phonemes, dict_flags: 0, found_in_list: true, prefix: None, skip_words: 0, late_suffix: Vec::new() });
            }
            let leading_break = match token {
                NumberToken::Cardinal(digits) | NumberToken::Grouped(digits) => {
                    digits.len() <= MAX_GROUPED_DIGITS && starts_with_word_break(digits)
                }
                NumberToken::Decimal { integer, .. } => {
                    integer.len() <= MAX_GROUPED_DIGITS && starts_with_word_break(integer)
                }
                NumberToken::Ordinal(_) | NumberToken::Bare { .. } => false,
            };
            let mut phonemes = number_token_to_phonemes(token, dict, grammar)?;
            // C's LookupNum3 writes `thousands-digit, break, "thousand", break`:
            // an omitted "one" leaves the break in front.
            let one = lookup_num_phonemes(dict, &num_key(1));
            let omits_one = matches!(token, NumberToken::Cardinal(d)
                if d.len() == 4 && d.starts_with('1') && !phonemes.starts_with(trim_lookup(&one)));
            if leading_break || omits_one {
                phonemes.insert(0, PHON_END_WORD);
            }
            // A `%` in a point's entry marks it unstressed (French `_.`): C
            // leaves the whole number unstressed then.  A fused decimal point
            // is added after the digits' stress, which it does not take part in.
            let mut stress = true;
            if let Some(tail) = tail.as_ref().filter(|t| !t.fused) {
                stress = !tail.raw.contains(&PHON_STRESS_U);
                tail.append_to(&mut phonemes);
            }
            if stress {
                set_word_stress(&mut phonemes, phdata, stress_opts, Some(0), -1, control);
            }
            if let Some(tail) = tail.as_ref().filter(|t| t.fused) {
                tail.append_to(&mut phonemes);
            }
            Some(WordResult { phonemes, dict_flags: 0, found_in_list: true, prefix: None, skip_words: 0, late_suffix: Vec::new() })
        }
    }
}

/// Translate a single lowercase word to phoneme bytes (with stress markers).
///
/// Strategy (mirrors `TranslateWord` in translateword.c):
/// 1. Try dictionary lookup
/// 2. Fall back to translation rules
/// 3. Apply SetWordStress to place stress markers
///
/// Returns the raw phoneme byte sequence with stress markers inserted,
/// and the dictionary flags for post-processing (e.g. strend promotion).
/// Part-of-speech expectation for a word, derived from the *preceding* word (a
/// `FLAG_VERBF`/`NOUNF`/`PASTF` function word like "to"/"the"/"had").  Selects
/// the `$verb`/`$noun`/`$past` homograph pronunciation ("to lˈɪv" vs "lˈaɪv").
#[derive(Clone, Copy, Debug, Default)]
pub struct PosExpect {
    pub verb: bool,
    pub noun: bool,
    pub past: bool,
    /// This word is the last one in its clause.
    ///
    /// Dictionary entries flagged `$atend` only apply there — the English
    /// article "a" is `eI` at a clause end but `a#` in running text, and C
    /// gates it on `word_end < translator->clause_end`.
    pub at_clause_end: bool,
    /// This word starts the clause, for `$atstart` entries.
    pub at_clause_start: bool,
    /// The word as written begins with a capital, for `$capital` entries.
    pub first_upper: bool,
    /// The word as written is all capitals, for `$allcaps` entries.
    ///
    /// English "I" depends on this: its dictionary entry carries
    /// `$u+ $strend $verbf $allcaps $only`, so without the flag the lookup falls
    /// through to a flagless entry and the pronoun keeps primary stress.
    pub all_upper: bool,
    /// The letters of this *same* word that have already been dealt with, for a
    /// rule pre-context that reaches back past the fragment being translated.
    /// NUL-padded, like [`Self::next_word`].
    ///
    /// C translates a word by advancing a pointer through the clause buffer, so
    /// after spelling the unpronounceable letters of `s's` the leftover `'s` is
    /// still matched with the `s` in front of it — which is what picks
    /// `s) 's` → `I2z` over the bare `'s` → `z`.
    pub prev_letters: [u8; 8],
    /// The end of the previous word in the clause, NUL-padded, for a rule
    /// pre-context that reaches back across the word boundary (pt `por_) que`).
    pub prev_tail: [u8; 8],
    /// The first few UTF-8 bytes of the *following* word, NUL-padded.
    ///
    /// A rule's post-context may reach past the word boundary — Portuguese's
    /// `_) o (_C` reads the article "o" as `U` when the next word starts with a
    /// consonant — and C can do that because its rule matcher walks the clause
    /// buffer.  The port translates a word at a time, so the next word's opening
    /// is carried here and appended to the rules buffer.
    pub next_word: [u8; 8],
    /// The lowercased words after this one in its clause, single-spaced with a
    /// trailing space and NUL-padded, for multi-word list entries (`(de een)`).
    pub follow: [u8; 32],
    /// A hyphen joins this word to the next, so it does not end a word in C's
    /// phoneme list and the word-final changes (devoicing) do not apply.
    pub hyphen_after: bool,
    /// `FLAG_HYPHEN` / `FLAG_HYPHEN_AFTER` of the word, for the rule contexts
    /// that match a hyphen (`-) t (-`).
    pub hyphen_flags: u32,
    /// Return the phonemes unstressed: the caller is a prefix branch that
    /// stresses prefix and stem together, as C's `TranslateWord3` does.
    pub defer_stress: bool,
    /// Flags of the whole word, set while its stem is translated after a
    /// prefix was removed.
    pub word_flags: u32,
    /// Leave the `.replace` table out.  C applies it while reading the clause,
    /// so a letter looked up by name (`LookupLetter`) is never substituted.
    pub no_replace: bool,
    /// Skip the whole-word dictionary lookup: C hands the remainder of a word
    /// whose leading letters were spelled straight to `TranslateRules`.
    pub rules_only: bool,
}

/// A [`PosExpect`] carrying only the opening of the word that follows, so a
/// composed multi-word number gets the same cross-boundary rule contexts a typed
/// phrase does (French "vingt et unième" keeps the liaison `t` of "vingt").
fn expect_before(next: Option<&str>) -> PosExpect {
    let mut e = PosExpect::default();
    if let Some(nw) = next {
        let b = nw.as_bytes();
        let n = (0..=b.len().min(8)).rev().find(|&k| nw.is_char_boundary(k)).unwrap_or(0);
        e.next_word[..n].copy_from_slice(&b[..n]);
    }
    e
}

/// The part-of-speech expectation a word inherits from the preceding word.
/// English only; other languages get the default (no expectation).  Kept
/// conservative — only high-confidence verb/past triggers — since a false
/// positive mispronounces an otherwise-correct default.
fn pos_expect_after(prev: Option<&str>) -> PosExpect {
    let Some(p) = prev else { return PosExpect::default() };
    // After "to", a modal, or a subject pronoun, the next content word is
    // (almost always) a verb ("I lˈɪv", "we jˈuːz", "they rɪkˈɔːd").
    const VERB_TRIGGERS: &[&str] = &[
        "to", "will", "would", "shall", "should", "can", "could", "may", "might",
        "must", "cannot", "don't", "doesn't", "didn't", "let", "lets", "please",
        "i", "we", "you", "they", "he", "she", "it",
    ];
    // After a perfect/passive auxiliary, a homograph takes its past form
    // ("have rˈɛd", "was tˈɔːn").
    const PAST_TRIGGERS: &[&str] = &["had", "has", "have", "having", "was", "were", "been"];
    // After a determiner the next word is a noun/adjective, selecting the `$noun`
    // form ("a ˈkɒnvɜːt", "the ˈprɒdjuːs", "the lˈɜːnɪd professor").  Only the ~15
    // `$noun`-flagged words are affected; everything else keeps its default.
    const NOUN_TRIGGERS: &[&str] = &[
        "a", "an", "the", "this", "that", "these", "those", "my", "your", "his",
        "her", "its", "our", "their", "no", "some", "any", "each", "every", "another",
    ];
    PosExpect {
        verb: VERB_TRIGGERS.contains(&p),
        noun: NOUN_TRIGGERS.contains(&p),
        past: PAST_TRIGGERS.contains(&p),
        ..Default::default()
    }
}

/// C's `expect_verb` / `expect_noun` / `expect_past`: the previous word's list
/// flags (`$verbf`, `$nounf`, `$pastf`) say which homograph form the next word takes.
fn pos_expect_after_in(prev: Option<&str>, dict: &Dictionary, dict_condition: u32) -> PosExpect {
    let legacy = pos_expect_after(prev);
    let Some(word) = prev else { return legacy };
    let ctx = LookupCtx { dict_condition, ..Default::default() };
    let flags2 = lookup(dict, word, &ctx).map_or(0, |r| r.flags2.0);
    let past = flags2 & 0x8 != 0;
    let verb = !past && flags2 & 0x1 != 0;
    let noun = !past && !verb && flags2 & 0x4 != 0;
    PosExpect {
        verb: legacy.verb || verb,
        noun: legacy.noun || noun,
        past: legacy.past || past,
        ..legacy
    }
}

pub fn word_to_phonemes(
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    lang_opts: &LangOptions,
) -> WordResult {
    let r = word_to_phonemes_inner(word, dict, phdata, stress_opts, lang_opts, 0, PosExpect::default());
    emoji_parts_fallback(r, word, dict, phdata, stress_opts, lang_opts, PosExpect::default())
}

/// What follows a letter that is spelled out: C's `LookupLetter` ends the
/// buffer with `RULE_SPELLING` (rule symbol `W`) when more letters of the word
/// follow, which is how pt `_) g (_W` tells an acronym's inner letters from
/// its last.
fn letter_expect(more_letters_follow: bool) -> PosExpect {
    let mut expect = PosExpect::default();
    if more_letters_follow {
        expect.next_word[0] = crate::dictionary::RULE_SPELLING;
    }
    expect
}

/// Speak an emoji sequence that has no dictionary entry of its own as its
/// **parts**: the base emoji's name followed by each modifier's name.
///
/// Upstream 1.53.0 ("added skin tone emoji support: sequences are spoken as the
/// base name plus the modifier names") — `👍🏽` is one token so it matches a
/// multi-codepoint entry when one exists, but CLDR does not give every
/// base+tone combination a name, and without this fallback the whole token was
/// **silently dropped** (`👍🏽` produced no output at all).
///
/// Only runs when the whole-sequence lookup produced nothing, so sequences that
/// *do* have an entry (families, flags, subdivision tag flags) are unaffected.
fn emoji_parts_fallback(
    result: WordResult,
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    lang_opts: &LangOptions,
    expect: PosExpect,
) -> WordResult {
    // "Found but empty" counts as nothing spoken: a base+tone key can exist in
    // the dictionary with an empty body (`👍🏽` looks up to a single NUL byte).
    let spoke_something = result.phonemes.iter().any(|&b| b != 0);
    if spoke_something || !word.chars().any(is_skin_tone) {
        return result;
    }
    let base: String = word.chars().filter(|&c| !is_skin_tone(c)).collect();
    let mut parts: Vec<String> = Vec::new();
    if !base.is_empty() {
        parts.push(base);
    }
    parts.extend(word.chars().filter(|&c| is_skin_tone(c)).map(String::from));

    let mut phonemes: Vec<u8> = Vec::new();
    let mut dict_flags = 0;
    let mut found_in_list = false;
    for part in parts {
        let r = word_to_phonemes_inner(&part, dict, phdata, stress_opts, lang_opts, 0, expect);
        if !r.phonemes.iter().any(|&b| b != 0) {
            continue;
        }
        if !phonemes.is_empty() {
            phonemes.push(crate::phoneme::PHON_END_WORD);
        }
        phonemes.extend(r.phonemes);
        dict_flags |= r.dict_flags;
        found_in_list |= r.found_in_list;
    }
    if phonemes.is_empty() {
        return result;
    }
    WordResult { phonemes, dict_flags, found_in_list, prefix: None, skip_words: 0, late_suffix: Vec::new() }
}

/// Like [`word_to_phonemes`] but with a part-of-speech expectation, so a
/// homograph after a trigger word takes its `$verb`/`$noun`/`$past` form.
pub fn word_to_phonemes_pos(
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    lang_opts: &LangOptions,
    expect: PosExpect,
) -> WordResult {
    let r = word_to_phonemes_inner(word, dict, phdata, stress_opts, lang_opts, 0, expect);
    emoji_parts_fallback(r, word, dict, phdata, stress_opts, lang_opts, expect)
}

/// Decompose Korean Hangul syllables (U+AC00–U+D7A3) into conjoining jamo — a
/// leading consonant (U+1100+L), a vowel (U+1161+V), and an optional trailing
/// consonant (U+11A7+T) — which is the form the ko rules match.  Mirrors the
/// Hangul handling in C's `TranslateChar`.  Returns `None` (no allocation) when
/// the word contains no Hangul syllable.
fn decompose_hangul(word: &str) -> Option<String> {
    const SBASE: u32 = 0xAC00;
    const LBASE: u32 = 0x1100;
    const VBASE: u32 = 0x1161;
    const TBASE: u32 = 0x11A7;
    const VCOUNT: u32 = 21;
    const TCOUNT: u32 = 28;
    let is_hangul = |c: char| (SBASE..=0xD7A3).contains(&(c as u32));
    if !word.chars().any(is_hangul) {
        return None;
    }
    let mut out = String::with_capacity(word.len() + 6);
    for c in word.chars() {
        let cp = c as u32;
        if is_hangul(c) {
            let s = cp - SBASE;
            let (l, v, t) = (s / (VCOUNT * TCOUNT), (s % (VCOUNT * TCOUNT)) / TCOUNT, s % TCOUNT);
            // Leading ㅇ (choseong ieung, index 11) is a silent placeholder for a
            // vowel-initial syllable — it never carries the "ng" sound (that is
            // only the *trailing* ᆼ), so omit it rather than emit the jamo's
            // letter-name pronunciation.
            if l != 11 {
                out.push(char::from_u32(LBASE + l).unwrap());
            }
            out.push(char::from_u32(VBASE + v).unwrap());
            if t != 0 {
                out.push(char::from_u32(TBASE + t).unwrap());
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Internal marker in `rule_word_flags` meaning "we are already expanding a
/// TEXTMODE replacement", to stop a replacement from recursively re-expanding.
/// Bit 30 — unused by the rules engine's word flags.
const FLAG_IN_TEXTMODE: u32 = 0x4000_0000;
/// `$unstressend` (dictionary flag bit 11): the word loses its stress when it ends
/// the clause.  C's `FLAG_UNSTRESS_END`, the counterpart of `FLAG_STRESS_END`.
const FLAG_UNSTRESS_END: u32 = 1 << 11;

/// Peel a chain of stacked suffixes.
///
/// Mirrors the `while (more_suffixes)` loop in C's `TranslateWord3()`: while the
/// suffix just removed is marked `SUFX_M`, re-run the rules on the stem and, if
/// that finds another (non-prefix) suffix, remove it too.  Each newly found
/// suffix's phonemes go *before* those collected so far, since the loop walks
/// outside-in.
///
/// Returns the innermost stem still to translate, its `end_type`, the
/// accumulated suffix phonemes, and the stem phonemes of the last
/// re-translation in the chain.  Without this, `s's's` lost the stem and read
/// `s'I2zI2z` instead of `'EsI2zI2z`.
#[allow(clippy::too_many_arguments)]
fn peel_stacked_suffixes(
    word: &str,
    result: &crate::dictionary::rules::RulesResult,
    dict: &Dictionary,
    letter_bits: &[u8; 256],
    lang_opts: &LangOptions,
    phdata: &PhonemeData,
) -> PeeledSuffixes {
    /// Same bound as C's prefix loop; no real word stacks this many suffixes.
    const MAX_SUFFIXES: usize = 50;

    let mut cur_word = word.to_string();
    let mut cur_end_type = result.end_type;
    let mut end_phonemes: Vec<u8> = result.end_phonemes.to_vec();
    let mut stem_phonemes: Option<Vec<u8>> = None;

    for _ in 0..MAX_SUFFIXES {
        if cur_end_type & SUFX_M == 0 {
            break;
        }
        let Some((stem, _, stem_word_flags)) =
            remove_standard_suffix(&cur_word, cur_end_type, dict)
        else {
            break;
        };

        let mut stem_buf = Vec::with_capacity(stem.len() + 3);
        stem_buf.push(b' ');
        stem_buf.extend_from_slice(stem.as_bytes());
        stem_buf.push(b' ');
        stem_buf.push(0);
        let (mut vc, mut sc) = (0i32, 0i32);
        let stem_rules = translate_rules_phdata(
            dict,
            &stem_buf,
            1,
            stem_word_flags,
            0,
            letter_bits,
            lang_opts.dict_condition,
            &mut vc,
            &mut sc,
            Some(phdata),
        );

        if stem_rules.end_type == 0
            || stem_rules.end_type & SUFX_P != 0
            || stem_rules.suffix_start <= 1
        {
            break;
        }

        // This suffix is inside the ones already collected.
        let mut merged: Vec<u8> = stem_rules.end_phonemes.to_vec();
        // Drop the NUL terminator between the two runs, if present.
        if let Some(pos) = merged.iter().position(|&b| b == 0) {
            merged.truncate(pos);
        }
        merged.extend_from_slice(&end_phonemes);
        end_phonemes = merged;

        // What this re-translation made of the stem *without* its own ending.
        // `SUFX_Q` ("don't retranslate") on that ending means the caller must
        // keep exactly this, so it cannot be recomputed later.
        stem_phonemes = Some(
            stem_rules.phonemes.iter().copied().take_while(|&b| b != 0).collect(),
        );
        cur_word = stem;
        cur_end_type = stem_rules.end_type;
    }

    PeeledSuffixes { word: cur_word, end_type: cur_end_type, end_phonemes, stem_phonemes }
}

/// What [`peel_stacked_suffixes`] found.
struct PeeledSuffixes {
    /// The innermost stem still to translate.
    word: String,
    /// That stem's own ending type.
    end_type: u32,
    /// Every suffix's phonemes, innermost first.
    end_phonemes: Vec<u8>,
    /// The stem phonemes produced by the last re-translation in the chain, if
    /// the chain ran at all.
    stem_phonemes: Option<Vec<u8>>,
}

/// Whether `code` is a **tone phoneme** — a `phSTRESS`-type entry that carries
/// its own program, as Mandarin's `55`/`35`/`214`/`51`/`11` do.  Plain stress
/// markers are the same type but have no program and `std_length <= 4`.
fn is_tone_phoneme(code: u8, phdata: &PhonemeData) -> bool {
    matches!(phdata.get(code), Some(ph) if ph.typ == 1 && ph.program != 0)
}

/// Attach each tone phoneme to a syllable's vowel.
///
/// C keeps the tone as a *property* of the vowel (`PHONEME_LIST.tone_ph`), so a
/// syllable carries exactly one tone no matter where the letter-to-sound rules
/// emitted it.  This port keeps tones as ordinary list entries, so they are
/// moved next to the vowel they belong to and any extras are dropped:
///
/// * a tone written after the vowel (Mandarin `ni3`) stays with that vowel, and
///   is pulled back over the syllable's coda — `wen2` is `w'u@35n`, not
///   `w'u@n35`;
/// * a tone written before the vowel (Thai marks the syllable ahead of it)
///   attaches to the following vowel — `kh,a5?…`, not `kh5,a?…`;
/// * a second tone for a vowel that already has one is dropped, which is how
///   one tone per syllable falls out.
///
/// No-op for languages whose tables have no tone phonemes.
fn reorder_tone_phonemes(phonemes: &mut Vec<u8>, phdata: &PhonemeData) {
    if !phonemes.iter().take_while(|&&c| c != 0).any(|&c| is_tone_phoneme(c, phdata)) {
        return;
    }
    let is_vowel = |c: u8| matches!(phdata.get(c), Some(ph) if ph.typ == 2 /* phVOWEL */);
    let is_break = |c: u8| c == 0 || c == crate::phoneme::PHON_END_WORD;

    let mut out: Vec<u8> = Vec::with_capacity(phonemes.len());
    // The most recent vowel in `out` and whether it already carries a tone —
    // one tone per syllable, as in C where the tone is a field on the vowel.
    let mut last_vowel: Option<usize> = None;
    let mut last_vowel_toned = false;
    let mut pending_tone: Option<u8> = None;

    for &code in phonemes.iter() {
        if is_break(code) {
            // A tone with no vowel to attach to in this word is dropped.
            pending_tone = None;
            last_vowel = None;
            last_vowel_toned = false;
            out.push(code);
            if code == 0 {
                break;
            }
            continue;
        }
        if is_tone_phoneme(code, phdata) {
            match last_vowel {
                // Pull the tone back onto this syllable's vowel, over any coda.
                Some(v) if !last_vowel_toned => {
                    out.insert(v + 1, code);
                    last_vowel_toned = true;
                }
                // The syllable already has its tone: drop the extra.
                Some(_) => {}
                // Written ahead of the syllable (Thai): hold it for the vowel.
                None => {
                    if pending_tone.is_none() {
                        pending_tone = Some(code);
                    }
                }
            }
            continue;
        }
        out.push(code);
        if is_vowel(code) {
            last_vowel = Some(out.len() - 1);
            last_vowel_toned = false;
            if let Some(tone) = pending_tone.take() {
                out.push(tone);
                last_vowel_toned = true;
            }
        }
    }
    *phonemes = out;
}

fn word_to_phonemes_inner(
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    lang_opts: &LangOptions,
    rule_word_flags: u32,
    expect: PosExpect,
) -> WordResult {
    // Apply the rules file's `.replace` table before dict/rule lookup, exactly
    // as C does per-character in `TranslateChar`.  Japanese uses it to fold
    // full-width katakana onto hiragana and decompose precomposed voiced kana
    // into base + combining dakuten (the form the ja rules match); without it
    // katakana and every voiced kana produce no phonemes.  Other languages use
    // it for their own normalisation.  No-op (None, no alloc) without a
    // `.replace` section; runs here so it also covers TEXTMODE re-expansion.
    let normalized = if expect.no_replace || lang_opts.switched {
        None
    } else {
        dict.apply_replacements(word)
    };
    let word = normalized.as_deref().unwrap_or(word);

    // Korean: break each Hangul syllable into its jamo (the ko rules match the
    // decomposed leading/vowel/trailing jamo, not the precomposed syllable).
    let hangul = decompose_hangul(word);
    let word = hangul.as_deref().unwrap_or(word);

    let ctx = LookupCtx {
        // Not a symbol-name lookup: C only sets `lookup_symbol` when resolving a
        // punctuation/symbol name, and it is what disables the `$atend` gate.
        lookup_symbol: false,
        at_clause_end: expect.at_clause_end,
        is_first_word: expect.at_clause_start,
        word_flags: (if expect.first_upper { crate::dictionary::FLAG_FIRST_UPPER } else { 0 })
            | (if expect.all_upper { crate::dictionary::FLAG_ALL_UPPER } else { 0 }),
        expect_verb: expect.verb,
        expect_noun: expect.noun,
        expect_past: expect.past,
        dict_condition: lang_opts.dict_condition,
        // A stem left by a removed prefix is looked up as C does with `SUFX_P`,
        // which skips `$only` and `$onlys` entries.
        end_flags: if rule_word_flags & FLAG_PREFIX_REMOVED != 0 { SUFX_P } else { 0 },
        ..Default::default()
    };

    // Try dictionary first
    let follow = expect.follow.split(|&b| b == 0).next().unwrap_or(&[]);
    let dict_result = if expect.rules_only { None } else { lookup_with_follow(dict, word, &ctx, follow) };

    // Extract flags from dict even if no phonemes (FLAGS-only entries).
    // Note: FLAGS-only entries have FLAG_FOUND_ATTRIBUTES (bit 30) but NOT FLAG_FOUND (bit 31).
    const FLAG_FOUND_ATTRIBUTES: u32 = 0x4000_0000;
    let dict_flags_from_lookup = dict_result.as_ref()
        .filter(|r| r.flags1.0 & (FLAG_FOUND_ATTRIBUTES | 0x8000_0000) != 0)
        .map(|r| r.flags1.0)
        .unwrap_or(0);
    // A stem translated after prefix removal gets its rule conditions from the
    // flags of the whole word.
    let rules_flags = if expect.word_flags != 0 { expect.word_flags } else { dict_flags_from_lookup };

    // An `$accent` entry without a pronunciation is spoken as the letter's
    // accent-table name (`é` → "e acute").
    if let Some(ref result) = dict_result {
        if result.flags2.contains(crate::dictionary::FLAG_ACCENT)
            && result.phonemes.iter().all(|&b| b == 0)
        {
            let letter = word.strip_prefix('_').unwrap_or(word).chars().next();
            let mut phonemes = letter
                .map(|c| accented_letter_phonemes(c, dict, phdata, stress_opts, lang_opts))
                .unwrap_or_default();
            if !phonemes.is_empty() {
                phonemes.push(0);
                let dict_flags = result.flags1.0;
                set_word_stress(&mut phonemes, phdata, stress_opts, Some(dict_flags), -1, 0);
                return WordResult { phonemes, dict_flags, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() };
            }
        }
    }

    if let Some(ref result) = dict_result {
        if result.flags1.found() && !result.phonemes.is_empty() {
            // TEXTMODE entry: the "phonemes" are really a replacement *text*
            // (emoji-name entries like `😀`→"grinning face", `$text` rules).
            // Re-translate it as words instead of reading the bytes as phonemes.
            // `textmode` is *reversed* for languages whose dictionary is written
            // as replacement text (cmn/yue/zh), so compare against the language's
            // default rather than testing the bit directly.
            let is_textmode = result.flags1.textmode() != lang_opts.reversed_textmode;
            if is_textmode && rule_word_flags & FLAG_IN_TEXTMODE == 0 {
                let replacement = String::from_utf8_lossy(&result.phonemes);
                let replacement = replacement.trim_end_matches('\0');
                // Run the replacement through the *tokenizer*, not just
                // `split_whitespace`: a script written without spaces (Thai,
                // Chinese) needs the same word segmentation as ordinary input,
                // or only its first word is spoken.
                let mut phonemes: Vec<u8> = Vec::new();
                // Set when a part is dropped because its translation is a
                // language-switch marker: the entry *was* understood, so the
                // word must come out silent rather than fall through to reading
                // the replacement text as phoneme codes.
                let mut dropped_switch = false;
                // Tokenize rather than just splitting on whitespace: a script
                // written without spaces (Thai, Chinese) needs the same word
                // segmentation as ordinary input, or only its first word is
                // spoken (`th 👍` stopped after two syllables of four words).
                // Anything the tokenizer doesn't turn into a word or a number
                // falls back to the whitespace pieces.
                let tokens = tokenize_opts(replacement, &lang_opts.number_grammar);
                let mut parts: Vec<String> = Vec::new();
                let mut usable = true;
                for t in &tokens {
                    match t {
                        Token::Word(w) => parts.push(w.clone()),
                        Token::Number(NumberToken::Cardinal(n)) => parts.push(n.clone()),
                        Token::Number(NumberToken::Decimal { integer, fractional, .. }) => {
                            parts.push(format!("{integer}.{fractional}"))
                        }
                        Token::Space | Token::WordJoin | Token::ClauseBoundary(_) | Token::Punctuation(_) => {}
                        _ => usable = false,
                    }
                }
                if !usable || parts.is_empty() {
                    parts = replacement.split_whitespace().map(str::to_string).collect();
                }
                for part in &parts {
                    // Re-translate each replacement word the way the tokenizer
                    // hands an input word over: lower-cased for the dictionary
                    // and the letter rules, with the capitalisation carried in
                    // the flags instead.  German names every symbol with a
                    // capitalised noun (`∑` → "Summenzeichen", `😀` →
                    // "grinsendes Gesicht"); without this the rules match
                    // nothing, the part is dropped, and a single-word reading
                    // falls through to being read as raw phoneme *codes*
                    // (`-v de ∑` → `B'AE,EQ@,aIQqJ^Q"QE2`).
                    let lower = part.to_lowercase();
                    let letters: Vec<char> =
                        part.chars().filter(|c| c.is_alphabetic()).collect();
                    // Clause position is deliberately *not* passed down: an
                    // `$atend` entry is chosen by the word's position in the
                    // clause buffer, and a replacement word never has one, so C
                    // reads `≤` as "less than or equal **tU**", the running-text
                    // form, even at the end of a clause.  The clause-end flags
                    // that do apply (`$u+`, `$strend`, `$unstressend`) are
                    // handled below instead.
                    let part_expect = PosExpect {
                        first_upper: part.chars().next().is_some_and(char::is_uppercase),
                        all_upper: !letters.is_empty()
                            && letters.iter().all(|c| c.is_uppercase()),
                        ..PosExpect::default()
                    };
                    let wr = word_to_phonemes_inner(
                        &lower, dict, phdata, stress_opts, lang_opts,
                        rule_word_flags | FLAG_IN_TEXTMODE, part_expect,
                    );
                    // The clause-end stress attributes *do* apply, and C
                    // applies them to every word of the replacement rather than
                    // just its last — `÷` reads "divided b'aI", `es ™` "símbolo
                    // D'e marca comercial", and Hungarian's `$unstressend`
                    // words go the other way (`→` → `j,ob: _!,oldAlR2A …`).
                    // They only apply when the symbol itself ends the clause:
                    // `a 👍 b` and `👍 the` both keep "up" reduced.
                    let mut owned;
                    let mut ph = wr.phonemes.strip_suffix(&[0]).unwrap_or(&wr.phonemes);
                    if expect.at_clause_end
                        && wr.dict_flags
                            & (crate::dictionary::FLAG_STRESS_END
                                | crate::dictionary::FLAG_STRESS_END2
                                | FLAG_UNSTRESS_END)
                            != 0
                    {
                        owned = ph.to_vec();
                        let to = if wr.dict_flags & FLAG_UNSTRESS_END != 0 { 3 } else { 4 };
                        change_word_stress(&mut owned, phdata, to);
                        ph = &owned[..];
                    }
                    if ph.is_empty() {
                        continue;
                    }
                    // A replacement word whose translation is a language-switch
                    // marker (German `©` → "Copyright", which the de rules send
                    // to English) can't be switched from here — the switch is
                    // applied one level up, per clause token.  Drop the part
                    // rather than emit the marker as if it were a phoneme; C
                    // produces nothing for these too.
                    if ph[0] == crate::phoneme::PHON_SWITCH {
                        dropped_switch = true;
                        continue;
                    }
                    if !phonemes.is_empty() {
                        phonemes.push(crate::phoneme::PHON_END_WORD);
                    }
                    phonemes.extend_from_slice(ph);
                }
                if !phonemes.is_empty() || dropped_switch {
                    let dot_flags = result.flags1.0
                        & (crate::dictionary::FLAG_ALLOW_DOT | crate::dictionary::FLAG_NEEDS_DOT);
                    return WordResult { phonemes, dict_flags: dot_flags, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() };
                }
            }
            let dict_flags = result.flags1.0;
            // `-X`: C prints `Found: 'word' [phonemes]  flags` and runs no rules.
            if crate::dictionary::rules::trace_enabled() && rule_word_flags == 0 {
                crate::dictionary::rules::trace_found(
                    word,
                    &result.phonemes,
                    [result.flags1.0, result.flags2.0],
                );
            }
            // Language-switch entry (`_^_LL`): the phonemes are `phonSWITCH`
            // followed by the target language name.  Return them verbatim —
            // stress placement must not touch the embedded language bytes; the
            // caller re-translates the word with the target language.
            if result.phonemes[0] == crate::phoneme::PHON_SWITCH {
                return WordResult {
                    phonemes: result.phonemes.clone(),
                    dict_flags,
                    found_in_list: true,
                    prefix: None,
                    skip_words: result.skipwords,
                    late_suffix: Vec::new(),
                };
            }
            let mut phonemes = result.phonemes.clone();
            if expect.defer_stress {
                return WordResult { phonemes, dict_flags, found_in_list: true, prefix: None, skip_words: 0, late_suffix: Vec::new() };
            }
            // Apply stress placement
            set_word_stress(&mut phonemes, phdata, stress_opts, Some(dict_flags as u32), -1, 0);
            // (No stressed-vowel upgrading here: the pronunciation came from the
            // list, and C's `isTranslationGiven` gate keeps the vowel changes off
            // a word whose phonemes were given — `tr b` is `b'e`, not `b'E`.)
            if stress_opts.alt_attributes
                && dict_flags & (crate::dictionary::FLAG_ALT_TRANS
                    | crate::dictionary::FLAG_ALT2_TRANS) != 0
            {
                let alt2 = dict_flags & crate::dictionary::FLAG_ALT2_TRANS != 0;
                apply_special_attribute2(&mut phonemes, phdata, alt2);
            }
            // Word-final devoicing (e.g. German Auslautverhärtung)
            if stress_opts.word_final_devoicing && !expect.hyphen_after {
                apply_word_final_devoicing(&mut phonemes, phdata);
            }
            return WordResult {
                phonemes,
                dict_flags,
                found_in_list: true,
                prefix: None,
                skip_words: result.skipwords,
                late_suffix: Vec::new(),
            };
        }
    }

    if rule_word_flags == 0 {
        if let Some(token) = NumberToken::parse(word, &lang_opts.number_grammar) {
            if let Some(result) =
                translate_number_token(
                    &token,
                    dict,
                    phdata,
                    stress_opts,
                    &lang_opts.number_grammar,
                    &|symbol| symbol_phonemes(symbol, dict, lang_opts),
                )
            {
                return result;
            }
        }
    }

    // `-X`: announce the word before its rule matches, as C's `TranslateWord`
    // does.  Only top-level words (a recursive expansion keeps the same header).
    if crate::dictionary::rules::trace_enabled() && rule_word_flags == 0 {
        crate::dictionary::rules::trace_word(word);
    }

    // Fall back to translation rules (potentially using dict flags from FLAGS-only entry)
    // Use the dictionary's language-specific letter_bits (Cyrillic, Arabic, etc.)
    //
    // `FLAG_IN_TEXTMODE` is *our* marker, not one of C's word flags — it must not
    // reach the rules engine, where bit 30 is `FLAG_FOUND_ATTRIBUTES` and changes
    // how the word is matched.  (Symptom: a Thai emoji whose replacement text is
    // a spaceless run of words stopped translating after the first two
    // syllables.)
    let rule_word_flags = rule_word_flags & !FLAG_IN_TEXTMODE;
    let letter_bits = &*dict.letter_bits;
    let mut vowel_count = 0i32;
    let mut stressed_count = 0i32;

    // Prepare word buffer with leading space (for rule pre-context), and with
    // the start of the *next* word after it so a post-context can reach across
    // the boundary (`_) o (_C`).  The translation loop itself stops at the
    // space, so the extra text is only ever read by a context.
    let mut word_buf = Vec::with_capacity(word.len() + 20);
    let tail = expect.prev_tail.split(|&b| b == 0).next().unwrap_or(&[]);
    word_buf.extend_from_slice(tail);
    word_buf.push(b' ');
    // Letters of the same word that come *before* this fragment, so a rule's
    // pre-context can see them.  They are not translated: `word_start` points
    // past them, and because the test for `_`/start-of-word reads the buffer
    // byte itself, the fragment correctly stops counting as a word start.
    let prev = expect.prev_letters.split(|&b| b == 0).next().unwrap_or(&[]);
    word_buf.extend_from_slice(prev);
    let word_start = word_buf.len();
    word_buf.extend_from_slice(word.as_bytes());
    word_buf.push(b' ');
    let next = expect.next_word.split(|&b| b == 0).next().unwrap_or(&[]);
    if !next.is_empty() {
        word_buf.extend_from_slice(next);
        word_buf.push(b' ');
    }
    word_buf.push(0);

    // C passes the flags from the *list* lookup into `TranslateRules`
    // (`dict_flags[0]`), which is what lets a `$w_alt<n>` rule condition see a
    // flags-only entry: `en_list`'s `banana $alt3` is what selects the `a#`
    // rule, giving upstream's `ba#n'A:n@`.
    let result = translate_rules_phdata(
        dict,
        &word_buf,
        word_start, // past the leading space and any in-word pre-context
        rule_word_flags | expect.hyphen_flags,
        rules_flags,
        &letter_bits,
        lang_opts.dict_condition,
        &mut vowel_count,
        &mut stressed_count,
        Some(phdata),
    );
    let result = confirm_prefix(
        result, word, &word_buf, word_start, rule_word_flags, rules_flags, dict, lang_opts, phdata,
    );
    if result.phonemes.first() == Some(&crate::phoneme::PHON_SWITCH) && result.phonemes.len() > 2 {
        return WordResult {
            phonemes: result.phonemes.clone(),
            dict_flags: 0,
            found_in_list: false,
            prefix: None,
            skip_words: 0,
            late_suffix: Vec::new(),
        };
    }

    // C's second `TranslateRules` call, with `end_phonemes == NULL`: translate the
    // whole word straight through, ignoring prefix/suffix endings.  Needed for
    // every path where the ending can't actually be removed, because the first
    // call stops as soon as an ending matches and would otherwise leave a
    // half-translated word.
    let translate_whole_word = || {
        let (mut vc, mut sc) = (0i32, 0i32);
        let r = crate::dictionary::rules::translate_rules_ext(
            dict,
            &word_buf,
            word_start,
            rule_word_flags,
            rules_flags,
            &letter_bits,
            lang_opts.dict_condition,
            &mut vc,
            &mut sc,
            Some(phdata),
            false,
        );
        combine_rules_result(&r)
    };
    // Only a *prefix* match cuts the rules scan short, so only then does the
    // partial result need re-translating; after a suffix match `result` still
    // holds the whole word, as it always did.
    let fallback_phonemes = |result: &crate::dictionary::rules::RulesResult| {
        if result.end_type & SUFX_P != 0 {
            translate_whole_word()
        } else {
            combine_rules_result(result)
        }
    };

    fn rules_produced_output(r: &crate::dictionary::rules::RulesResult) -> bool {
        if r.spellword || r.end_type != 0 {
            return true;
        }
        r.phonemes.iter().any(|&b| b != 0) || r.end_phonemes.iter().any(|&b| b != 0)
    }

    // A lone letter the rules don't cover is spoken as its name, built from the
    // accent tables when it has none (C's `LookupLetter` fallback).
    if result.spellword
        && rule_word_flags == 0
        && word.chars().count() == 1
        && result.phonemes.iter().all(|&b| b == 0)
    {
        if let Some(c) = word.chars().next() {
            let mut phonemes = accented_letter_phonemes(c, dict, phdata, stress_opts, lang_opts);
            if !phonemes.is_empty() {
                phonemes.push(0);
                set_word_stress(&mut phonemes, phdata, stress_opts, Some(dict_flags_from_lookup), -1, 0);
                return WordResult { phonemes, dict_flags: dict_flags_from_lookup, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() };
            }
        }
    }

    if rules_produced_output(&result) {
        let mut stress_dict_flags = dict_flags_from_lookup;
        let mut stress_done = false;
        let mut peeled: Option<PeeledPrefix> = None;
        let mut late_suffix: Vec<u8> = Vec::new();
        let listed_with_prefix = remove_standard_prefix(word, result.end_type).and_then(|(stem, flags)| {
            prefix_with_stem_listed(word, &stem, flags, rules_flags, dict, lang_opts, phdata, expect)
        });
        let mut phonemes = if let Some((listed, listed_flags)) = listed_with_prefix {
            if stress_dict_flags == 0 {
                stress_dict_flags = listed_flags;
            }
            listed
        } else if result.end_type != 0 && (result.end_type & SUFX_P) != 0 {
            if let Some((stem, stem_word_flags)) = remove_standard_prefix(word, result.end_type) {
                let mut prefix_ph = Vec::new();
                append_raw_phonemes(&mut prefix_ph, &result.end_phonemes);
                let stem_wr = word_to_phonemes_inner(
                    &stem,
                    dict,
                    phdata,
                    stress_opts,
                    lang_opts,
                    stem_word_flags,
                    PosExpect { defer_stress: true, word_flags: rules_flags, ..expect },
                );
                let mut stem_ph = Vec::new();
                append_raw_phonemes(&mut stem_ph, &stem_wr.phonemes);
                late_suffix = stem_wr.late_suffix;
                // Prefixes the stem peeled itself join this word's prefix run.
                let inner = stem_wr.prefix;
                let inner_len = inner.map_or(0, |p| p.len).min(stem_ph.len());
                prefix_ph.extend(stem_ph.drain(..inner_len));
                let stale_stem_flags = !late_suffix.is_empty() && stem_wr.dict_flags != 0;
                let prefix_flags =
                    stress_dict_flags != 0 || inner.is_some_and(|p| p.flagged) || stale_stem_flags;
                if stress_dict_flags == 0 {
                    stress_dict_flags = stem_wr.dict_flags;
                }
                let prefix_stressed = prefix_ph
                    .iter()
                    .any(|&b| b == crate::phoneme::PHON_STRESS_P || b == crate::phoneme::PHON_STRESS_P2);
                // C stresses the stem alone unless the prefix or a list entry
                // already fixes the stress of the whole word.  A word that is
                // itself the stem of an outer prefix leaves all stress to the
                // outermost call, which sees the whole prefix run.
                if !expect.defer_stress {
                    let flags = Some(stress_dict_flags as u32);
                    if prefix_flags || prefix_stressed {
                        if stress_opts.prefixes || result.end_type & crate::dictionary::SUFX_T != 0 {
                            stem_ph.push(0);
                            set_word_stress(&mut stem_ph, phdata, stress_opts, flags, 3, 0);
                            stem_ph.retain(|&b| b != 0);
                            reduce_later_primaries(&mut prefix_ph);
                        }
                    } else {
                        stem_ph.push(0);
                        let control = if late_suffix.is_empty() { 0 } else { 2 };
                        set_word_stress(&mut stem_ph, phdata, stress_opts, flags, -1, control);
                        stem_ph.retain(|&b| b != 0);
                        stress_done = true;
                    }
                }
                peeled = Some(PeeledPrefix { len: prefix_ph.len(), flagged: prefix_flags });
                let mut combined = prefix_ph;
                combined.extend_from_slice(&stem_ph);
                combined.push(0);
                combined
            } else {
                let mut fallback = fallback_phonemes(&result);
                fallback.push(0);
                fallback
            }
        } else if result.end_type != 0 && result.suffix_start > 1 {
            // A word can stack suffixes when the outer one is marked `SUFX_M`
            // ("allow more suffixes before this suffix") — English `'s` is, so
            // `s's's` is stem `s` + `'s` + `'s`.  Peel them innermost-last,
            // accumulating each suffix's phonemes in front of the ones already
            // collected (C's `strcpy(end_phonemes2, end_phonemes); … strcat`).
            // Bounded like C's prefix loop; upstream left this loop uncapped,
            // which is how a 440-byte `s's's…` smashed its stack (#2495).
            let keeps_stem_stress = result.end_type & crate::dictionary::SUFX_T != 0;
            let peeled =
                peel_stacked_suffixes(word, &result, dict, &letter_bits, lang_opts, phdata);
            let word = peeled.word.as_str();
            let result_end_type = peeled.end_type;
            let result_end_phonemes = peeled.end_phonemes;
            let peeled_stem_phonemes = peeled.stem_phonemes;
            if let Some((stem, end_flags, stem_word_flags)) =
                remove_standard_suffix(word, result_end_type, dict)
            {
                let stem_lookup = lookup(
                    dict,
                    &stem,
                    &LookupCtx {
                        // $atend entries apply only at clause end; lookup_symbol: true would bypass that gate for a stem.
                        lookup_symbol: false,
                        end_flags,
                        expect_verb: expect.verb,
                        expect_noun: expect.noun,
                        expect_past: expect.past,
                        dict_condition: lang_opts.dict_condition,
                        ..Default::default()
                    },
                );

                let mut combined = Vec::new();
                let mut used_stem = false;

                // `SUFX_Q` on the ending means "don't retranslate": when the
                // stem isn't in the dictionary, keep what the *whole word*
                // produced instead of re-running the rules on the stem.  Italian
                // "casa" matches `@C) a (_S1q`, and the retranslation loses the
                // intervocalic `A) s (A → z` (`k'asa` for `k'aza`).
                let no_retranslate = result_end_type & crate::dictionary::SUFX_Q != 0;

                if let Some(stem_lookup) = stem_lookup {
                    if !stem_lookup.phonemes.is_empty() {
                        combined.extend_from_slice(&stem_lookup.phonemes);
                        // C takes the stem's flags only when the word's own entry has none.
                        if stress_dict_flags == 0 {
                            stress_dict_flags = stem_lookup.flags1.0;
                        }
                        used_stem = true;
                    } else {
                        // A *flags-only* entry for the stem still applies, as it
                        // does in C ("Flags:  somm   $alt2" in its trace): the
                        // flags decide the word's stress and its alternate vowel
                        // quality even though the pronunciation comes from the
                        // rules.  Italian `somma` reads `s'omma` because of it.
                        // As for a stem with phonemes, only a word whose own
                        // entry has no flags takes them: `privilegia $3` keeps
                        // its `e` although the stem `privilegi` is `$alt`.
                        if stress_dict_flags == 0 {
                            stress_dict_flags = stem_lookup.flags1.0;
                        }
                    }
                }

                // (With `SUFX_Q` the stem is left untranslated, so `used_stem`
                // stays false and the fallback below — the whole word's own
                // phonemes — is what gets used.)
                if !used_stem && !no_retranslate {
                    let mut stem_buf = Vec::with_capacity(stem.len() + 3);
                    stem_buf.push(b' ');
                    stem_buf.extend_from_slice(stem.as_bytes());
                    stem_buf.push(b' ');
                    stem_buf.push(0);

                    let mut stem_vc = 0i32;
                    let mut stem_sc = 0i32;
                    let stem_rules = translate_rules_phdata(
                        dict,
                        &stem_buf,
                        1,
                        stem_word_flags,
                        rules_flags,
                        &letter_bits,
                        lang_opts.dict_condition,
                        &mut stem_vc,
                        &mut stem_sc,
                        Some(phdata),
                    );
                    // A stem can itself start with a standard prefix (German
                    // `be-`, `ge-`); the rules stop at it, so hand that case to
                    // the full word translator instead of keeping the fragment.
                    let stem_phonemes = if stem_rules.end_type & SUFX_P != 0 {
                        word_to_phonemes_inner(
                            &stem, dict, phdata, stress_opts, lang_opts, stem_word_flags, expect,
                        )
                        .phonemes
                    } else {
                        combine_rules_result(&stem_rules)
                    };
                    if stem_phonemes.iter().any(|&b| b != 0) {
                        append_raw_phonemes(&mut combined, &stem_phonemes);
                        used_stem = true;
                    }
                }

                if used_stem {
                    let ending = if keeps_stem_stress { &mut late_suffix } else { &mut combined };
                    append_raw_phonemes(ending, &result_end_phonemes);
                    combined.push(0);
                    combined
                } else if let Some(stem_ph) = peeled_stem_phonemes
                    .filter(|ph| no_retranslate && ph.iter().any(|&b| b != 0))
                {
                    // `SUFX_Q` reached through a stacked suffix: the inner
                    // ending forbids retranslating its stem, so keep what the
                    // chain's own re-translation made of it and add every
                    // suffix on top.  Falling back to the *whole word's*
                    // phonemes here dropped the inner ending — Dutch
                    // `vorige` (= `vorig` + `-e`, where `ig` is a `S2dq`
                    // ending) read `v'o:r@` instead of `v'o:r@Q@`.
                    let mut combined = stem_ph;
                    append_raw_phonemes(&mut combined, &result_end_phonemes);
                    combined.push(0);
                    combined
                } else {
                    let mut fallback = fallback_phonemes(&result);
                    fallback.push(0);
                    fallback
                }
            } else {
                let mut fallback = fallback_phonemes(&result);
                fallback.push(0);
                fallback
            }
        } else if result.end_type != 0 {
            // An ending matched but neither branch could use it (e.g. a prefix
            // whose letters can't be removed): translate the word in full rather
            // than keep the partial result the scan stopped at.
            let mut combined = fallback_phonemes(&result);
            combined.push(0);
            combined
        } else {
            let mut combined = combine_rules_result(&result);
            combined.push(0);
            combined
        };

        // Apply stress placement; use dict flags from FLAGS-only entry if available
        let flags_for_stress = if stress_dict_flags != 0 {
            Some(stress_dict_flags as u32)
        } else {
            Some(0)  // non-NULL mirrors C's behavior of passing non-NULL dictionary_flags
        };
        if expect.defer_stress {
            return WordResult {
                phonemes,
                dict_flags: stress_dict_flags,
                found_in_list: false,
                prefix: peeled,
                skip_words: 0,
                late_suffix,
            };
        }
        if !stress_done {
            let control = if late_suffix.is_empty() || peeled.is_some() { 0 } else { 2 };
            set_word_stress(&mut phonemes, phdata, stress_opts, flags_for_stress, -1, control);
        }
        append_late_suffix(&mut phonemes, &late_suffix);
        // Stressed-vowel upgrading (e.g. Turkish e→E, o→O under primary stress)
        if stress_opts.alt_stress_upgrade {
            apply_alt_stress_upgrade(&mut phonemes, phdata);
        }
        if stress_opts.alt_attributes {
            let flags = stress_dict_flags;
            if flags & (crate::dictionary::FLAG_ALT_TRANS | crate::dictionary::FLAG_ALT2_TRANS) != 0
            {
                let alt2 = flags & crate::dictionary::FLAG_ALT2_TRANS != 0;
                apply_special_attribute2(&mut phonemes, phdata, alt2);
            }
        }
        // Word-final devoicing (e.g. German Auslautverhärtung)
        if stress_opts.word_final_devoicing && !expect.hyphen_after {
            apply_word_final_devoicing(&mut phonemes, phdata);
        }
        // Tone phonemes belong to their syllable's vowel, not to the end of the
        // syllable where the tone digit was written (no-op for atonal languages).
        reorder_tone_phonemes(&mut phonemes, phdata);
        // Pronounced by the rules: only the *flags* came from the list, so
        // this is not `FLAG_FOUND` and the stress-conditioned changes still apply.
        return WordResult { phonemes, dict_flags: stress_dict_flags, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() };
    }

    // Could not translate (unknown word)
    WordResult { phonemes: Vec::new(), dict_flags: dict_flags_from_lookup, found_in_list: false, prefix: None, skip_words: 0, late_suffix: Vec::new() }
}

// ---------------------------------------------------------------------------
// Translator
// ---------------------------------------------------------------------------

/// One sentence / clause unit for dictionary + stress promotion (`text_to_ipa` and
/// `translate_to_codes` share this pipeline — GitHub #4 and related parity).
#[derive(Clone, PartialEq)]
pub(crate) enum TranslateEntryKind {
    Word,
    ClauseBoundary,
    Other,
    /// A `phonSWITCH` language-switch marker (`_^_LL`): `phonemes` is
    /// `[phonSWITCH, <target-language bytes…>]`.  Excluded from word-level
    /// stress promotion; resolved at render time.
    LangSwitch,
    /// A letter spelled with names from several phoneme tables: `phonemes`
    /// holds `letters::encode` items, switches included.
    Stream,
}

pub(crate) struct TranslateEntry {
    pub(crate) phonemes:   Vec<u8>,
    pub(crate) dict_flags: u32,
    /// The word's phonemes came from the dictionary *list* (C's `FLAG_FOUND`),
    /// which suppresses the stress-conditioned phoneme changes.
    pub(crate) found_in_list: bool,
    pub(crate) kind:       TranslateEntryKind,
    pub(crate) word_lower: Option<String>,
    /// Suppress the language's inter-word gap after this word.
    ///
    /// Mandarin's third-tone sandhi *joins* the pair it applies to: 你好 reads
    /// as `ni35X'Au214_|` — one gap, at the end — not as two separate words.
    pub(crate) no_word_gap: bool,
    /// The letters of one abandoned word share this id (the index of the first),
    /// so that clause-final stress sees them as the word they came from.
    pub(crate) word_group: Option<usize>,
}

/// Parse the content of a `[[ … ]]` inline-phoneme span into phoneme codes.
///
/// Mirrors espeak's `EncodePhonemes` / `LookupPhonemeString`: the mnemonic
/// string is scanned left-to-right, greedily matching the **longest** phoneme
/// mnemonic (1–4 characters) present in the currently-selected phoneme table.
/// Whitespace separates mnemonics; unrecognised characters are skipped.
///
/// Stress marks such as `'` and `,` are ordinary phonemes in the table, so
/// they are matched here and rendered as stress by the IPA stage.
pub(crate) fn parse_inline_phonemes(content: &str, phdata: &PhonemeData) -> Vec<u8> {
    let bytes = content.as_bytes();
    let mut codes = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            // A space inside `[[…]]` is a word boundary, exactly as in normal
            // text: without the separator `[[h@l'oU w'3:ld]]` ran together as
            // one word (`h@l'oUw'3:ld`) in `-x` and in the audio.
            if !codes.is_empty() && codes.last() != Some(&crate::phoneme::PHON_END_WORD) {
                codes.push(crate::phoneme::PHON_END_WORD);
            }
            i += 1;
            continue;
        }
        let maxlen = 4.min(bytes.len() - i);
        let mut matched = false;
        for len in (1..=maxlen).rev() {
            let Some(slice) = content.get(i..i + len) else { continue };
            let code = phdata.lookup_phoneme(slice);
            if code != 0 {
                codes.push(code);
                i += len;
                matched = true;
                if code == crate::phoneme::PHON_SWITCH {
                    // `phonSWITCH` is followed by a language name, which C
                    // lowercases and drops entirely when it is the default voice
                    // (`_^_EN` in de_list is just "switch to the default").
                    let start = i;
                    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                        codes.push(bytes[i].to_ascii_lowercase());
                        i += 1;
                    }
                    let name_at = codes.len() - (i - start);
                    if &codes[name_at..] == b"en" {
                        codes.truncate(name_at);
                    }
                }
                break;
            }
        }
        if !matched {
            i += 1; // skip an unrecognised character
        }
    }
    codes
}

/// Apply a segment's SSML [`SayAs`](ssml::SayAs) interpretation to its text,
/// given the language it will be translated in.  `Ordinal` appends the English
/// ordinal suffix to integer tokens ("3" → "3rd"), which the number path then
/// reads as an ordinal; other languages/modes pass through unchanged.
/// Process SSML into the plain text the synthesizer should speak, for the audio
/// path: strips tags, decodes entities, applies text-level `<say-as>` spelling
/// (`characters`/`digits`/`telephone`) **and** the language-aware interpret modes
/// (`ordinal`/`date`/`time`) — the latter were previously only applied on the
/// `--ipa`/`-x` path.  `<voice>` language spans are read in `lang` (audio voice
/// switching is not yet wired).
pub fn ssml_to_speech_text(text: &str, lang: &str) -> String {
    ssml_to_speech_text_opts(text, lang, 100)
}

/// [`ssml_to_speech_text`] with the `--ssml-break` percentage
/// (`espeakSSML_BREAK_MUL`) applied to every `<break time=…>`.
pub fn ssml_to_speech_text_opts(text: &str, lang: &str, ssml_break: u32) -> String {
    ssml::process_markup_opts(text, ssml_break)
        .0
        .into_iter()
        .map(|s| interpret_segment_text(&s.text, s.interpret, lang))
        .collect()
}

fn interpret_segment_text(text: &str, interpret: ssml::SayAs, lang: &str) -> String {
    // These renderings are English-only (best-effort); other languages pass
    // through and read the raw content.
    if interpret == ssml::SayAs::Normal || primary_bcp47_subtag(lang) != "en" {
        return text.to_string();
    }
    match interpret {
        ssml::SayAs::Ordinal => ordinalize_english(text),
        ssml::SayAs::Date => dateize_english(text),
        ssml::SayAs::Time => timeize_english(text),
        ssml::SayAs::Normal => unreachable!(),
    }
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

/// Render an ISO `YYYY-MM-DD` (or `YYYY/MM/DD`) date as English words:
/// "2024-01-15" → "January 15th 2024" (read as "January fifteenth …").
/// Unrecognised input is returned unchanged (read normally).
fn dateize_english(text: &str) -> String {
    let parts: Vec<&str> = text.trim().split(['-', '/']).collect();
    if parts.len() == 3 && parts[0].len() == 4 {
        if let (Ok(y), Ok(m), Ok(d)) =
            (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>())
        {
            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                let ds = d.to_string();
                return format!("{} {d}{} {y}", MONTHS[(m - 1) as usize], english_ordinal_suffix(&ds));
            }
        }
    }
    text.to_string()
}

/// Render a `HH:MM` (or `HH:MM:SS`) time as English words: "14:30" → "14 30"
/// ("fourteen thirty"), "9:05" → "9 oh 5", "14:00" → "14 o'clock".
/// Unrecognised input is returned unchanged.
fn timeize_english(text: &str) -> String {
    let t = text.trim();
    if let Some((h, rest)) = t.split_once(':') {
        let m = rest.split(':').next().unwrap_or("");
        if let (Ok(hh), Ok(mm)) = (h.parse::<u32>(), m.parse::<u32>()) {
            if hh < 24 && mm < 60 {
                return match mm {
                    0 => format!("{hh} o'clock"),
                    1..=9 => format!("{hh} oh {mm}"),
                    _ => format!("{hh} {mm}"),
                };
            }
        }
    }
    text.to_string()
}

/// Append the English ordinal suffix to every integer token in `text`.
fn ordinalize_english(text: &str) -> String {
    text.split_whitespace()
        .map(|tok| {
            if !tok.is_empty() && tok.bytes().all(|b| b.is_ascii_digit()) {
                format!("{tok}{}", english_ordinal_suffix(tok))
            } else {
                tok.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The English ordinal suffix for a run of decimal digits (`st`/`nd`/`rd`/`th`).
fn english_ordinal_suffix(digits: &str) -> &'static str {
    let last_two = &digits[digits.len().saturating_sub(2)..];
    if matches!(last_two, "11" | "12" | "13") {
        return "th";
    }
    match digits.as_bytes().last() {
        Some(b'1') => "st",
        Some(b'2') => "nd",
        Some(b'3') => "rd",
        _ => "th",
    }
}

/// Technical symbols that eSpeak NG speaks as their name by default (i.e.
/// without `--punct`).  Everything else — `.,;:!?'"()[]{}` etc. — is treated
/// as pause/clause punctuation and produces no spoken output.
///
/// Determined empirically against the C reference (`x <sym> y`).
/// A word token that is exactly one alphabetic character — an acronym letter
/// ("U", "e").  A leading Lojban stress mark (`ˈ`) is ignored so caps-mark-stress
/// languages still qualify.
fn is_single_letter_word(w: &str) -> bool {
    let mut chars = w.chars().filter(|&c| c != '\u{02c8}');
    matches!(chars.next(), Some(c) if c.is_alphabetic()) && chars.next().is_none()
}

/// Whether the codes since the last clause boundary are nothing but the pauses
/// of a bracket, which belong to the word that follows.
fn only_pauses_since_clause_start(codes: &[PhonemeCode]) -> bool {
    let mut spoken = codes.iter().rev().filter(|c| c.marker.is_none());
    let mut pauses = 0;
    loop {
        match spoken.next() {
            Some(c) if c.code == crate::phoneme::PHON_PAUSE && !c.is_boundary => pauses += 1,
            Some(c) => return pauses > 0 && c.is_boundary && c.code == 0,
            None => return false,
        }
    }
}

/// A hyphen with a blank on each side (` - `) or doubled (`--`) is a dash that
/// pauses before the next word.
fn spaced_dash(tokens: &[Token], ix: usize) -> bool {
    let before = ix > 0 && matches!(tokens[ix - 1], Token::Space);
    let after = matches!(tokens.get(ix + 1), Some(Token::Space));
    before && after
}

fn word_tail(word: Option<&str>) -> [u8; 8] {
    let mut out = [0u8; 8];
    let Some(w) = word else { return out };
    let start = (w.len().saturating_sub(8)..=w.len()).find(|&k| w.is_char_boundary(k)).unwrap_or(w.len());
    out[..w.len() - start].copy_from_slice(&w.as_bytes()[start..]);
    out
}

fn hyphen_flags_at(tokens: &[Token], ix: usize) -> u32 {
    let joined_before = ix > 0 && matches!(tokens[ix - 1], Token::WordJoin);
    let joined_after = matches!(tokens.get(ix + 1), Some(Token::WordJoin));
    (if joined_before { crate::dictionary::FLAG_HYPHEN } else { 0 })
        | (if joined_after { crate::dictionary::FLAG_HYPHEN_AFTER } else { 0 })
}

fn is_dotted_letters(word: &str) -> bool {
    word.contains('.') && word.split('.').all(is_single_letter_word)
}

/// Whether a '.' followed by `after` stays inside the clause, as C's
/// `ReadClause` decides: a letter or hyphen follows, or blanks and then a
/// lower-case letter.
fn dot_stays_in_clause<I>(after: I) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    let mut after = after.peekable();
    match after.peek() {
        Some(&c) if c.is_alphabetic() || c == '-' => true,
        Some(c) if c.is_whitespace() => after.find(|c| !c.is_whitespace()).is_some_and(char::is_lowercase),
        _ => false,
    }
}

/// C reads `e.g.` as the words `e . g .`, which a multi-word list entry
/// (`(e . g .)`) can match; spell the dotted word out as those words when the
/// list has such an entry, and otherwise drop a stay-in-clause dot.
fn expand_dotted_phrases(tokens: &mut Vec<Token>, dict: &Dictionary, lang: &str) {
    let ctx = LookupCtx { dict_condition: dict.dict_condition, ..Default::default() };
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    for (ix, token) in tokens.iter().enumerate() {
        let Token::Word(word) = token else { out.push(token.clone()); continue };
        let core = word.trim_end_matches('.');
        if !is_dotted_letters(core) {
            out.push(token.clone());
            continue;
        }
        let pieces: Vec<String> = core.split('.').map(|p| lowercase_for_lang(p, lang)).collect();
        let mut follow: String = pieces[1..].iter().map(|p| format!(". {p} ")).collect();
        if word.ends_with('.') {
            follow.push_str(". ");
        }
        let rest = follow_text(tokens, ix + 1, lang);
        follow.push_str(&String::from_utf8_lossy(rest.split(|&b| b == 0).next().unwrap_or(&[])));
        let matched = lookup_with_follow(dict, &pieces[0], &ctx, follow.as_bytes())
            .is_some_and(|r| r.skipwords > 0 && !r.phonemes.is_empty());
        if !matched {
            out.push(Token::Word(core.to_string()));
            continue;
        }
        for (n, piece) in core.split('.').enumerate() {
            let dots = if n + 1 < pieces.len() || word.ends_with('.') { 1 } else { 0 };
            out.push(Token::Word(piece.to_string()));
            out.extend((0..dots).flat_map(|_| [Token::Space, Token::Word(".".into())]));
            out.push(Token::Space);
        }
    }
    *tokens = out;
}

fn is_spoken_symbol(c: char) -> bool {
    matches!(c, '\u{00BC}'..='\u{00BE}' | '\u{2150}'..='\u{215E}')
        || matches!(c, '.' | '#' | '$' | '%' | '&' | '*' | '+' | '/' | '=' | '@' | '~'
                | '×' | '÷' | '°' | '±' | '−' | '§' | '№'
                | '∞' | '√' | '∑' | '≈' | '≠' | '≤' | '≥'
                | '™' | '®' | '©'
                | '‰' | '‱')
}

/// Whether a digit directly follows a leading `-`, so the hyphen is a unary
/// minus (`-5`).  A currency symbol in between (`-$5`) is not: C drops the sign.
/// Takes a clone of the tokenizer iterator positioned just past the `-`.
fn minus_precedes_number<I>(chars: &std::iter::Peekable<I>) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    chars.clone().next().is_some_and(|d| d.is_ascii_digit())
}

/// Merge space-grouped thousands into a single number (`1 234 567` →
/// `1234567`) for languages that use the SI/ISO space separator
/// (`grammar.space_group`).  A run is a 1–3 digit leading group followed by one
/// or more ` <3 digits>` groups; anything that isn't exactly three digits ends
/// the run, so ordinary adjacent numbers are left alone.
fn apply_space_grouping(tokens: &mut Vec<Token>, grammar: &NumberGrammar) {
    if !grammar.space_group {
        return;
    }
    let is_digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Number(NumberToken::Cardinal(first)) = &tokens[i] {
            if first.len() <= 3 && is_digits(first) {
                // Greedily consume `<space> <exactly-3-digit cardinal>` groups.
                let mut merged = first.clone();
                let mut j = i + 1;
                let mut groups = 0;
                while matches!(tokens.get(j), Some(Token::Space | Token::WordJoin)) {
                    if let Some(Token::Number(NumberToken::Cardinal(g))) = tokens.get(j + 1) {
                        if g.len() == 3 && is_digits(g) {
                            merged.push_str(g);
                            j += 2;
                            groups += 1;
                            continue;
                        }
                    }
                    break;
                }
                // The final group may be a decimal (`1 234,56`): merge its
                // 3-digit integer part and keep the fraction on the whole number.
                let mut fraction: Option<String> = None;
                if matches!(tokens.get(j), Some(Token::Space | Token::WordJoin)) {
                    if let Some(Token::Number(NumberToken::Decimal { integer, fractional, .. })) =
                        tokens.get(j + 1)
                    {
                        if integer.len() == 3 && is_digits(integer) {
                            merged.push_str(integer);
                            fraction = Some(fractional.clone());
                            j += 2;
                            groups += 1;
                        }
                    }
                }
                if groups >= 1 {
                    let token = match fraction {
                        Some(fractional) => {
                            let grouped = merged.len() == MAX_UNSPLIT_DIGITS;
                            NumberToken::Decimal { integer: merged, fractional, grouped }
                        }
                        None => NumberToken::separated(merged),
                    };
                    out.push(Token::Number(token));
                    i = j;
                    continue;
                }
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// A writing system the port can route to a dedicated voice.
///
/// Mirrors the `alphabets[]` table in upstream `tr_languages.c`: a run of text
/// in a script the current voice does not read is handed to a voice that does
/// (upstream's `translator2` / "split a word where the script changes to a
/// foreign one"), instead of being dropped.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Script {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Armenian,
    Georgian,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhala,
    Thai,
    Myanmar,
    Ethiopic,
    Cherokee,
    Hangul,
    Kana,
    Han,
}

/// The script of a single character, or `None` for digits, punctuation, emoji
/// and scripts no shipped voice reads.
fn char_script(c: char) -> Option<Script> {
    let cp = c as u32;
    Some(match cp {
        0x0041..=0x005A | 0x0061..=0x007A => Script::Latin,
        // Latin-1 supplement letters, minus the two maths signs (× ÷) that sit
        // inside the letter ranges.
        0x00C0..=0x00FF if cp != 0x00D7 && cp != 0x00F7 => Script::Latin,
        0x0100..=0x024F | 0x1E00..=0x1EFF => Script::Latin, // Latin Extended-A/B, Additional
        0x0370..=0x03FF | 0x1F00..=0x1FFF => Script::Greek,
        0x0400..=0x052F => Script::Cyrillic,
        0x0530..=0x058F => Script::Armenian,
        0x0590..=0x05FF => Script::Hebrew,
        0x0600..=0x06FF | 0x0750..=0x077F | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => Script::Arabic,
        0x0900..=0x097F => Script::Devanagari,
        0x0980..=0x09FF => Script::Bengali,
        0x0A00..=0x0A7F => Script::Gurmukhi,
        0x0A80..=0x0AFF => Script::Gujarati,
        0x0B00..=0x0B7F => Script::Oriya,
        0x0B80..=0x0BFF => Script::Tamil,
        0x0C00..=0x0C7F => Script::Telugu,
        0x0C80..=0x0CFF => Script::Kannada,
        0x0D00..=0x0D7F => Script::Malayalam,
        0x0D80..=0x0DFF => Script::Sinhala,
        0x0E00..=0x0E7F => Script::Thai,
        0x1000..=0x109F => Script::Myanmar,
        0x10A0..=0x10FF | 0x2D00..=0x2D2F => Script::Georgian,
        0x1200..=0x137F => Script::Ethiopic,
        0x13A0..=0x13FF | 0xAB70..=0xABBF => Script::Cherokee,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => Script::Hangul,
        0x3040..=0x30FF | 0x31F0..=0x31FF => Script::Kana,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => Script::Han,
        0x20000..=0x2FA1F => Script::Han, // CJK Extension B-H + compatibility supplement
        _ => return None,
    })
}

/// Does upstream's `alphabets[]` table set `AL_WORDS` for this script — "use the
/// language to speak words"?
///
/// Only these scripts switch translator for a whole word.  Everything else
/// (Cyrillic, Hebrew, Thai, Han, Kana, …) is **spelled** letter by letter in the
/// current language: `en "хорошо"` reads out the Cyrillic letter names rather
/// than switching to Russian.  Taken from `tr_languages.c`, minus Greek: its row
/// also carries `AL_DONT_NAME | AL_NOT_LETTERS`, and upstream is observed to
/// spell Greek in an English voice (`αβγ` → `'alf@_b'i:t@_g'am@_`) rather than
/// switch, so it belongs with the spelled scripts here.
fn script_uses_words(script: Script) -> bool {
    matches!(
        script,
        Script::Armenian
            | Script::Arabic
            | Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Gujarati
            | Script::Tamil
            | Script::Kannada
            | Script::Malayalam
            | Script::Sinhala
            | Script::Georgian
            | Script::Hangul
    )
}

/// The default voice for a script — upstream picks the alternate translator from
/// the `alphabets[]` table's `language` field the same way.  (Upstream #2479:
/// the automatic switch target is hardcoded there too; here it can be overridden
/// per language by [`alt_alphabet_voice`].)
fn script_voice(script: Script) -> &'static str {
    match script {
        Script::Latin => "en",
        Script::Cyrillic => "ru",
        Script::Greek => "el",
        Script::Arabic => "ar",
        Script::Hebrew => "he",
        Script::Armenian => "hy",
        Script::Georgian => "ka",
        Script::Devanagari => "hi",
        Script::Bengali => "bn",
        Script::Gurmukhi => "pa",
        Script::Gujarati => "gu",
        Script::Oriya => "or",
        Script::Tamil => "ta",
        Script::Telugu => "te",
        Script::Kannada => "kn",
        Script::Malayalam => "ml",
        Script::Sinhala => "si",
        Script::Thai => "th",
        Script::Myanmar => "my",
        Script::Ethiopic => "am",
        Script::Cherokee => "chr",
        Script::Hangul => "ko",
        Script::Kana => "ja",
        Script::Han => "cmn",
    }
}

/// The scripts a voice reads natively.  Anything not listed reads Latin, which
/// matches the shipped voices (the large majority are Latin-script languages).
///
/// Japanese reads both kana and kanji; Korean reads hangul and hanja — hence a
/// slice rather than a single script.
fn native_scripts(lang: &str) -> &'static [Script] {
    match primary_bcp47_subtag(lang) {
        "ru" | "uk" | "bg" | "be" | "mk" | "mn" | "kk" | "ky" | "tt" | "ba"
        | "cv" | "tg" | "os" | "cu" | "ab" | "nog" => &[Script::Cyrillic],
        // Serbo-Croatian is digraphic, and all three voices share one
        // dictionary (C: `strcpy(tr->dictionary_name, "hbs")`) and declare
        // Cyrillic as `langopts.our_alphabet` — so `sr "Beograd"` ==
        // `sr "Београд"`, and `bs`/`hr` read the Cyrillic pangram as
        // Serbo-Croatian instead of spelling it out or switching to Russian.
        "sr" | "bs" | "hr" => &[Script::Cyrillic, Script::Latin],
        "el" | "grc" => &[Script::Greek],
        "ar" | "fa" | "ur" | "sd" | "ps" | "ug" => &[Script::Arabic],
        "he" => &[Script::Hebrew],
        "hy" | "hyw" => &[Script::Armenian],
        "ka" => &[Script::Georgian],
        "hi" | "mr" | "ne" | "sa" | "kok" | "mai" => &[Script::Devanagari],
        "bn" | "as" | "bpy" => &[Script::Bengali],
        "pa" => &[Script::Gurmukhi],
        "gu" => &[Script::Gujarati],
        "or" => &[Script::Oriya],
        "ta" => &[Script::Tamil],
        "te" => &[Script::Telugu],
        "kn" => &[Script::Kannada],
        "ml" => &[Script::Malayalam],
        "si" => &[Script::Sinhala],
        "th" => &[Script::Thai],
        "my" | "shn" => &[Script::Myanmar],
        "am" | "ti" => &[Script::Ethiopic],
        "chr" => &[Script::Cherokee],
        "ko" => &[Script::Hangul, Script::Han],
        "ja" => &[Script::Kana, Script::Han],
        "cmn" | "yue" | "hak" | "zh" => &[Script::Han],
        _ => &[Script::Latin],
    }
}

/// Per-language override of the fallback voice for a script — upstream's
/// `langopts.alt_alphabet` / `alt_alphabet_lang` (#2511).  Arabic nominates
/// Mandarin for Han ideographs so they are read rather than spelled out.
fn alt_alphabet_voice(lang: &str, script: Script) -> Option<&'static str> {
    match (primary_bcp47_subtag(lang), script) {
        ("ar", Script::Han) => Some("cmn"),
        _ => None,
    }
}

/// Whether the active dictionary has a spoken entry for `word` (used to keep a
/// listed word in the current language instead of switching by script).
fn dict_knows_word(word: &str, dict: &Dictionary) -> bool {
    let ctx = LookupCtx { lookup_symbol: true, ..Default::default() };
    matches!(lookup(dict, &word.to_lowercase(), &ctx), Some(r) if r.flags1.found())
}

/// Whether the rules abandon `word` at a letter they do not cover (C's
/// `FLAG_SPELLWORD`).
fn rules_abort_word(word: &str, dict: &Dictionary, lang_opts: &LangOptions) -> bool {
    if dict.rules().is_empty() {
        return false;
    }
    let word = lowercase_for_lang(word, &lang_opts.lang);
    let word = dict.apply_replacements(&word).unwrap_or(word);
    let word = decompose_hangul(&word).unwrap_or(word);
    let mut buf = Vec::with_capacity(word.len() + 3);
    buf.push(b' ');
    buf.extend_from_slice(word.as_bytes());
    buf.extend_from_slice(&[b' ', 0]);
    let (mut vowels, mut stressed) = (0i32, 0i32);
    crate::dictionary::rules::translate_rules(
        dict,
        &buf,
        1,
        0,
        0,
        &dict.letter_bits,
        lang_opts.dict_condition,
        &mut vowels,
        &mut stressed,
    )
    .aborted
}

/// Whether the dictionary list pronounces `word` (`Mª`, `kg`), seen with the
/// capitalisation the word was written in.
fn dict_lists_word(word: &str, dict: &Dictionary, options: &LangOptions) -> bool {
    let letters = || word.chars().filter(|c| c.is_alphabetic());
    let mut word_flags = 0;
    if word.chars().next().is_some_and(char::is_uppercase) {
        word_flags |= crate::dictionary::FLAG_FIRST_UPPER;
    }
    if letters().next().is_some() && letters().all(char::is_uppercase) {
        word_flags |= crate::dictionary::FLAG_ALL_UPPER;
    }
    let ctx = LookupCtx { word_flags, dict_condition: options.dict_condition, ..Default::default() };
    let lower = lowercase_for_lang(word, &options.lang);
    matches!(lookup(dict, &lower, &ctx), Some(r) if r.flags1.found())
}

/// C's `TranslateClause` redoes an abandoned word as separate one-letter words:
/// the rules in `dict` cannot read `ʃab`, so it becomes `ʃ`, `a`, `b`.  Returns,
/// for each resulting token, the index of the first letter of the word it came
/// from (`None` for every other token).
fn split_abandoned_words(
    tokens: &mut Vec<Token>,
    dict: &Dictionary,
    options: &LangOptions,
) -> Vec<Option<usize>> {
    let abandoned = |word: &str| {
        !word.is_ascii()
            && word.chars().count() > 1
            && !dict_lists_word(word, dict, options)
            && !is_unpronounceable(&lowercase_for_lang(word, &options.lang), &[], dict, options)
            && rules_abort_word(word, dict, options)
            // `TranslateWord3` spells an unpronounceable word before the
            // rules get to abandon it.
            && !is_unpronounceable(&lowercase_for_lang(word, &options.lang), &[], dict, options)
    };
    if !tokens.iter().any(|t| matches!(t, Token::Word(w) if abandoned(w))) {
        return vec![None; tokens.len()];
    }
    let mut out = Vec::with_capacity(tokens.len() + 4);
    let mut letters_at = Vec::with_capacity(tokens.len() + 4);
    for token in tokens.drain(..) {
        match token {
            Token::Word(word) if abandoned(&word) => {
                let first = out.len();
                let letters = word.chars().filter(|c| !matches!(*c as u32, 0x300..=0x36f));
                for (i, letter) in letters.enumerate() {
                    if i > 0 {
                        out.push(Token::Space);
                        letters_at.push(Some(first));
                    }
                    out.push(Token::Word(letter.to_string()));
                    letters_at.push(Some(first));
                }
            }
            other => {
                out.push(other);
                letters_at.push(None);
            }
        }
    }
    *tokens = out;
    letters_at
}

/// Whether the language's **own letter rules** can read this word.
///
/// C never switches translator by script — its `alphabets[]` table nominates a
/// language only for *naming* an unknown letter — and several voices carry
/// rules for a second script: Serbo-Croatian is digraphic, Macedonian and Uzbek
/// have Latin transliterations (and Uzbek a Cyrillic one), Lingua Franca Nova a
/// Cyrillic orthography.  A word those rules cover must therefore stay in the
/// language, instead of being routed to whatever voice owns the script:
/// `mk "Dzidarski"` is `dz'id&Rsk,I`, not `d'@ z'@ I_:…`, and `uz "Ўзбекча"` is
/// `o:zb'ektS&`, not Russian.
fn rules_know_word(word: &str, dict: &Dictionary, lang_opts: &LangOptions) -> bool {
    if word.is_empty() || dict.rules().is_empty() {
        return false;
    }
    // C reads the clause in lower case, so a capital never reaches the rules.
    let word = lowercase_for_lang(word, &lang_opts.lang);
    let mut buf = Vec::with_capacity(word.len() + 3);
    buf.push(b' ');
    buf.extend_from_slice(word.as_bytes());
    buf.push(b' ');
    buf.push(0);
    let (mut vowels, mut stressed) = (0i32, 0i32);
    let r = crate::dictionary::rules::translate_rules(
        dict,
        &buf,
        1,
        0,
        0,
        &dict.letter_bits,
        lang_opts.dict_condition,
        &mut vowels,
        &mut stressed,
    );
    // `spellword` means a letter had no rule at all — that is the case where the
    // word really is foreign to this language.
    !r.spellword && r.phonemes.iter().any(|&b| b != 0)
}

/// If `word` is written in a script the `lang` voice doesn't read (a Cyrillic or
/// Greek word inside an English clause, a Latin word inside a Russian one, Han
/// ideographs inside Arabic, …), return a voice that does, so the word is
/// re-translated instead of dropped.
fn foreign_script_voice(word: &str, lang: &str) -> Option<&'static str> {
    let script = word.chars().find_map(char_script)?;
    if native_scripts(lang).contains(&script) {
        return None; // the current voice already reads this script
    }
    // A language explicitly nominated by `alt_alphabet` wins over the AL_WORDS
    // rule (upstream checks `langopts.alt_alphabet` first).
    if let Some(alt) = alt_alphabet_voice(lang, script) {
        return Some(alt);
    }
    // Otherwise only an `AL_WORDS` script switches; the rest get spelled.
    script_uses_words(script).then(|| script_voice(script))
}

/// C's `Unpronouncable2`: no start-of-word rule matches the word, or the one
/// that does is marked `$unpron`.
fn rules_reject_word_start(rest: &str, next: &[u8], dict: &Dictionary, lang_opts: &LangOptions) -> bool {
    let word = rest.split(' ').next().unwrap_or("");
    let mut buf = Vec::with_capacity(word.len() + 12);
    buf.push(b' ');
    buf.extend_from_slice(word.as_bytes());
    buf.push(b' ');
    let next = next.split(|&b| b == 0).next().unwrap_or(&[]);
    if !next.is_empty() {
        buf.extend_from_slice(next);
        buf.push(b' ');
    }
    buf.push(0);
    let (mut vowels, mut stressed) = (0i32, 0i32);
    let r = crate::dictionary::rules::translate_rules(
        dict,
        &buf,
        1,
        crate::dictionary::FLAG_UNPRON_TEST,
        0,
        &dict.letter_bits,
        lang_opts.dict_condition,
        &mut vowels,
        &mut stressed,
    );
    r.end_type == 0 || r.end_type & crate::dictionary::SUFX_UNPRON != 0
}

/// C's `Unpronouncable`: is the first vowel too far into the word?
///
/// A word with more than `max_initial_consonants` consonants before its first
/// vowel (or none at all) is read letter by letter — that is how "BBC" becomes
/// `b,i:b,i:s'i:`.
fn is_unpronounceable(rest: &str, next: &[u8], dict: &Dictionary, lang_opts: &LangOptions) -> bool {
    // `LOPT_UNPRONOUNCABLE == 1` turns the test off for the language.
    if lang_opts.unpronouncable == 1 {
        return false;
    }
    let replaced = dict.apply_replacements(rest);
    let rest = replaced.as_deref().unwrap_or(rest);
    // C refuses the test for a character outside the language's own alphabet
    // ("so we can re-translate the word as English"), which keeps every
    // non-Latin script out of it.  The port applies the rule only to Latin text
    // in a Latin-script language, where a generic vowel test is reliable.
    if dict.letter_bits_offset > 0 {
        return false;
    }
    let in_other_alphabet = |c: char| {
        letters::alphabet_from_char(c as u32).is_some_and(|a| a.offset != dict.letter_bits_offset)
    };
    let is_vowel = |c: char| {
        let base = crate::dictionary::rules::fold_accent(c);
        matches!(base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
            || crate::dictionary::rules::is_letter_wc(
                &dict.letter_bits,
                c as u32,
                dict.letter_bits_offset,
                7, // LETTERGP_VOWEL2
            )
    };
    let Some(first) = rest.chars().next() else { return false };
    if in_other_alphabet(first) {
        return false;
    }
    let mut count = 0usize;
    let mut vowel_posn = 9usize;
    for c in rest.chars() {
        if c == ' ' {
            break;
        }
        if !c.is_alphabetic() && c != '\'' {
            return false; // not our alphabet
        }
        if c == '\'' && count > 1 {
            break;
        }
        count += 1;
        if is_vowel(c) {
            vowel_posn = count;
            break;
        }
    }

    // `LOPT_UNPRONOUNCABLE == 2` (English, German, Spanish, …): C hands the
    // decision to the language's own rules — a word no start-of-word rule can
    // read is spelled.  The port asks the narrower question "is there a vowel at
    // all?", which agrees with upstream on the words that matter ("BBC" and
    // "FBI" are spelled; "brown", "street" and German "schnell" are not) without
    // depending on every anchored rule matching exactly.
    if lang_opts.unpronouncable == 2 && vowel_posn > 2 {
        return rules_reject_word_start(rest, next, dict, lang_opts);
    }

    if lang_opts.unpronouncable > 3 && first as u32 == lang_opts.unpronouncable {
        vowel_posn = vowel_posn.saturating_sub(1);
    }
    vowel_posn > lang_opts.max_initial_consonants + 1
}

/// Spell the leading letters of an unpronounceable word, returning them and the
/// remainder still to translate — the `Unpronouncable` loop of `TranslateWord3`.
fn spell_unpronounceable<'a>(
    word: &'a str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
    next: &[u8],
) -> Option<(Vec<u8>, &'a str)> {
    if word.chars().count() < 2 {
        return None;
    }
    let mut out: Vec<u8> = Vec::new();
    let mut rest = word;
    let mut posn = 0usize;
    let mut length = 999usize;
    while (length < 3 && length > 0)
        || (word.chars().count() > 1 && is_unpronounceable(rest, next, dict, lang_opts_ref(options)))
    {
        let Some(c) = rest.chars().next() else { break };
        if c == '\'' {
            break;
        }
        let lower = c.to_lowercase().to_string();
        let expect = letter_expect(rest.len() > c.len_utf8());
        let mut wr = word_to_phonemes_pos(&format!("_{lower}"), dict, phdata, stress_opts, options, expect);
        if wr.phonemes.iter().all(|&b| b == 0) || wr.dict_flags == 0 {
            wr = letter_word_phonemes(&lower, dict, phdata, stress_opts, options, expect);
        }
        if wr.phonemes.iter().all(|&b| b == 0) {
            wr.phonemes = superscript_name(c, dict, phdata, stress_opts, options);
        }
        if wr.phonemes.iter().all(|&b| b == 0) {
            return None;
        }
        let name = if out.is_empty() {
            &wr.phonemes[..]
        } else {
            without_leading_break(&wr.phonemes, stress_opts)
        };
        out.extend(name.iter().copied().filter(|&b| b != 0));
        rest = &rest[c.len_utf8()..];
        posn += 1;
        length = rest.chars().count();
        if posn > 16 {
            break;
        }
    }
    if posn == 0 {
        return None;
    }
    set_spelling_stress(&mut out, stress_opts, posn);
    Some((out, rest))
}

/// C's `SetSpellingStress`: reduce the primary stresses of spelled letter names.
///
/// With `spelling_stress` only the first stays primary; otherwise the last does,
/// plus every third one.  A stress right after a language switch is left alone.
fn set_spelling_stress(out: &mut [u8], stress_opts: &StressOpts, n_chars: usize) {
    let is_counted = |ix: usize, b: u8| {
        b == PHON_STRESS_P && !(ix > 0 && out[ix - 1] == crate::phoneme::PHON_SWITCH)
    };
    let counted: Vec<usize> =
        (0..out.len()).filter(|&ix| is_counted(ix, out[ix])).collect();
    if n_chars <= 1 {
        return;
    }
    let n_stress = counted.len();
    for (ix, &at) in counted.iter().enumerate() {
        let count = ix + 1;
        let reduce = if stress_opts.spelling_stress {
            count > 1
        } else {
            count != n_stress && (count % 3 != 0 || count + 1 == n_stress)
        };
        if reduce {
            out[at] = PHON_STRESS_3;
        }
    }
}

/// Identity helper so `is_unpronounceable` can take the options by reference.
fn lang_opts_ref(o: &LangOptions) -> &LangOptions { o }

/// Keep the first primary stress of a prefix and demote the others to secondary.
fn reduce_later_primaries(prefix: &mut [u8]) {
    let mut seen = false;
    for b in prefix.iter_mut().filter(|b| **b == PHON_STRESS_P) {
        if seen {
            *b = PHON_STRESS_3;
        }
        seen = true;
    }
}

/// Drop the break that stress placement puts before a vowel-initial letter name.
///
/// C adds that break once for the whole spelled word, so only the first
/// letter keeps it.  Languages without the `vowel_pause & 0x30` break write
/// any leading pause in the letter name itself, which stays.
fn without_leading_break<'a>(phonemes: &'a [u8], stress_opts: &StressOpts) -> &'a [u8] {
    use crate::phoneme::{PHON_PAUSE_NOLINK, PHON_PAUSE_VSHORT};
    match phonemes.first() {
        Some(&c)
            if stress_opts.vowel_pause & 0x30 != 0
                && (c == PHON_PAUSE_NOLINK || c == PHON_PAUSE_VSHORT) =>
        {
            &phonemes[1..]
        }
        _ => phonemes,
    }
}

/// The language's own dictionary entry for `key`, unstressed, with its flags.
fn raw_list_entry(
    key: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Option<(Vec<u8>, u32)> {
    let expect = PosExpect { defer_stress: true, ..PosExpect::default() };
    let wr = word_to_phonemes_inner(key, dict, phdata, stress_opts, options, 0, expect);
    let ph: Vec<u8> = wr.phonemes.into_iter().filter(|&b| b != 0).collect();
    (wr.found_in_list && !ph.is_empty()).then_some((ph, wr.dict_flags))
}

/// C's `LookupLetter2`: the name of a base letter, from `_x`, then `x`, then
/// the letter-to-sound rules.
fn base_letter_name(
    letter: u32,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Vec<u8> {
    let Some(c) = char::from_u32(letter) else { return Vec::new() };
    let key = c.to_string();
    let named = raw_list_entry(&format!("_{key}"), dict, phdata, stress_opts, options)
        .or_else(|| raw_list_entry(&key, dict, phdata, stress_opts, options));
    match named {
        Some((ph, _)) => ph,
        None => {
            let expect = PosExpect { defer_stress: true, ..PosExpect::default() };
            word_to_phonemes_inner(&key, dict, phdata, stress_opts, options, 0, expect)
                .phonemes
                .into_iter()
                .filter(|&b| b != 0)
                .collect()
        }
    }
}

/// C's `LookupAccentedLetter`: the name of an accented letter, ligature or
/// IPA letter built from its base letter's name and its accent's name.
/// Empty when the letter has no table entry or the language lacks a name.
fn accented_letter_phonemes(
    letter: char,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Vec<u8> {
    use crate::phoneme::{PHON_PAUSE_VSHORT, PHON_STRESS_2, PHON_STRESS_P};
    const FLAG_ACCENT_BEFORE: u32 = 0x1000;
    let Some(spec) = accent::accent_spec(letter as u32) else { return Vec::new() };
    let entry = |key: &str| raw_list_entry(key, dict, phdata, stress_opts, options);
    let Some((accent1, flags1)) = entry(spec.accent1.0) else { return Vec::new() };
    let first = base_letter_name(spec.basic, dict, phdata, stress_opts, options);
    if first.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<u8> = Vec::new();
    let mut accent2: Vec<u8> = Vec::new();
    if let Some((ph, flags2)) = spec.accent2.and_then(entry) {
        if flags2 & FLAG_ACCENT_BEFORE != 0 {
            out.extend(ph);
        } else {
            accent2 = ph;
        }
    }
    if let Some(second) = spec.second {
        out.extend(accent1);
        out.push(PHON_PAUSE_VSHORT);
        out.extend(first);
        out.push(PHON_STRESS_P);
        out.extend(base_letter_name(second, dict, phdata, stress_opts, options));
        out.extend(accent2);
    } else if accents_say_first(&options.lang) || flags1 & FLAG_ACCENT_BEFORE != 0 || spec.accent1.1 {
        out.extend(accent1);
        out.extend([PHON_PAUSE_VSHORT, PHON_STRESS_P]);
        out.extend(first);
    } else {
        out.push(PHON_STRESS_2);
        out.extend(first);
        out.push(PHON_PAUSE_VSHORT);
        out.extend(accent1);
        out.push(PHON_PAUSE_VSHORT);
    }
    out
}

/// A letter's name taken from the entry for the letter as a word, as `LookupLetter` does.
///
/// The stress is placed with no dictionary flags, so `$u+` on French `a` does
/// not keep the spelled letter unstressed.
fn letter_word_phonemes(
    lower: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
    expect: PosExpect,
) -> WordResult {
    let listed = lookup_with_follow(dict, lower, &LookupCtx::default(), &[]).filter(|r| {
        r.flags1.found()
            && r.flags1.stress_bits() != 0
            && !r.flags1.textmode()
            && !r.flags2.contains(crate::dictionary::FLAG_ACCENT)
            && r.phonemes.first().is_some_and(|&b| b != 0 && b != crate::phoneme::PHON_SWITCH)
    });
    let Some(r) = listed else {
        return word_to_phonemes_pos(lower, dict, phdata, stress_opts, options, expect);
    };
    let mut phonemes: Vec<u8> = r.phonemes.iter().copied().take_while(|&b| b != 0).collect();
    phonemes.push(0);
    set_word_stress(&mut phonemes, phdata, stress_opts, Some(0), -1, 0);
    WordResult { phonemes, dict_flags: r.flags1.0, found_in_list: true, prefix: None, skip_words: 0, late_suffix: Vec::new() }
}

/// `langopts.accents & 1`: Afrikaans, Croatian, Bosnian, Serbian and Swedish
/// say the accent name before the letter name.
fn accents_say_first(lang: &str) -> bool {
    matches!(primary_bcp47_subtag(lang), "af" | "hr" | "bs" | "sr" | "sv")
}

/// Spell a word as its letters' *names*, run together as one word — C's
/// `SpeakIndividualLetters` with `spell_word == 1`.
///
/// `LookupLetter2` asks for the name (`_a`) before the letter-as-a-word, so "a"
/// is "ay" rather than the article, and `SetSpellingStress` then demotes every
/// primary stress but the last: English `usa` reads `j,u:,Es'eI`.
fn spell_word_letters(
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    let replaced = dict.apply_replacements(word);
    let chars: Vec<char> = replaced.as_deref().unwrap_or(word).chars().collect();
    for (ix, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            continue;
        }
        let lower = c.to_lowercase().to_string();
        let expect = letter_expect(ix + 1 < chars.len());
        let mut wr = word_to_phonemes_pos(&format!("_{lower}"), dict, phdata, stress_opts, options, expect);
        if wr.phonemes.iter().all(|&b| b == 0) || wr.dict_flags == 0 {
            wr = letter_word_phonemes(&lower, dict, phdata, stress_opts, options, expect);
        }
        if wr.phonemes.iter().all(|&b| b == 0) {
            wr.phonemes = superscript_name(c, dict, phdata, stress_opts, options);
        }
        if wr.phonemes.iter().all(|&b| b == 0) {
            return None;
        }
        let name = if out.is_empty() {
            &wr.phonemes[..]
        } else {
            without_leading_break(&wr.phonemes, stress_opts)
        };
        out.extend(name.iter().copied().filter(|&b| b != 0));
    }
    if out.is_empty() {
        return None;
    }
    set_spelling_stress(&mut out, stress_opts, chars.iter().filter(|c| c.is_alphanumeric()).count());
    Some(out)
}

/// The dictionary key of the name `TranslateLetter` announces for a script
/// (`alphabets[].name`); Greek is `AL_DONT_NAME`.
fn alphabet_name_key(script: Script) -> Option<&'static str> {
    Some(match script {
        Script::Armenian => "_hy",
        Script::Hebrew => "_he",
        Script::Arabic => "_ar",
        Script::Devanagari => "_hi",
        Script::Bengali => "_bn",
        Script::Gurmukhi => "_gur",
        Script::Gujarati => "_gu",
        Script::Oriya => "_or",
        Script::Tamil => "_ta",
        Script::Telugu => "_te",
        Script::Kannada => "_kn",
        Script::Malayalam => "_ml",
        Script::Sinhala => "_si",
        Script::Thai => "_th",
        Script::Myanmar => "_my",
        Script::Georgian => "_ka",
        Script::Ethiopic => "_eth",
        Script::Kana => "_ja",
        Script::Han => "_zh",
        _ => return None,
    })
}

/// As [`alphabet_name_key`], plus Cyrillic: a letter handed to another voice
/// (`it м` → `_^_RU`) is announced as "cirillico", whereas a Cyrillic letter the
/// dictionary lists as a word (`en х`) never reaches `TranslateLetter`.
fn switched_alphabet_name_key(script: Script) -> Option<&'static str> {
    match script {
        Script::Cyrillic => Some("_cyr"),
        other => alphabet_name_key(other),
    }
}

/// Spell a word the current voice can't read: one letter name per character,
/// looked up in this language's own dictionary (`х` → `x'a:` in English).
///
/// This is upstream's `FLAG_SPELLWORD` path — an unrecognised letter inside a
/// word aborts the translation and the word is spelled instead.  Returns `None`
/// when no character has a name, so the caller can fall back.
fn spell_foreign_word(
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for c in word.chars() {
        // Combining marks have no name of their own.
        if matches!(c as u32, 0x0300..=0x036F) {
            continue;
        }
        // Letter names are listed lower-case (`_х`), so a capital spells the
        // same as its lower-case form.
        let lower = c.to_lowercase().to_string();
        let mut wr = word_to_phonemes(&format!("_{lower}"), dict, phdata, stress_opts, options);
        if wr.phonemes.iter().all(|&b| b == 0) || !wr.found_in_list {
            wr = word_to_phonemes(&lower, dict, phdata, stress_opts, options);
        }
        // A name has to come from the *dictionary* (`_х` → "kha").  Letter-to-
        // sound rules will happily produce phonemes for any character, which is
        // not a name — if even one letter has none, the language can't spell
        // this word and the caller falls back to switching voice.
        if wr.phonemes.is_empty() || !wr.found_in_list {
            return None;
        }
        // `TranslateLetter` says the alphabet's name (`_he`) ahead of every
        // letter it spells from another script.
        let alphabet = char_script(c)
            .and_then(alphabet_name_key)
            .and_then(|key| raw_list_entry(key, dict, phdata, stress_opts, options))
            .map(|(ph, _)| ph);
        // Each spelled letter is its own word (`x'a: 'o: 'Er …`) — unless the
        // name already ends in a pause, as the Greek letter names do
        // (`'alf@_b'i:t@_`), where upstream runs them together.
        if !out.is_empty() && (alphabet.is_some() || !out.last().is_some_and(|&c| is_pause_code(c))) {
            out.push(crate::phoneme::PHON_END_WORD);
        }
        if let Some(name) = alphabet {
            out.extend(name);
        }
        // Strip any 0 terminator: it ends the *whole* sequence for the IPA
        // renderer, which would truncate the word after its first letter.
        out.extend(wr.phonemes.iter().copied().filter(|&b| b != 0));
    }
    (!out.is_empty()).then_some(out)
}

/// C's `derived_letters` (`IsSuperscript`): the base character of a super- or
/// subscript letter or digit.
fn derived_letter(c: char) -> Option<char> {
    let cp = c as u32;
    let digit = |zero: u32| char::from_digit(cp - zero, 10);
    match cp {
        0xaa => Some('a'),
        0xba => Some('o'),
        0xb2 => Some('2'),
        0xb3 => Some('3'),
        0xb9 => Some('1'),
        0x2b0 | 0x2095 => Some('h'),
        0x2b1 => Some('\u{266}'),
        0x2b2 => Some('j'),
        0x2b3 => Some('r'),
        0x2b4 => Some('\u{279}'),
        0x2b5 => Some('\u{27b}'),
        0x2b6 => Some('\u{281}'),
        0x2b7 => Some('w'),
        0x2b8 => Some('y'),
        0x2c0 => Some('\u{294}'),
        0x2c1 => Some('\u{295}'),
        0x2e0 => Some('\u{263}'),
        0x2e1 | 0x2097 => Some('l'),
        0x2e2 | 0x209b => Some('s'),
        0x2e3 | 0x2093 => Some('x'),
        0x2070 => Some('0'),
        0x2071 => Some('i'),
        0x2074..=0x2079 => digit(0x2070),
        0x2080..=0x2089 => digit(0x2080),
        0x207a | 0x208a => Some('+'),
        0x207b | 0x208b => Some('-'),
        0x207c | 0x208c => Some('='),
        0x207d | 0x208d => Some('('),
        0x207e | 0x208e => Some(')'),
        0x207f | 0x2099 => Some('n'),
        0x2090 => Some('a'),
        0x2091 => Some('e'),
        0x2092 => Some('o'),
        0x2094 => Some('\u{259}'),
        0x2096 => Some('k'),
        0x2098 => Some('m'),
        0x209a => Some('p'),
        0x209c => Some('t'),
        _ => None,
    }
}

/// C's `LookupLetter`: the name of one letter in this language, from `_x`, then
/// `x`, then the letter-to-sound rules, then the accent tables.  Stressed, no
/// terminator; a result that starts with `phonSWITCH` hands the letter to
/// another language.
fn letter_name(
    letter: char,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Vec<u8> {
    let key = letter.to_string();
    let spoken = |wr: &WordResult| wr.phonemes.iter().any(|&b| b != 0);
    // C stresses a letter name afresh (`SetWordStress` with no flags), so a
    // function word's `$u` does not make `a` unstressed.
    let word = |text: &str| {
        let expect = PosExpect { no_replace: true, defer_stress: true, ..PosExpect::default() };
        word_to_phonemes_inner(text, dict, phdata, stress_opts, options, 0, expect)
    };
    let mut wr = word(&format!("_{key}"));
    if !spoken(&wr) || !wr.found_in_list {
        wr = word(&key);
    }
    if spoken(&wr) {
        let mut name = wr.phonemes;
        if name.first() != Some(&crate::phoneme::PHON_SWITCH) {
            if name.last() != Some(&0) {
                name.push(0);
            }
            set_word_stress(&mut name, phdata, stress_opts, Some(0), -1, 0);
        }
        name.retain(|&b| b != 0);
        return name;
    }
    let mut accented = accented_letter_phonemes(letter, dict, phdata, stress_opts, options);
    if accented.is_empty() {
        return accented;
    }
    accented.push(0);
    set_word_stress(&mut accented, phdata, stress_opts, Some(0), -1, 0);
    accented.retain(|&b| b != 0);
    accented
}

/// The name of the base letter of a super- or subscript letter (`ª` is `a`),
/// with a terminating NUL; empty when `c` is not one or the base has no name.
fn superscript_name(
    c: char,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
) -> Vec<u8> {
    let mut name = c
        .to_lowercase()
        .next()
        .and_then(derived_letter)
        .map(|base| letter_name(base, dict, phdata, stress_opts, options))
        .unwrap_or_default();
    if !name.is_empty() {
        name.push(0);
    }
    name
}

/// How `LetterFallback` says a letter the language cannot pronounce.
pub(crate) struct LetterEntry {
    pub(crate) phonemes: Vec<u8>,
    pub(crate) word: String,
}

/// C's `TranslateLetter` for a lone letter the rules left silent.
///
/// The letter's own name first (a super- or subscript letter by its base
/// letter's name), then — for a voice that is not English — the English name,
/// which `english_has` vouches for, and last the word "letter" followed by the
/// digits of the letter's code point.
fn letter_fallback(
    letter: char,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
    english_has: &dyn Fn(char) -> bool,
) -> Option<LetterEntry> {
    use crate::phoneme::{PHON_PAUSE, PHON_PAUSE_VSHORT, PHON_SWITCH};
    let (mut letter, mut name) = (letter, Vec::new());
    for candidate in std::iter::once(letter).chain(derived_letter(letter)) {
        letter = candidate;
        name = letter_name(letter, dict, phdata, stress_opts, options);
        if !name.is_empty() {
            break;
        }
    }
    let entry = |mut phonemes: Vec<u8>, word: String| {
        phonemes.push(0);
        Some(LetterEntry { phonemes, word })
    };
    if !name.is_empty() {
        return entry(name, letter.to_string());
    }
    if primary_bcp47_subtag(&options.lang) != "en" && english_has(letter) {
        let mut switch = vec![PHON_SWITCH];
        switch.extend_from_slice(b"en");
        return entry(switch, format!("_{letter}"));
    }
    // C's `alphabets[]` hands 0xa700..0xd7ff to Korean, which has no name and
    // does not speak a code.
    if (0xa700..=0xd7ff).contains(&(letter as u32)) {
        return None;
    }
    let own = |key: &str| raw_list_entry(key, dict, phdata, stress_opts, options).map(|(ph, _)| ph);
    let mut out = Some(letter)
        .filter(|c| c.is_alphabetic())
        .and_then(|_| own("_?A"))
        .or_else(|| (!letter.is_whitespace()).then(|| own("_??")).flatten())
        .unwrap_or_else(|| parse_inline_phonemes("l'et@", phdata));
    const HEX_NAMES: [&str; 6] = ["'e:j", "b'i:", "s'i:", "d'i:", "'i:", "'ef"];
    for digit in format!("{:x}", letter as u32).chars() {
        out.push(PHON_PAUSE_VSHORT);
        let mut spelled = letter_name(digit, dict, phdata, stress_opts, options);
        if spelled.first().is_none_or(|&b| b == PHON_SWITCH) && digit >= 'a' {
            spelled = parse_inline_phonemes(HEX_NAMES[(digit as u8 - b'a') as usize], phdata);
        }
        out.extend(spelled);
    }
    out.push(PHON_PAUSE);
    entry(out, letter.to_string())
}

/// C's `TranslateRoman` validity scan over a lower-case word: `ix`, `xxi` and
/// `mcm`, but not `iiii`, `vx` or `ic`.  Returns the value when it lies within
/// the language's `min_roman..=max_roman` (2..=49 by default).
fn c_roman_value(word: &str, bounds: (i32, i32)) -> Option<u32> {
    const LETTERS: &str = "ixcmvld";
    const VALUES: [i32; 7] = [1, 10, 100, 1000, 5, 50, 500];
    if word.chars().count() < 2 {
        return None;
    }
    let (mut acc, mut prev, mut subtract, mut repeat) = (0i32, 0i32, 0x7fffi32, 0);
    for c in word.chars() {
        let mut value = VALUES[LETTERS.find(c)?];
        if value == prev {
            repeat += 1;
            if repeat >= 3 {
                return None;
            }
        } else {
            repeat = 0;
        }
        if prev > 1 && prev != 10 && prev != 100 && value >= prev {
            return None;
        }
        if prev != 0 && prev < value {
            if acc % 10 != 0 || prev * 10 < value {
                return None;
            }
            subtract = prev;
            value -= subtract;
        } else if value >= subtract {
            return None;
        } else {
            acc += prev;
        }
        prev = value;
    }
    acc += prev;
    (bounds.0..=bounds.1).contains(&acc).then_some(acc as u32)
}

/// C's `min_roman` and `max_roman` for the language.
fn roman_bounds(lang: &str) -> (i32, i32) {
    match primary_bcp47_subtag(lang) {
        "hu" => (1, 899),
        "la" | "lt" | "pt" => (2, 5000),
        _ => (2, 49),
    }
}

/// Whether C's `TranslateWord3` tries `TranslateRoman` on a word of this case.
fn roman_numerals_apply(numbers: u32, all_upper: bool) -> bool {
    const NUM_ROMAN: u32 = 0x0100_0000;
    const NUM_ROMAN_CAPITALS: u32 = 0x0200_0000;
    const NUM_ROMAN_ORDINAL: u32 = 0x0800_0000;
    numbers & NUM_ROMAN_ORDINAL == 0
        && numbers & (NUM_ROMAN | NUM_ROMAN_CAPITALS) != 0
        && (numbers & NUM_ROMAN_CAPITALS == 0 || all_upper)
}

/// A Roman numeral read as "roman" plus its number (`vi` is "roman six"), for
/// a language with `NUM_ROMAN` and no capitals-only rule.
fn roman_number_phonemes(
    word: &str,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    grammar: &NumberGrammar,
) -> Option<Vec<u8>> {
    const NUM_ROMAN_AFTER: u32 = 0x0400_0000;
    let value = c_roman_value(word, roman_bounds(&dict.lang))?;
    let number = translate_number_token(
        &NumberToken::Cardinal(value.to_string()), dict, phdata, stress_opts, grammar, &|_| Vec::new(),
    )?;
    let roman = trim_lookup(&lookup_num_phonemes(dict, "_roman")).to_vec();
    let digits = number.phonemes.iter().copied().filter(|&b| b != 0);
    let mut out = if grammar.c_numbers & NUM_ROMAN_AFTER != 0 {
        digits.chain(roman).collect()
    } else {
        roman.into_iter().chain(digits).collect::<Vec<u8>>()
    };
    out.push(0);
    set_word_stress(&mut out, phdata, stress_opts, Some(0), -1, 0);
    Some(out)
}

/// Whether the language treats `ordinal` as an ordinal: C accepts a suffix only
/// when the dictionary has `_#<suffix>` (or it is the language's own indicator).
fn ordinal_is_recognised(ordinal: &OrdinalNumber, dict: &Dictionary, grammar: &NumberGrammar) -> bool {
    match &ordinal.marker {
        OrdinalMarker::Dot => true,
        OrdinalMarker::Suffix(suffix) => {
            grammar.ordinals.indicator.as_deref() == Some(suffix)
                || !lookup_num_phonemes(dict, &format!("_#{suffix}")).is_empty()
        }
    }
}

/// A number glued to a suffix the language has no ordinal entry for is the number
/// followed by the suffix as a word ("1st" in es is "uno" + the letters "st").
fn split_unrecognised_ordinals(tokens: &mut Vec<Token>, dict: &Dictionary, grammar: &NumberGrammar) {
    let split = |token: Token| match token {
        Token::Number(NumberToken::Ordinal(ord))
            if !ordinal_is_recognised(&ord, dict, grammar) =>
        {
            let OrdinalMarker::Suffix(suffix) = ord.marker else { unreachable!("dot ordinals are recognised") };
            vec![Token::Number(NumberToken::Cardinal(ord.digits)), Token::Word(suffix)]
        }
        other => vec![other],
    };
    *tokens = std::mem::take(tokens).into_iter().flat_map(split).collect();
}

/// Join a cardinal and the word glued to it into one ordinal-style token when
/// the dictionary has a multi-word entry for them (fr `1er`, `21ème`); the
/// number then consumes the word, as C's `FLAG_SKIPWORDS` does.
fn merge_digit_follow(tokens: &mut Vec<Token>, dict: &Dictionary) {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut iter = std::mem::take(tokens).into_iter().peekable();
    while let Some(token) = iter.next() {
        if let (Token::Number(NumberToken::Cardinal(digits)), Some(Token::Word(word))) = (&token, iter.peek()) {
            if lookup_digit_follow(dict, digits, word).is_some() {
                let marker = OrdinalMarker::Suffix(word.clone());
                let merged = NumberToken::Ordinal(OrdinalNumber { digits: digits.clone(), marker });
                iter.next();
                out.push(Token::Number(merged));
                continue;
            }
        }
        out.push(token);
    }
    *tokens = out;
}

/// Downgrade a clause-ending period that is not followed by a capitalised
/// word to a plain word break, and a comma between two numbers to a space.
fn suppress_abbreviation_periods(tokens: &mut [Token]) {
    // A period only ends a sentence when a capitalised word (or nothing)
    // follows — "one. two" is not a break, "one. Two" is.  Upstream's `-x`
    // shows exactly this: `w'0n t'u:` on one line versus `w'0n` / `t'u:` on
    // two.  Language-independent, like the clause tokenizer in C.
    for i in 0..tokens.len() {
        if !matches!(tokens[i], Token::ClauseBoundary('.')) {
            continue;
        }
        let next_word = tokens[i + 1..].iter().find(|t| {
            !matches!(t, Token::Space | Token::WordJoin) && !matches!(t, Token::ClauseBoundary(_))
        });
        let ends_sentence = match next_word {
            None => true,
            Some(Token::Word(w)) => w.chars().next().is_some_and(|c| !c.is_lowercase()),
            // A number or symbol after the period keeps the sentence break.
            Some(_) => true,
        };
        if !ends_sentence {
            tokens[i] = Token::Space;
        }
    }

    // A comma directly between two numbers is a digit separator, not a clause
    // break: `en "3,14"` reads "three fourteen" on one line upstream, where the
    // port used to add a clause break as well as the space.
    for i in 1..tokens.len().saturating_sub(1) {
        if matches!(tokens[i], Token::ClauseBoundary(','))
            && matches!(tokens[i - 1], Token::Number(_))
            && matches!(tokens[i + 1], Token::Number(_))
        {
            tokens[i] = Token::Space;
        }
    }

}

/// Read a `H:MM` (or `H:MM:SS`) time.  The colon is a time separator, not a
/// clause boundary: it is silent, the hour drops any leading zero (`02:30` →
/// "two thirty", `00:30` → "zero thirty"), and languages with a spoken connector
/// insert it between hour and minute for a valid hour (0–23) — German "uhr",
/// French "heures", Italian "e" (`10:30` → "zehn uhr dreißig" / "dix heures
/// trente" / "dieci e trenta").  A field must be *exactly* two digits, so `10:3`
/// and `10:300` are not times (their `:` stays a boundary).  The minute keeps its
/// leading zero (`10:05` → "…zero five"), matching espeak-ng.
fn apply_time_reading(tokens: &mut Vec<Token>, lang: &str) {
    let connector = match primary_bcp47_subtag(lang) {
        "de" => Some("uhr"),
        "fr" => Some("heures"),
        _ => None,
    };
    let is_colon = |t: Option<&Token>| matches!(t, Some(Token::Punctuation(':')));
    let read_colon = primary_bcp47_subtag(lang) == "en";
    // A cardinal token whose surface is exactly two digits (a minute/second).
    let two_digit = |t: Option<&Token>| -> Option<String> {
        match t {
            Some(Token::Number(NumberToken::Cardinal(s)))
                if s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit()) =>
            {
                Some(s.clone())
            }
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        // Hour: a run of digits immediately followed by ':' + two-digit minutes.
        let hour = match &tokens[i] {
            Token::Number(NumberToken::Cardinal(h)) if h.bytes().all(|b| b.is_ascii_digit()) => {
                Some(h.clone())
            }
            _ => None,
        };
        if let Some(h) = hour {
            if is_colon(tokens.get(i + 1)) {
                if let Some(minute) = two_digit(tokens.get(i + 2)) {
                    // Hour with leading zeros stripped, but never to nothing.
                    let h_trim = h.trim_start_matches('0');
                    let h_norm = if h_trim.is_empty() { "0" } else { h_trim };
                    let h_val: u32 = h.parse().unwrap_or(u32::MAX);
                    out.push(Token::Number(NumberToken::Cardinal(h_norm.to_string())));
                    out.push(Token::Space);
                    if let Some(word) = connector {
                        if h_val <= 23 {
                            out.push(Token::Word(word.to_string()));
                            out.push(Token::Space);
                        }
                    } else if primary_bcp47_subtag(lang) == "it" && h_val <= 23 {
                        // The Italian rule `_L16_) : (_L17D_` gives the colon a
                        // paused, diminished "e" of its own rather than the word.
                        out.push(Token::InlinePhonemes("_%%E_".to_string()));
                        out.push(Token::Space);
                    }
                    out.push(Token::Number(NumberToken::Cardinal(minute)));
                    let mut j = i + 3;
                    // Optional ":SS" seconds — colon silent, no connector.
                    if is_colon(tokens.get(j)) {
                        if let Some(sec) = two_digit(tokens.get(j + 1)) {
                            out.push(Token::Space);
                            out.push(Token::Number(NumberToken::Cardinal(sec)));
                            j += 2;
                        }
                    }
                    i = j;
                    continue;
                }
            }
        }
        // A ':' that isn't a valid time (a ratio like "3:2"): English reads it as
        // "colon", every other language drops it — neither breaks the clause.
        if matches!(&tokens[i], Token::Punctuation(':')) {
            if read_colon {
                out.push(Token::Space);
                out.push(Token::Word("colon".to_string()));
                out.push(Token::Space);
            }
            i += 1;
            continue;
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// A hyphen-minus `-` directly between two numbers is read as the "-" symbol,
/// which espeak-ng voices only for some languages: English "dash", German
/// "Strich" (`5-10` → "five dash ten" / "fünf strich zehn", `3-4-5` → "three
/// dash four dash five").  Every other language drops it (the port already did
/// for all), so this is purely additive.  Only fires between two `Number`s — a
/// letter on either side (`x-5`, `5-x`) or a word-joining hyphen ("well-known",
/// handled in the tokenizer) is untouched, and en/em dashes are not `-`.
fn apply_dash_reading(tokens: &mut Vec<Token>, lang: &str) {
    let dash = match primary_bcp47_subtag(lang) {
        "en" => "dash",
        "de" => "Strich",
        _ => return,
    };
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if matches!(&tokens[i], Token::Number(_))
            && matches!(tokens.get(i + 1), Some(Token::Punctuation('-')))
            && matches!(tokens.get(i + 2), Some(Token::Number(_)))
        {
            out.push(tokens[i].clone()); // the first number
            out.push(Token::Space);
            out.push(Token::Word(dash.to_string()));
            out.push(Token::Space);
            // Leave the second number for the next iteration so a chain
            // ("3-4-5") keeps matching.
            i += 2;
            continue;
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// A lowercase `x` between two numbers is a dimension separator, read "by":
/// `8x10` → "eight by ten", `1920x1080` → "…by…".  English-only; requires a
/// number on both sides so algebra (`2x`) and multipliers (`3x faster`) — which
/// have no number after the `x` — are left as the letter.
fn apply_dimensions(tokens: &mut [Token], lang: &str) {
    if primary_bcp47_subtag(lang) != "en" {
        return;
    }
    // A real dimension has a non-zero number on each side; a `0` side means it's
    // something else (a hex literal like `0x1F` → "0","x","1","F").
    let is_num = |t: Option<&Token>| {
        matches!(t, Some(Token::Number(NumberToken::Cardinal(n))) if n != "0")
            || matches!(t, Some(Token::Number(NumberToken::Decimal { .. })))
    };
    for i in 0..tokens.len() {
        if !matches!(&tokens[i], Token::Word(w) if w == "x") {
            continue;
        }
        let prev = match (i, tokens.get(i.wrapping_sub(1))) {
            (0, _) => None,
            (_, Some(Token::Space | Token::WordJoin)) => tokens.get(i.wrapping_sub(2)),
            (_, t) => t,
        };
        let next = match tokens.get(i + 1) {
            Some(Token::Space | Token::WordJoin) => tokens.get(i + 2),
            t => t,
        };
        if is_num(prev) && is_num(next) {
            tokens[i] = Token::Word("by".to_string());
        }
    }
}

/// English decades: a bare "s" right after a round-ten number is a spoken decade,
/// not a letter — `1980s` → "nineteen eighties", `80s`/`'80s` → "eighties",
/// `1900s` → "nineteen hundreds".  English-only.
fn apply_decades(tokens: &mut Vec<Token>, lang: &str) {
    if primary_bcp47_subtag(lang) != "en" {
        return;
    }
    // A trailing plural "s", allowing a possessive apostrophe ("1980s",
    // "1980's", "1980’s") — the "s"/"'s" may be one token or a separate `'`.
    let is_s = |t: Option<&Token>| {
        matches!(t, Some(Token::Word(w)) if w.trim_start_matches(['\'', '’']).eq_ignore_ascii_case("s"))
    };
    let apostrophe = |t: Option<&Token>| matches!(t, Some(Token::Punctuation('\'' | '’')));

    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Number(NumberToken::Cardinal(digits)) = &tokens[i] {
            let consumed = if is_s(tokens.get(i + 1)) {
                2
            } else if apostrophe(tokens.get(i + 1)) && is_s(tokens.get(i + 2)) {
                3
            } else {
                0
            };
            if consumed > 0 {
                if let Some(decade) = decade_reading(digits) {
                    out.extend(decade);
                    i += consumed;
                    continue;
                }
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// The spoken tokens for a written decade `<digits>s`, or `None` if it isn't one
/// (not a round ten, or outside the handled range).  The century of a 4-digit
/// year reads as a number ("nineteen"); the decade pluralises ("eighties").
fn decade_reading(digits: &str) -> Option<Vec<Token>> {
    let n: u32 = digits.parse().ok()?;
    if n % 10 != 0 {
        return None;
    }
    let tens_plural = |t: u32| -> Option<&'static str> {
        Some(match t {
            1 => "tens", 2 => "twenties", 3 => "thirties", 4 => "forties",
            5 => "fifties", 6 => "sixties", 7 => "seventies", 8 => "eighties", 9 => "nineties",
            _ => return None,
        })
    };
    // Bare two-digit decade ("80s" → "eighties", "'90s" → "nineties").
    if (10..=90).contains(&n) {
        return Some(vec![Token::Word(tens_plural(n / 10)?.to_string())]);
    }
    // Full 11xx–19xx year decade ("1980s" → "nineteen eighties", "1900s" →
    // "nineteen hundreds").  2000s are ambiguous ("two thousands" vs "twenty
    // twenties") and left alone.  The century reads as its teen word ("nineteen").
    if (1100..=1999).contains(&n) {
        const TEENS: [&str; 9] = [
            "eleven", "twelve", "thirteen", "fourteen", "fifteen",
            "sixteen", "seventeen", "eighteen", "nineteen",
        ];
        let (century, dec) = (n / 100, n % 100);
        Some(vec![
            Token::Word(TEENS[(century - 11) as usize].to_string()),
            Token::Word(if dec == 0 { "hundreds".to_string() } else { tens_plural(dec / 10)?.to_string() }),
        ])
    } else {
        None
    }
}

/// Read `<base>^<exp>` as "<base> to the power of <exp>" (`2^10` → "two to the
/// power of ten") — the caret is otherwise dropped.  Only between two numbers
/// (an unambiguous exponent), English-only.
fn apply_exponents(tokens: &mut Vec<Token>, lang: &str) {
    if primary_bcp47_subtag(lang) != "en" {
        return;
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Number(base) = &tokens[i] {
            if matches!(tokens.get(i + 1), Some(Token::Punctuation('^'))) {
                if let Some(Token::Number(exp)) = tokens.get(i + 2) {
                    out.push(Token::Number(base.clone()));
                    for w in ["to", "the", "power", "of"] {
                        out.push(Token::Word(w.to_string()));
                    }
                    out.push(Token::Number(exp.clone()));
                    i += 3;
                    continue;
                }
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// English fraction denominator word for a small denominator (`2`–`10`),
/// singular or plural — `1/2`→"half", `3/4`→"quarters".  `None` otherwise.
fn fraction_denominator_word(denom: &str, plural: bool) -> Option<&'static str> {
    Some(match (denom, plural) {
        ("2", false) => "half",     ("2", true) => "halves",
        ("3", false) => "third",    ("3", true) => "thirds",
        ("4", false) => "quarter",  ("4", true) => "quarters",
        ("5", false) => "fifth",    ("5", true) => "fifths",
        ("6", false) => "sixth",    ("6", true) => "sixths",
        ("7", false) => "seventh",  ("7", true) => "sevenths",
        ("8", false) => "eighth",   ("8", true) => "eighths",
        ("9", false) => "ninth",    ("9", true) => "ninths",
        ("10", false) => "tenth",   ("10", true) => "tenths",
        _ => return None,
    })
}

/// Read `<num>/<den>` as a spoken fraction (`1/2`→"one half", `3/4`→"three
/// quarters") for small denominators — this also renders the vulgar-fraction
/// characters, which `normalize_number_symbols` turns into `n/m`.  English-only;
/// a trailing `/` (a date like `1/2/2024`) is left alone.
fn apply_fractions(tokens: &mut Vec<Token>, lang: &str) {
    if primary_bcp47_subtag(lang) != "en" {
        return;
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Number(NumberToken::Cardinal(num)) = &tokens[i] {
            let is_slash = matches!(tokens.get(i + 1), Some(Token::Punctuation('/')));
            let not_date = !matches!(tokens.get(i + 3), Some(Token::Punctuation('/')));
            if is_slash && not_date {
                if let Some(Token::Number(NumberToken::Cardinal(den))) = tokens.get(i + 2) {
                    if let Some(word) = fraction_denominator_word(den, num != "1") {
                        // A whole number immediately before (`1 1/2`, i.e. `1½`) is a
                        // mixed number → "<whole> and <num> <fraction>" ("one and one
                        // half").  (The idiomatic "and a half" is avoided: a bare "a"
                        // reads as the letter name, not the article.)
                        let mixed = matches!(out.last(), Some(Token::Space | Token::WordJoin))
                            && matches!(out.get(out.len().wrapping_sub(2)), Some(Token::Number(NumberToken::Cardinal(_))));
                        if mixed {
                            out.pop(); // drop the space between whole and fraction
                            out.push(Token::Word("and".to_string()));
                        }
                        out.push(Token::Number(NumberToken::Cardinal(num.clone())));
                        out.push(Token::Word(word.to_string()));
                        i += 3;
                        continue;
                    }
                }
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// Look up the spoken name of a standalone symbol via the dictionary's
/// character-name entries (`_%`, `_$`, …), mirroring `LookupCharName`.
/// Returns the phoneme bytes, or empty if the symbol has no name in this voice.
/// Which name a symbol character should be read by.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SymbolName {
    /// The character appears in running text (`a − b`).  The dictionary's own
    /// entry for the character wins.
    Inline,
    /// Punctuation being announced by name (`--punct`), where C calls
    /// `LookupCharName()` and reads the `_<char>` entry — `&` is "ampersand"
    /// here but "and" inline.
    Announce,
    /// An arithmetic operator: the word for the *operation* wins over the
    /// dictionary's name for the character ("minus", not "minus sign").
    Operator,
}

/// How C classifies this character as clause punctuation
/// (`clause_type_from_codepoint`): which clause type it ends, and whether it
/// ends one even when no space follows (`OPTIONAL_SPACE_AFTER` — the Devanagari
/// danda and the CJK full stop do, which is how scripts written without spaces
/// get sentence breaks at all).  `None` for everything else.
///
/// Transcribed from the `ESPEAKNG_PROPERTY_*` clause-type properties in
/// `ucd-tools/src/proplist.c`, keeping only the property combinations C's
/// `switch` maps to a clause type: U+2048/U+2049 (`⁈`/`⁉`) carry both
/// QUESTION_MARK and EXCLAMATION_MARK, hit no case, and so *are* spoken by name.
fn clause_punctuation_class(c: char) -> Option<(ClauseTerminator, bool)> {
    use ClauseTerminator::{Comma, Exclamation, Period, Question};
    let code = c as u32;
    let class = match code {
        0x002C | 0x003A..=0x003B | 0x0387 | 0x055D | 0x060C | 0x061B | 0x0702
        | 0x0706..=0x0708 | 0x07F8 | 0x0F14 | 0x1363..=0x1366 | 0x1802 | 0x1804 | 0x1808
        | 0x2013..=0x2014 | 0x204F | 0x2982 | 0x2E32 | 0x2E34..=0x2E35 | 0x2E3A..=0x2E3B
        | 0x2E41 | 0xA4FE | 0xA60D | 0xA6F4..=0xA6F6 | 0xFE10..=0xFE11 | 0xFE13..=0xFE14
        | 0xFE31..=0xFE32 | 0xFE50..=0xFE51 | 0xFE54..=0xFE55 | 0xFF64 | 0x1144D
        | 0x12471..=0x12472 | 0x1DA87 | 0x1DA89..=0x1DA8A | 0x1F101..=0x1F10A | 0xE002C
        | 0xE003A..=0xE003B => (Comma, false),
        0x00A1 | 0x00BF | 0x0EAF | 0x1801 | 0x2026 | 0x22EE..=0x22F1 | 0x3001 | 0xFE19
        | 0xFF0C | 0xFF1A..=0xFF1B => (Comma, true),
        0x0021 | 0x055B..=0x055C | 0x0703 | 0x07F9 | 0x1944 | 0x203C | 0x2755 | 0x2757
        | 0x2762..=0x2763 | 0xFE15 | 0xFE57 | 0x1E95E | 0xE0021 => (Exclamation, false),
        0xFF01 => (Exclamation, true),
        0x002E | 0x06D4 | 0x0700..=0x0701 | 0x0704 | 0x0965 | 0x0F0E | 0x10FB | 0x1362
        | 0x1368 | 0x166E | 0x1803 | 0x1809 | 0x2488..=0x249B | 0x2CF9 | 0x2CFE | 0x2E33
        | 0x2E3C | 0xA4FF | 0xA60E | 0xA6F3 | 0xFE12 | 0xFE52 | 0xFF61 | 0x16AF5
        | 0x1BC9F | 0x1DA88 | 0x1F100 | 0xE002E => (Period, false),
        0x0589 | 0x0964 | 0x0DF4 | 0x0F0D | 0x3002 | 0xFF0E => (Period, true),
        0x003F | 0x037E | 0x055E | 0x061F | 0x0709 | 0x1367 | 0x1945 | 0x2047
        | 0x2753..=0x2754 | 0x2CFA..=0x2CFB | 0xA60F | 0xA6F7 | 0xFE16 | 0xFE56 | 0x11143
        | 0x1E95F | 0xE003F => (Question, false),
        0xFF1F => (Question, true),
        _ => return None,
    };
    Some(class)
}

/// Whether the character is clause punctuation at all — it breaks the clause
/// instead of being spoken, so it never reaches the dictionary for a name.
fn is_clause_punctuation(c: char) -> bool {
    clause_punctuation_class(c).is_some()
}

/// C's `IsBracket()`: a bracket or quote, which pauses rather than speaks.
fn is_bracket(c: char) -> bool {
    matches!(c,
        '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
        | '\u{ab}' | '\u{bb}'          // « »
        | '\u{300a}' | '\u{300b}')     // 《 》
        || matches!(c as u32, 0x2014..=0x201F) // dashes and the curly quotes
}

/// Does the dictionary give this character a name of its own — a bare
/// single-character list entry such as `#` → "hash" (`en_list`) or `−` →
/// "mínusz" (`hu_emoji`)?
///
/// This is the test C effectively applies to a symbol that is not clause
/// punctuation: the character reaches `TranslateWord()` like any other token, so
/// a list entry for it is spoken and anything else falls through.  The `_<char>`
/// *character-name* entries (`_(` → "left paren") are deliberately **not**
/// consulted here — C speaks those only when spelling or announcing
/// punctuation, which is why `a ( b` is silent while `a − b` is not.
fn dict_names_symbol(dict: &Dictionary, c: char, dict_condition: u32) -> bool {
    dict_symbol_flags(dict, c, dict_condition).is_some()
}

/// Whether the dictionary has a `_<char>` name entry for the character.
fn has_underscore_name(dict: &Dictionary, c: char, dict_condition: u32) -> bool {
    let ctx = LookupCtx { lookup_symbol: true, dict_condition, ..Default::default() };
    lookup(dict, &format!("_{c}"), &ctx).is_some_and(|r| r.flags1.found())
}

/// The dictionary flags of a bare single-character entry, when it has one.
/// `$max3` lives here: a symbol entry carrying it is spoken at most three times
/// in a row (C's `phonemes_repeat_count`), which is what keeps `~ ~ ~ ~ ~ ~`
/// to three tildes.
fn dict_symbol_flags(dict: &Dictionary, c: char, dict_condition: u32) -> Option<u32> {
    // Clause punctuation goes to the clause path, not the dictionary: Hindi
    // `।`, Ethiopic `።` and CJK `。` all have list entries naming them, and C
    // speaks none of them (`a ❓ b` → "a b").
    if is_clause_punctuation(c) && c != '.' {
        return None;
    }
    // The entry may be behind a `?n` condition — Portuguese names `§` twice,
    // once per variety (`?1 §` / `?2 §`) — so the language's own conditions have
    // to be in the context or neither matches.
    let ctx = LookupCtx { lookup_symbol: true, dict_condition, ..Default::default() };
    lookup(dict, &c.to_string(), &ctx)
        .filter(|r| r.flags1.found() && !r.phonemes.is_empty())
        .map(|r| r.flags1.0)
}

/// A symbol's reading, and whether it came from the dictionary **as phonemes**
/// (C's `SFLAG_DICTIONARY`, set when `FLAG_FOUND && !FLAG_TEXTMODE`).  The
/// difference is visible: `pl @` has phonemes in `pl_list`, so the
/// stress-conditioned changes don't touch it (`m'awpa`), while `tr −` has
/// *replacement text* in `tr_emoji`, which is re-translated by the rules and so
/// does take them (`eksi` → `Eks'I`).
fn lookup_symbol_name(
    c: char,
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
    mode: SymbolName,
    expect: PosExpect,
) -> (Vec<u8>, bool) {
    // The dictionary's own entry for the character comes first.  Every shipped
    // language carries the symbol block of its `*_emoji` data (86 of them name
    // U+2212 MINUS SIGN alone) and those are the readings C speaks, including
    // the ones the table below cannot express: the pause markers around `=`
    // (`_:'i:kw@Lz_:`), "sum sign" rather than "sum" for `∑`, and every
    // currency, arrow, fraction and sign the table never listed.
    // An operator is read by translating the character as a word — through the
    // language's list *and* its letter rules, which is where C finds it (`fi`
    // `__) - (  mi:nus`, `el`'s `- m'ion` entry).  The table below is only the
    // fallback for languages whose data has nothing.
    if mode == SymbolName::Operator {
        let wr = word_to_phonemes_pos(&c.to_string(), dict, phdata, stress_opts, options, expect);
        if wr.phonemes.iter().any(|&b| b != 0) {
            return (wr.phonemes, wr.found_in_list);
        }
    }
    if mode != SymbolName::Operator && mode != SymbolName::Announce
        && dict_names_symbol(dict, c, options.dict_condition)
    {
        let wr =
            word_to_phonemes_pos(&c.to_string(), dict, phdata, stress_opts, options, expect);
        if wr.phonemes.iter().any(|&b| b != 0) {
            return (wr.phonemes, wr.found_in_list);
        }
    }
    // An operator stops here.  C reads it by translating the character (above),
    // so when the language's data has no reading there is nothing to say — it
    // does *not* fall back to the character's spell-out name, which would make
    // `-5` "hyphen five" (fr "tiret", pl "myślnik"), nor to another language.
    // The table above is this port's own addition for the languages whose data
    // is silent; past it, silence is the right answer.
    if mode == SymbolName::Operator {
        return (Vec::new(), false);
    }
    // In running text C ignores a character the language names only as `_<char>`
    // (`LookupLetter`: "specified as _* so ignore it"), which is why French
    // says nothing for `/` or `#` unless punctuation is announced.
    if mode == SymbolName::Inline && c == '.' {
        let wr = word_to_phonemes_pos(".", dict, phdata, stress_opts, options, expect);
        if wr.phonemes.iter().any(|&b| b != 0) {
            return (wr.phonemes, wr.found_in_list);
        }
    }
    if mode == SymbolName::Inline
        && primary_bcp47_subtag(&options.lang) != "en"
        && !matches!(c, '$' | '€' | '£' | '¥' | '¢') && has_underscore_name(dict, c, options.dict_condition) {
        return (Vec::new(), false);
    }
    // Otherwise use the dictionary character-name entry (`_%`, `_$`, …).
    // Running text ignores a character that only has a `_<char>` name entry
    // (C's `LookupLetter` with `next_byte == -1`): `_/` is spoken when the
    // character is spelled, never in `a/b`.
    let keys = if mode == SymbolName::Inline {
        vec![c.to_string(), format!("_{c}")]
    } else {
        vec![format!("_{c}"), c.to_string()]
    };
    for key in keys {
        let wr = word_to_phonemes(&key, dict, phdata, stress_opts, options);
        if mode == SymbolName::Inline && key.starts_with('_') && !wr.phonemes.iter().all(|&b| b == 0) {
            return (Vec::new(), false);
        }
        // A result of nothing but terminators is not a reading: it used to be
        // returned as one, which silenced the fallback below (`fi -5` lost its
        // "miinus" because `_−` came back as a lone NUL).
        if wr.phonemes.iter().any(|&b| b != 0) {
            return (wr.phonemes, wr.found_in_list);
        }
    }
    // Still nothing: C's `LookupCharName` names the character in the **default
    // voice** instead of dropping it (`language = L('e','n')` when the
    // character's alphabet nominates nothing), wrapping the result in a
    // language switch — `sv #` reads `(en)h'aS(sv)`.  Returning the marker
    // hands that to the switch machinery, which re-translates the character
    // itself in the target language; if English has no name either, nothing is
    // spoken, as in C.
    if primary_bcp47_subtag(&options.lang) != "en" {
        let mut switch = vec![crate::phoneme::PHON_SWITCH];
        switch.extend_from_slice(b"en");
        return (switch, false);
    }
    (Vec::new(), false)
}

/// Lower-case a word the way the language does — C's `towlower2`.
///
/// Two differences from Rust's `char::to_lowercase`, which applies the *full*
/// Unicode mapping: `İ` (U+0130) lower-cases to a bare `i` rather than `i` plus
/// a combining dot (the dot broke the following rule match, so `tr İstanbul`
/// lost its `s` and read `itanb'UL`), and in Turkish and Azerbaijani `I` is the
/// *dotless* `ı` — C's `langopts.dotless_i`, which only those two set.  So `tr
/// IŞIK` is `@S'@k`, not `iS'Ik`.
fn lowercase_for_lang(word: &str, lang: &str) -> String {
    let dotless_i = matches!(primary_bcp47_subtag(lang), "tr" | "az");
    let mut out = String::with_capacity(word.len());
    for c in word.chars() {
        match c {
            'I' if dotless_i => out.push('ı'),
            'İ' => out.push('i'), // the simple mapping, as `ucd_tolower` gives
            _ => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// The first few UTF-8 bytes of the next word in the clause, for the rule
/// post-contexts that reach across a word boundary.  A *number* counts: the
/// hyphen of `-5` is read "minus" by a rule whose post-context is "boundary then
/// a digit" (`__) - (_D`), so the digits have to be visible.
fn next_word_opening(tokens: &[Token], from: usize, lang: &str) -> [u8; 8] {
    let mut out = [0u8; 8];
    let Some(next) = tokens[from..]
        .iter()
        .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
        .find_map(|t| match t {
            Token::Word(w) => Some(lowercase_for_lang(w, lang)),
            Token::Number(n) => Some(n.surface().to_string()),
            // A currency sign stays in C's clause text (pt `r (_\$` reads "R$").
            Token::Punctuation(c) if matches!(c, '$' | '€' | '£' | '¥' | '¢') => Some(c.to_string()),
            _ => None,
        })
    else {
        return out;
    };
    let b = next.as_bytes();
    let n = (0..=b.len().min(8)).rev().find(|&k| next.is_char_boundary(k)).unwrap_or(0);
    out[..n].copy_from_slice(&b[..n]);
    out
}

/// The words that follow `from` as a multi-word list entry sees them: lower
/// case, single-spaced, ending in a space, cut off at 31 bytes.  Anything but
/// words and spaces ends the run.
fn follow_text(tokens: &[Token], from: usize, lang: &str) -> [u8; 32] {
    let mut text = String::new();
    for t in &tokens[from..] {
        match t {
            Token::Space | Token::WordJoin => {}
            Token::Word(w) => {
                text.push_str(&lowercase_for_lang(w, lang));
                text.push(' ');
            }
            _ => break,
        }
        if text.len() >= 31 {
            break;
        }
    }
    let n = (0..=text.len().min(31)).rev().find(|&k| text.is_char_boundary(k)).unwrap_or(0);
    let mut out = [0u8; 32];
    out[..n].copy_from_slice(&text.as_bytes()[..n]);
    out
}

/// Whether no word follows the symbol in its clause — the case where C reads
/// the character by name instead of by its word entry.
fn symbol_ends_clause(tokens: &[Token], token_ix: usize) -> bool {
    !tokens[token_ix + 1..]
        .iter()
        .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
        .any(|t| matches!(t, Token::Word(_) | Token::Number(_) | Token::InlinePhonemes(_)))
}

pub(crate) fn build_translate_entries(
    tokens: &[Token],
    dict: &Dictionary,
    phdata: &PhonemeData,
    stress_opts: &StressOpts,
    options: &LangOptions,
    english_word: &dyn Fn(char, bool) -> Option<String>,
    alts: &letters::AltLangs,
    spelled: &[Option<usize>],
) -> Vec<TranslateEntry> {
    let mut entries: Vec<TranslateEntry> = Vec::with_capacity(tokens.len());
    // `$max3` bookkeeping: which symbol is repeating, and how many times so far.
    let mut repeat_of_symbol: Option<char> = None;
    let mut repeat_count = 0u32;
    // Part-of-speech homograph selection: a verb-triggering function word ("to",
    // a modal) makes the next word take its `$verb` pronunciation ("to lˈɪv" not
    // "lˈaɪv"); a perfect/passive auxiliary ("had"/"have"/"was") selects `$past`
    // ("have rˈɛd" not "rˈiːd").  The dictionary's default is already the
    // noun/adjective form, so only verb/past need triggering.  `prev_word` is the
    // previous *word* (carried across spaces, reset at a clause boundary).
    let mut prev_word: Option<String> = None;

    // Clause position per token, for the `$atend` / `$atstart` dictionary
    // entries: a word is "at the clause end" when no further word follows before
    // the next clause boundary (C: `word_end < translator->clause_end`).
    let is_word = |t: &Token| matches!(t, Token::Word(_) | Token::Number(_));
    let mut at_end = vec![false; tokens.len()];
    let mut at_start = vec![false; tokens.len()];
    let mut seen_word_in_clause = false;
    // A symbol that is read by name is a word of the clause like any other, so
    // it both *takes* a clause position (`👍` alone is clause-final and keeps
    // "up" stressed; `a 👍 b` reduces it) and *occupies* one, which stops the
    // word before it from being clause-final (`a ÷` reads "a#", not "'eI").
    // Punctuation that is not spoken does neither.
    let word_like: Vec<bool> = tokens
        .iter()
        .map(|t| match t {
            Token::Operator(_) => true,
            Token::Punctuation(c) => {
                is_spoken_symbol(*c)
                    || punct_covers(options, *c)
                    || dict_names_symbol(dict, *c, options.dict_condition)
            }
            t => is_word(t),
        })
        .collect();
    for (i, t) in tokens.iter().enumerate() {
        if matches!(t, Token::ClauseBoundary(_)) {
            seen_word_in_clause = false;
        } else if word_like[i] {
            at_start[i] = !seen_word_in_clause;
            seen_word_in_clause = true;
            at_end[i] = !tokens[i + 1..]
                .iter()
                .enumerate()
                .take_while(|(_, t)| !matches!(t, Token::ClauseBoundary(_)))
                .any(|(k, _)| word_like[i + 1 + k]);
        }
    }
    if options.switched {
        at_end.iter_mut().for_each(|e| *e = options.switched_last);
    }
    // The letters of an abandoned word keep the clause position of the word:
    // C translates each with the flags of the word they came from.
    let mut last_letter = std::collections::HashMap::new();
    for (i, word) in spelled.iter().enumerate() {
        if let Some(first) = word {
            last_letter.insert(*first, i);
        }
    }
    for (i, word) in spelled.iter().enumerate() {
        if let Some(first) = word {
            at_start[i] = at_start[*first];
            at_end[i] = at_end[last_letter[first]];
        }
    }

    // Per-clause letter-case counts, for C's abbreviation rule: "an upper case
    // word in a lower case clause could be an abbreviation".
    let (clause_lower, clause_upper) = {
        let mut lower = vec![0usize; tokens.len()];
        let mut upper = vec![0usize; tokens.len()];
        let mut start = 0usize;
        for i in 0..=tokens.len() {
            if i == tokens.len() || matches!(tokens[i], Token::ClauseBoundary(_)) {
                let (mut l, mut u) = (0usize, 0usize);
                for t in &tokens[start..i] {
                    if let Token::Word(w) = t {
                        l += w.chars().filter(|c| c.is_lowercase()).count();
                        u += w.chars().filter(|c| c.is_uppercase()).count();
                    }
                }
                lower[start..i].fill(l);
                upper[start..i].fill(u);
                start = i + 1;
            }
        }
        (lower, upper)
    };

    // Words a multi-word list entry already pronounced with its first word.
    let mut skip_words = 0usize;
    let mut dot_removed_at: Option<usize> = None;
    for (token_ix, token) in tokens.iter().enumerate() {
        match token {
            Token::Punctuation('.') if dot_removed_at == Some(token_ix) => {
                entries.push(TranslateEntry {
                    phonemes: Vec::new(),
                    dict_flags: 0,
                    kind: TranslateEntryKind::Other,
                    word_lower: None,
                    no_word_gap: false,
                    word_group: None,
                    found_in_list: false,
                });
            }
            // Later passes pair entries with word tokens one to one, so a word
            // the entry swallowed keeps an empty entry of its own.
            Token::Word(_) if skip_words > 0 => {
                skip_words -= 1;
                entries.push(TranslateEntry {
                    phonemes: Vec::new(),
                    dict_flags: 0,
                    found_in_list: false,
                    kind: TranslateEntryKind::Word,
                    word_lower: None,
                    no_word_gap: false,
                    word_group: spelled.get(token_ix).copied().flatten(),
                });
            }
            Token::Word(word) => {
                // A word in a script the voice can't read (Cyrillic/Greek inside a
                // Latin clause) switches to a voice for that script — the same
                // `phonSWITCH` re-translation used for `_^_LL` dict entries — so it
                // is spoken instead of dropped.
                // …unless this language's own dictionary knows the word: a
                // listed loanword or a pinyin reading belongs to the current
                // voice, and upstream likewise consults the dictionary before
                // the alphabet check in `TranslateRules`.
                // A word in a script this voice doesn't read, and that upstream
                // does *not* mark `AL_WORDS`, is **spelled** with the current
                // language's letter names rather than switching voice — but only
                // once the normal translation has had its turn, because some
                // letters are covered by the language's own rules (English reads
                // Greek letters directly, so `αβγ` is one word, while Cyrillic
                // falls through to spelling and comes out as separate words).
                // The dictionary's `.replace` table runs before anything else —
                // C applies it per character in `TranslateClause` — and it is
                // how a digraphic language reads its second script:
                // `mk "Dzidarski"` → `ѕидарски`, `lfn "Тота"` → `tota`,
                // `uz "Ўзбекча"` → `ózbekça`.  The script tests below have to
                // look at the *replaced* text, or the word is declared foreign
                // and spelled out letter by letter.
                // The table is keyed on lower case, as C's `SubstituteChar` is
                // (it folds the character before the lookup), so `Dzidarski`
                // transliterates as well as `dzidarski` does.
                let replaced = dict.apply_replacements(word);
                // Only the *script* tests below look at the replaced text; the
                // translation itself keeps the original, since
                // `word_to_phonemes` applies the table again (Icelandic's
                // `x → xz` would double up).
                let word_script = replaced.as_deref().unwrap_or(word);
                let foreign = word_script
                    .chars()
                    .find_map(char_script)
                    .filter(|sc| !native_scripts(&options.lang).contains(sc));
                let next_in_script = |from: usize| {
                    tokens[from..].iter().find_map(|t| match t {
                        Token::Word(w) => Some(w.chars().next().and_then(char_script) == foreign),
                        _ => None,
                    })
                };
                let lower = lowercase_for_lang(word, &options.lang);
                let letters = word.chars().filter(|c| c.is_alphabetic());
                // The opening of the next word in the same clause, for the
                // post-contexts that reach across the boundary.
                let next_word = next_word_opening(tokens, token_ix + 1, &options.lang);
                // C re-reads each letter of an abandoned word from a buffer outside
                // the clause text, where `word_end < clause_end` always holds.
                let expect = PosExpect {
                    at_clause_end: at_end[token_ix] && spelled[token_ix].is_none(),
                    at_clause_start: at_start[token_ix],
                    first_upper: word.chars().next().is_some_and(char::is_uppercase),
                    all_upper: letters.clone().next().is_some()
                        && letters.clone().all(char::is_uppercase),
                    next_word,
                    // The tone-sandhi pass indexes one syllable run per word and
                    // cannot take a merged phrase entry.
                    follow: if matches!(
                        primary_bcp47_subtag(&options.lang),
                        "cmn" | "zh" | "yue" | "hak" | "vi" | "shn"
                    ) {
                        [0; 32]
                    } else {
                        follow_text(tokens, token_ix + 1, &options.lang)
                    },
                    hyphen_after: matches!(tokens.get(token_ix + 1), Some(Token::WordJoin)),
                    hyphen_flags: hyphen_flags_at(tokens, token_ix),
                    prev_tail: word_tail(prev_word.as_deref()),
                    ..pos_expect_after_in(prev_word.as_deref(), dict, options.dict_condition)
                };
                let mut wr = word_to_phonemes_pos(&lower, dict, phdata, stress_opts, options, expect);
                // The indicator of an ordinal that was read as a number (`3.º`)
                // is already spoken in the number.
                let ordinal_indicator = token_ix > 0
                    && matches!(tokens[token_ix - 1], Token::Number(NumberToken::Ordinal(_)));
                if !wr.found_in_list && wr.phonemes.iter().all(|&b| b == 0) && !ordinal_indicator {
                    let mut chars = lower.chars();
                    let lone = chars.next().filter(|c| c.is_alphabetic() && chars.next().is_none());
                    let voice = letters::Ctx { dict, phdata, stress: stress_opts, options };
                    let env = letters::LetterEnv { voice, tr: voice, alts };
                    let reading = lone.map(|c| (c, letters::translate_letter(&env, c)));
                    if let Some((c, reading)) = reading {
                        prev_word = Some(lower.clone());
                        let (mut phonemes, kind) = match reading {
                            letters::LetterOut::HandOver(name) => {
                                let target = letters::switch_language(&name);
                                match letters::spell_in_switched(voice, alts, &target, c) {
                                    Some(items) if items.is_empty() => {
                                        (Vec::new(), TranslateEntryKind::Word)
                                    }
                                    Some(items) => {
                                        (letters::encode(&items), TranslateEntryKind::Stream)
                                    }
                                    None => (name, TranslateEntryKind::Word),
                                }
                            }
                            letters::LetterOut::Items(items) if letters::has_switch(&items) => {
                                (letters::encode(&items), TranslateEntryKind::Stream)
                            }
                            letters::LetterOut::Items(items) => {
                                (letters::plain_bytes(&items), TranslateEntryKind::Word)
                            }
                        };
                        phonemes.push(0);
                        entries.push(TranslateEntry {
                            phonemes,
                            dict_flags: 0,
                            kind,
                            word_lower: Some(c.to_string()),
                            no_word_gap: false,
                            word_group: spelled.get(token_ix).copied().flatten(),
                            found_in_list: false,
                        });
                        continue;
                    }
                }
                // C drops the opening pause of a word that follows a hyphen.
                if token_ix > 0 && matches!(tokens[token_ix - 1], Token::WordJoin) {
                    if wr.phonemes.first().is_some_and(|&c| {
                        c != crate::phoneme::PHON_SWITCH
                            && phdata.get(c).is_some_and(|p| p.typ == crate::phoneme::PH_PAUSE)
                    }) {
                        wr.phonemes.remove(0);
                    }
                }
                // A lone letter handed to another voice (`it м` → `_^_RU`, a
                // Devanagari letter in `de`) is spelled by that voice, with the
                // alphabet's name first, as `TranslateLetter` does; a word
                // followed by another in the same script is switched whole.
                if wr.phonemes.first() == Some(&crate::phoneme::PHON_SWITCH)
                    && next_in_script(token_ix + 1) != Some(true)
                {
                    // `TranslateChar` has already broken a Hangul syllable into jamo.
                    let jamo = decompose_hangul(word_script);
                    let mut chars = jamo.as_deref().unwrap_or(word_script).chars();
                    let lone = chars.next().filter(|c| c.is_alphabetic() && chars.next().is_none());
                    let target = String::from_utf8_lossy(&wr.phonemes[1..])
                        .trim_end_matches('\0')
                        .to_string();
                    let voice = letters::Ctx { dict, phdata, stress: stress_opts, options };
                    let items = lone.and_then(|c| letters::spell_in_switched(voice, alts, &target, c));
                    if let Some(items) = items {
                        let mut phonemes = letters::encode(&items);
                        phonemes.push(0);
                        let silent = items.is_empty();
                        entries.push(TranslateEntry {
                            phonemes: if silent { Vec::new() } else { phonemes },
                            dict_flags: 0,
                            kind: if silent { TranslateEntryKind::Word } else { TranslateEntryKind::Stream },
                            word_lower: Some(word.clone()),
                            no_word_gap: false,
                            word_group: None,
                            found_in_list: false,
                        });
                        prev_word = None;
                        continue;
                    }
                }
                skip_words = wr.skip_words;
                let allows_dot = wr.dict_flags
                    & (crate::dictionary::FLAG_ALLOW_DOT | crate::dictionary::FLAG_NEEDS_DOT)
                    != 0;
                dot_removed_at = allows_dot.then_some(token_ix + 1);
                let lower = if skip_words == 0 {
                    lower
                } else {
                    let rest = follow_text(tokens, token_ix + 1, &options.lang);
                    let rest = String::from_utf8_lossy(rest.split(|&b| b == 0).next().unwrap_or(&[]))
                        .split_whitespace()
                        .take(skip_words)
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("{lower} {rest}")
                };
                let roman = (!wr.found_in_list
                    && roman_numerals_apply(options.number_grammar.c_numbers, expect.all_upper)
                    && !(expect.first_upper && !expect.all_upper)
                    && !dict_lists_word(word, dict, options)
                    && !matches!(tokens.get(token_ix.wrapping_sub(1)), Some(Token::Number(_))))
                .then(|| roman_number_phonemes(&lower, dict, phdata, stress_opts, &options.number_grammar))
                .flatten();
                // C spells a word whose list entry carries `$abbrev` but gives no
                // pronunciation ("the word has $abbrev flag, but no pronunciation
                // specified.  Speak as individual letters") — English `usa` is
                // `j,u:,Es'eI`.
                let dot_follows = matches!(tokens.get(token_ix + 1), Some(Token::Punctuation('.')));
                if skip_words == 0 && dot_follows && is_single_letter_word(&lower) {
                    // C's `CheckDottedAbbrev`: a lone letter before a dot word is spelled.
                    if let Some(spelled) = spell_word_letters(&lower, dict, phdata, stress_opts, options) {
                        wr.phonemes = spelled;
                        wr.found_in_list = false;
                        wr.dict_flags = 0;
                    }
                } else if !wr.found_in_list && lower.contains('.') {
                    // Single letters joined by dots (`e.g`, `U.S.A`) are one
                    // word that no list names; it is spelled out.
                    if let Some(spelled) = spell_word_letters(
                        &lower.replace('.', ""), dict, phdata, stress_opts, options,
                    ) {
                        wr.phonemes = spelled;
                    }
                } else if !wr.found_in_list
                    && wr.dict_flags & crate::dictionary::FLAG_ABBREV != 0
                {
                    if let Some(spelled) =
                        spell_word_letters(&lower, dict, phdata, stress_opts, options)
                    {
                        wr.phonemes = spelled;
                    }
                }
                // An English word that is a valid Roman numeral and that the
                // dictionary does not list (`vi`, `xii`) is read as one.
                else if let Some(roman) = roman {
                    wr.phonemes = roman;
                }
                // A word the dictionary doesn't pronounce and whose first vowel
                // is too far in — "BBC" has none at all — is spelled letter by
                // letter, then whatever remains is translated normally.
                else if !wr.found_in_list
                    && !dict_knows_word(&lower, dict)
                    && !dict_lists_word(word, dict, options)
                    && is_unpronounceable(&lower, &next_word, dict, options)
                {
                    if let Some((letters, rest)) =
                        spell_unpronounceable(&lower, dict, phdata, stress_opts, options, &next_word)
                    {
                        let mut ph = letters;
                        if !rest.is_empty() {
                            // The remainder is still part of the same word, so
                            // the letters already spelled stay available as a
                            // rule pre-context (`s's` → `'Es` + `s) 's` → `I2z`,
                            // not the bare `'s` → `z`).
                            // C blanks the byte before the rest unless it opens with an apostrophe.
                            let spelled = if rest.starts_with('\'') {
                                &lower[..lower.len() - rest.len()]
                            } else {
                                ""
                            };
                            let mut prev_letters = [0u8; 8];
                            let b = spelled.as_bytes();
                            let keep = b.len().min(prev_letters.len());
                            let from = (0..=keep)
                                .rev()
                                .find(|&k| spelled.is_char_boundary(b.len() - k))
                                .unwrap_or(0);
                            prev_letters[..from].copy_from_slice(&b[b.len() - from..]);
                            let tail = word_to_phonemes_pos(
                                rest,
                                dict,
                                phdata,
                                stress_opts,
                                options,
                                PosExpect { prev_letters, rules_only: true, ..PosExpect::default() },
                            );
                            ph.extend(
                                without_leading_break(&tail.phonemes, stress_opts).iter().copied().filter(|&b| b != 0),
                            );
                        }
                        wr.phonemes = ph;
                    }
                }
                // …and a *short* all-capitals word the dictionary doesn't
                // pronounce, in a predominantly lower-case clause, is read as an
                // abbreviation too.
                else if expect.all_upper
                    && !wr.found_in_list
                    && (2..4).contains(&word.chars().count())
                    && word.chars().next().is_some_and(char::is_alphabetic)
                    && clause_lower[token_ix] > 3
                    && clause_upper[token_ix] <= clause_lower[token_ix]
                {
                    if let Some(spelled) =
                        spell_word_letters(&lower, dict, phdata, stress_opts, options)
                    {
                        wr.phonemes = spelled;
                        wr.found_in_list = false;
                    }
                }
                prev_word = Some(lower.clone());
                // Word-final inherent-vowel (schwa) deletion for the Indo-Aryan
                // scripts (Devanagari, Bengali, Gujarati, Gurmukhi).  Dravidian
                // (ta/te/kn/ml) and Oriya keep the inherent vowel, so they're
                // excluded; `delete_final_schwa` is also a no-op where the
                // inherent-vowel phoneme isn't `V`.
                if matches!(
                    primary_bcp47_subtag(&options.lang),
                    "hi" | "mr" | "ne" | "sa" | "kok" | "bn" | "as" | "gu" | "pa"
                ) {
                    delete_final_schwa(&mut wr.phonemes, phdata);
                }
                entries.push(TranslateEntry {
                    phonemes:   wr.phonemes,
                    dict_flags: wr.dict_flags,
                    found_in_list: wr.found_in_list,
                    kind:       TranslateEntryKind::Word,
                    word_lower: Some(lower),
                    no_word_gap: false,
                    word_group: spelled.get(token_ix).copied().flatten(),
                });
            }
            Token::Number(token) => {
                // Portuguese numbers: the pt dict lacks number keys, so build the
                // number word(s) from the value and pronounce them via the rules.
                if options.number_grammar.portuguese_cardinals {
                    if let Some(word) = portuguese_number_word(token) {
                        let mut phonemes: Vec<u8> = Vec::new();
                        // The composed entry inherits the *last* word's dict
                        // flags: clause-final stress promotion keys off
                        // `FLAG_STREND`, so dropping them left "101" as
                        // `sˈẽŋtʊ i ũŋ` where the same words typed out give
                        // `… ˈũŋ` (and upstream stresses both).
                        let mut last_flags = 0;
                        let parts: Vec<&str> = word.split_whitespace().collect();
                        for (pi, part) in parts.iter().enumerate() {
                            let wr = word_to_phonemes_pos(
                                part, dict, phdata, stress_opts, options,
                                expect_before(parts.get(pi + 1).copied()),
                            );
                            let ph = wr.phonemes.strip_suffix(&[0]).unwrap_or(&wr.phonemes);
                            if ph.is_empty() {
                                continue;
                            }
                            if !phonemes.is_empty() {
                                phonemes.push(crate::phoneme::PHON_END_WORD);
                            }
                            phonemes.extend_from_slice(ph);
                            last_flags = wr.dict_flags;
                        }
                        entries.push(TranslateEntry {
                            phonemes,
                            dict_flags: last_flags,
                            kind:       TranslateEntryKind::Word,
                            word_lower: Some(word),
                        no_word_gap: false, word_group: None,
                            found_in_list: false,
                        });
                        continue;
                    }
                }
                let wr = translate_number_token_with_tail(
                    token,
                    dict,
                    phdata,
                    stress_opts,
                    &options.number_grammar,
                    &|symbol| symbol_phonemes(symbol, dict, options),
                    number_tail(tokens, token_ix, dict),
                )
                .unwrap_or_else(|| {
                    let surface = token.surface();
                    word_to_phonemes(&surface, dict, phdata, stress_opts, options)
                });
                entries.push(TranslateEntry {
                    phonemes:   wr.phonemes,
                    dict_flags: wr.dict_flags,
                    found_in_list: wr.found_in_list,
                    kind:       TranslateEntryKind::Word,
                    word_lower: Some(token.surface().to_string().to_lowercase()),
                        no_word_gap: false, word_group: None
                });
            }
            Token::InlinePhonemes(content) => {
                entries.push(TranslateEntry {
                    phonemes:   parse_inline_phonemes(content, phdata),
                    dict_flags: 0,
                    kind:       TranslateEntryKind::Word,
                    word_lower: None,
                        no_word_gap: false, word_group: None,
                    found_in_list: false,
                });
            }
            Token::ClauseBoundary(c) => {
                // A clause boundary resets the part-of-speech expectation.
                prev_word = None;
                // `--punct`: announce clause punctuation by name instead of a
                // silent break.  Falls back to a break if the char has no name.
                let (name, name_from_dict) = if punct_covers(options, *c) {
                    lookup_symbol_name(
                        *c, dict, phdata, stress_opts, options, SymbolName::Announce,
                        PosExpect {
                            at_clause_end: at_end[token_ix],
                            at_clause_start: at_start[token_ix],
                            ..PosExpect::default()
                        },
                    )
                } else {
                    (Vec::new(), false)
                };
                if name.is_empty() {
                    entries.push(TranslateEntry {
                        phonemes:   Vec::new(),
                        dict_flags: 0,
                        kind:       TranslateEntryKind::ClauseBoundary,
                        word_lower: None,
                        no_word_gap: false, word_group: None,
                        found_in_list: false,
                    });
                } else {
                    entries.push(TranslateEntry {
                        phonemes:   name,
                        dict_flags: 0,
                        kind:       TranslateEntryKind::Word,
                        word_lower: None,
                        no_word_gap: false, word_group: None,
                        // A dictionary pronunciation is a *translation given*
                        // (C's `SFLAG_DICTIONARY`), which suppresses the
                        // stress-conditioned phoneme changes; a name that came
                        // from replacement text was read by the rules and does
                        // take them.
                        found_in_list: name_from_dict,
                    });
                }
            }
            // A point between two numbers that is not their decimal separator
            // (`de 3.14`) is not a word: C appends the language's `_.` entry to
            // the number before it (see `number_tail`), so a language without
            // that entry reads it as silence.
            Token::Punctuation('.') if point_after_number(tokens, token_ix) => {
                entries.push(TranslateEntry {
                    phonemes: Vec::new(), dict_flags: 0,
                    kind: TranslateEntryKind::Other, word_lower: None,
                    no_word_gap: false, word_group: None,
                    found_in_list: false,
                });
            }
            Token::Operator(c) | Token::Punctuation(c)
                if matches!(token, Token::Operator(_))
                    || is_spoken_symbol(*c)
                    || punct_covers(options, *c)
                    || dict_names_symbol(dict, *c, options.dict_condition)
                    || (!is_clause_punctuation(*c) && english_word(*c, false).is_some()) =>
            {
                // Technical symbols read as their name by default (# $ % & * + / = @ ~),
                // as does any character the dictionary names (see `dict_names_symbol`).
                let mode = if matches!(token, Token::Operator(_)) {
                    SymbolName::Operator
                } else if punct_covers(options, *c) && !is_spoken_symbol(*c) {
                    SymbolName::Announce
                } else {
                    SymbolName::Inline
                };
                // `$max3`: a symbol repeated more than three times in a row is
                // dropped after the third (C's `phonemes_repeat_count`), so
                // `~ ~ ~ ~ ~ ~` is three tildes.  Letters are *not* limited —
                // C limits them too, which upstream #2551 reports as a bug and
                // #2552 fixes; this port already reads all the repeats.
                let max3 = dict_symbol_flags(dict, *c, options.dict_condition)
                    .is_some_and(|f| f & crate::dictionary::FLAG_MAX3 != 0);
                let (phonemes, symbol_from_dict) = lookup_symbol_name(
                    *c, dict, phdata, stress_opts, options, mode,
                    PosExpect {
                        at_clause_end: at_end[token_ix],
                        at_clause_start: at_start[token_ix],
                        next_word: next_word_opening(tokens, token_ix + 1, &options.lang),
                        ..PosExpect::default()
                    },
                );
                let english_reading = (mode == SymbolName::Inline
                    && phonemes.first() == Some(&crate::phoneme::PHON_SWITCH)
                    && !dict_names_symbol(dict, *c, options.dict_condition))
                .then(|| english_word(*c, symbol_ends_clause(tokens, token_ix)));
                let phonemes = if english_reading == Some(None) { Vec::new() } else { phonemes };
                let phonemes = if max3 && repeat_of_symbol == Some(*c) {
                    repeat_count += 1;
                    if repeat_count > 3 { Vec::new() } else { phonemes }
                } else {
                    repeat_of_symbol = max3.then_some(*c);
                    repeat_count = 1;
                    phonemes
                };
                if phonemes.is_empty() {
                    entries.push(TranslateEntry {
                        phonemes, dict_flags: 0,
                        kind: TranslateEntryKind::Other, word_lower: None,
                        no_word_gap: false, word_group: None,
                        found_in_list: false,
                    });
                } else {
                    entries.push(TranslateEntry {
                        phonemes, dict_flags: 0,
                        kind: TranslateEntryKind::Word,
                        // What a language switch in these phonemes looks up in
                        // the target language.  A switch that came from the
                        // dictionary *entry* re-translates the character itself
                        // (`fr ©` is a bare `phonSWITCH`, and English's own `©`
                        // says "copyright"); a switch from the character-name
                        // fallback is C's `LookupCharName`, which reads the
                        // `_<char>` entry there (`he @` → English `_@` → "at
                        // sign").  Without either the marker leaked out as `_^_`.
                        word_lower: Some(match &english_reading {
                            Some(Some(word)) => word.clone(),
                            _ if dict_names_symbol(dict, *c, options.dict_condition) => c.to_string(),
                            _ => format!("_{c}"),
                        }),
                        no_word_gap: false, word_group: None,
                        // Phonemes from the dictionary are a *translation
                        // given*, so the stress-conditioned changes don't apply
                        // (`pl @` is `m'awpa` where the typed word "małpa",
                        // read by the rules, is `m'awpa#`); replacement text is
                        // re-translated and does take them (`tr −` → `Eks'I`).
                        found_in_list: symbol_from_dict,
                    });
                }
            }
            _ => {
                entries.push(TranslateEntry {
                    phonemes:   Vec::new(),
                    dict_flags: 0,
                    kind:       TranslateEntryKind::Other,
                    word_lower: None,
                        no_word_gap: false, word_group: None,
                    found_in_list: false,
                });
            }
        }
        // Any other word that speaks ends the `$max3` run, as C clears
        // `phonemes_repeat_count` for every word without the flag: the three
        // tildes of `~ a ~ ~ ~ ~` start again after the "a".
        let same_symbol = match token {
            Token::Punctuation(c) | Token::Operator(c) => repeat_of_symbol == Some(*c),
            _ => false,
        };
        if !same_symbol
            && entries.last().is_some_and(|e| e.phonemes.iter().any(|&b| b != 0))
        {
            repeat_of_symbol = None;
            repeat_count = 0;
        }
    }
    // Retag words whose translation is a language-switch marker so that
    // stress promotion skips them (they hold language bytes, not phonemes).
    for e in &mut entries {
        if e.kind == TranslateEntryKind::Word
            && e.phonemes.first() == Some(&crate::phoneme::PHON_SWITCH)
        {
            e.kind = TranslateEntryKind::LangSwitch;
        }
    }
    entries
}

/// The mnemonic of a phoneme code as a string (`"214"`, `"55"`, …).
fn phoneme_mnemonic(code: u8, phdata: &PhonemeData) -> String {
    phdata
        .get(code)
        .map(|ph| {
            ph.mnemonic
                .to_le_bytes()
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect()
        })
        .unwrap_or_default()
}

/// Give every toneless syllable the default tone (`phonDEFAULTTONE`, code 17 —
/// mnemonic `1`), as `CalcPitches_Tone` does for the tone languages.
///
/// Vietnamese additionally turns the *final stressed* syllable's missing tone
/// into the falling tone 7 at the end of a clause.
fn assign_default_tones(entries: &mut [TranslateEntry], phdata: &PhonemeData, base: &str) {
    const PHON_DEFAULT_TONE: u8 = 17;
    if phdata.get(PHON_DEFAULT_TONE).is_none_or(|ph| ph.typ != 1) {
        return; // this table has no tone phonemes
    }
    let vi_final_tone = (base == "vi").then(|| phdata.lookup_phoneme("7")).filter(|&c| c != 0);

    // Index of the last syllable that takes a tone, for Vietnamese's clause-final rule.
    let mut last_toneless: Option<(usize, usize)> = None;
    for ei in 0..entries.len() {
        if entries[ei].kind != TranslateEntryKind::Word {
            continue;
        }
        let mut i = 0;
        while i < entries[ei].phonemes.len() {
            let code = entries[ei].phonemes[i];
            if code == 0 {
                break;
            }
            if !matches!(phdata.get(code), Some(ph) if ph.typ == 2 /* phVOWEL */) {
                i += 1;
                continue;
            }
            let has_tone = entries[ei]
                .phonemes
                .get(i + 1)
                .is_some_and(|&c| is_tone_phoneme(c, phdata));
            if !has_tone {
                entries[ei].phonemes.insert(i + 1, PHON_DEFAULT_TONE);
                last_toneless = Some((ei, i + 1));
                i += 1;
            }
            i += 1;
        }
    }
    // Vietnamese: the clause-final stressed syllable takes the falling tone.
    if let (Some(tone7), Some((ei, pos))) = (vi_final_tone, last_toneless) {
        if entries[ei].phonemes.get(pos) == Some(&PHON_DEFAULT_TONE) {
            let is_last_word = entries[ei + 1..]
                .iter()
                .all(|e| e.kind != TranslateEntryKind::Word);
            if is_last_word {
                entries[ei].phonemes[pos] = tone7;
            }
        }
    }
}

/// Mandarin tone sandhi and default-tone assignment — a port of the Mandarin
/// branch of `CalcPitches_Tone()` (`intonation.c`), which rewrites the tone
/// phonemes *after* the phoneme list is built:
///
/// * a syllable with no tone gets tone 1 (`55`) after a pause, otherwise the
///   neutral tone 5 (`11`);
/// * `214` + `214` → the first becomes `35` (third-tone sandhi: nǐ hǎo →
///   "ní hǎo"); `214` before any other tone becomes `21`;
/// * `51` + `51` → the first becomes `53`;
/// * the neutral tone `11` takes its level from the preceding tone
///   (`55`→`22`, `53`→`33`, `214`→`44`).
///
/// Runs on the whole clause, since the rules reach across word boundaries.
fn apply_tone_sandhi(entries: &mut [TranslateEntry], phdata: &PhonemeData, lang: &str) {
    let base = primary_bcp47_subtag(lang);
    // Upstream's tone languages (`langopts.tone_language`), the ones that run
    // `CalcPitches_Tone`.  Thai is *not* one of them — it uses ordinary
    // intonation, so it only gets the placement pass.
    if !matches!(base, "cmn" | "zh" | "yue" | "hak" | "vi" | "shn") {
        return;
    }
    if base != "cmn" && base != "zh" {
        // No sandhi table for these, but every syllable still ends up with a
        // tone: `CalcPitches_Tone` gives a toneless one `phonDEFAULTTONE`
        // ("no tone specified, use default tone 1"), which is why Vietnamese
        // "xin" is `s'i1n` and not `s'in`.
        assign_default_tones(entries, phdata, base);
        return;
    }
    let code_of = |m: &str| phdata.lookup_phoneme(m);

    // Position of the previous syllable's tone phoneme, so sandhi can rewrite it.
    let mut prev_slot: Option<(usize, usize)> = None;
    let mut prev_tone = String::new();  // previous tone in this word
    let mut prevw_tone = String::new(); // previous tone across word boundaries
    let mut pause = true;
    let mut promoted = false;

    for ei in 0..entries.len() {
        match entries[ei].kind {
            TranslateEntryKind::Word => {}
            TranslateEntryKind::ClauseBoundary => {
                pause = true;
                prevw_tone.clear();
                prev_tone.clear();
                continue;
            }
            _ => continue,
        }
        prev_tone.clear(); // forget across word boundaries
        let mut i = 0;
        while i < entries[ei].phonemes.len() {
            let code = entries[ei].phonemes[i];
            if code == 0 {
                break;
            }
            let is_vowel = matches!(phdata.get(code), Some(ph) if ph.typ == 2);
            if !is_vowel {
                i += 1;
                continue;
            }

            // The syllable's tone sits directly after its vowel.
            let tone_pos = i + 1;
            let existing = entries[ei]
                .phonemes
                .get(tone_pos)
                .copied()
                .filter(|&c| is_tone_phoneme(c, phdata));
            let tone_code = match existing {
                Some(c) => {
                    promoted = false;
                    c
                }
                None => {
                    let mnem = if pause || promoted { "55" } else { "11" };
                    let c = code_of(mnem);
                    if c == 0 {
                        i += 1;
                        continue;
                    }
                    promoted = mnem == "55";
                    entries[ei].phonemes.insert(tone_pos, c);
                    c
                }
            };
            let mut tone = phoneme_mnemonic(tone_code, phdata);

            // 214 + 214 → 35 + 214;  214 + anything else → 21
            if prevw_tone == "214" {
                if let Some((pe, pp)) = prev_slot {
                    let joins = tone == "214";
                    let new = if joins { code_of("35") } else { code_of("21") };
                    if new != 0 {
                        entries[pe].phonemes[pp] = new;
                    }
                    if joins {
                        // 214 + 214 → 35 + 214 binds the two syllables into one
                        // word: upstream drops both the inter-word gap and the
                        // first syllable's stress (`ni35X'Au214_|`).
                        entries[pe].no_word_gap = true;
                        entries[pe]
                            .phonemes
                            .retain(|&c| !matches!(c, PHON_STRESS_P | PHON_STRESS_P2));
                    }
                }
            }
            // 51 + 51 → 53 + 51 (within the word)
            if prev_tone == "51" && tone == "51" {
                if let Some((pe, pp)) = prev_slot {
                    let new = code_of("53");
                    if new != 0 {
                        entries[pe].phonemes[pp] = new;
                    }
                }
            }
            // The neutral tone takes its level from the preceding tone.
            if tone == "11" {
                let replacement = match prevw_tone.as_str() {
                    "55" => "22",
                    "53" => "33",
                    "214" => "44",
                    _ => "",
                };
                if !replacement.is_empty() {
                    let new = code_of(replacement);
                    if new != 0 {
                        entries[ei].phonemes[tone_pos] = new;
                        tone = replacement.to_string();
                    }
                }
            }

            prev_slot = Some((ei, tone_pos));
            prev_tone = tone.clone();
            prevw_tone = tone;
            pause = false;
            i = tone_pos + 1;
        }
    }
}

pub(crate) fn promote_translate_entries(
    entries: &mut [TranslateEntry],
    phdata: &PhonemeData,
    lang: &str,
    outer_continues: bool,
) {
    const FLAG_STREND: u32 = 1 << 9;
    const FLAG_STREND2: u32 = 1 << 10;
    const PHON_STRESS_P_CODE: u8 = 6;
    const PHON_STRESS_P2_CODE: u8 = 7;

    fn promote_clause(
        entries: &mut [TranslateEntry],
        phdata: &PhonemeData,
        lang: &str,
        outer_continues: bool,
    ) {
        let n = entries.len();
        let alt_attributes = StressOpts::for_lang(lang).alt_attributes;
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            if entries[i].kind != TranslateEntryKind::Word {
                continue;
            }
            let dict_flags = entries[i].dict_flags;
            if dict_flags & (FLAG_STREND | FLAG_STREND2) == 0 {
                continue;
            }

            // The letters of one abandoned word all count as its last word.
            let own_group = entries[i].word_group;
            let is_last_word = !outer_continues
                && entries[i + 1..].iter().all(|e| {
                    !matches!(e.kind, TranslateEntryKind::Word | TranslateEntryKind::LangSwitch)
                        || (own_group.is_some() && e.word_group == own_group)
                });
            let following_all_unstressed = entries[i + 1..]
                .iter()
                .filter(|e| e.kind == TranslateEntryKind::Word)
                .all(|e| {
                    !e.phonemes
                        .iter()
                        .any(|&c| c == PHON_STRESS_P_CODE || c == PHON_STRESS_P2_CODE)
                });

            // A composed entry (a spelled-out number: "cento‖e‖um") holds several
            // words separated by END_WORD; end-stress promotion belongs to its
            // LAST word, exactly as if they had been typed as separate words.
            // Promoting over the whole buffer found "cento"'s existing stress and
            // did nothing, so pt "101" read `sˈẽŋtʊ i ũŋ` while "cento e um" gave
            // `… ˈũŋ`.
            let phonemes = &mut entries[i].phonemes;
            match phonemes.iter().rposition(|&c| c == crate::phoneme::PHON_END_WORD) {
                Some(sep) => {
                    let mut tail = phonemes[sep + 1..].to_vec();
                    promote_strend_stress(
                        &mut tail,
                        phdata,
                        dict_flags,
                        is_last_word,
                        following_all_unstressed,
                    );
                    phonemes.truncate(sep + 1);
                    phonemes.extend(tail);
                }
                None => promote_strend_stress(
                    phonemes,
                    phdata,
                    dict_flags,
                    is_last_word,
                    following_all_unstressed,
                ),
            }
            // C runs `ApplySpecialAttribute2` after the end-of-clause stress
            // change, so `del` (`$strend $alt2`) reads `d'el`, not `d'El`.
            if alt_attributes
                && dict_flags & (crate::dictionary::FLAG_ALT_TRANS | crate::dictionary::FLAG_ALT2_TRANS) != 0
            {
                let alt2 = dict_flags & crate::dictionary::FLAG_ALT2_TRANS != 0;
                apply_special_attribute2(&mut entries[i].phonemes, phdata, alt2);
            }
        }

        if primary_bcp47_subtag(lang) == "en" {
            apply_en_wh_clause_initial_secondary(entries, phdata);
        }
        // Tone languages: default tones + sandhi, across the whole clause.
        apply_tone_sandhi(entries, phdata, lang);
    }

    fn apply_en_wh_clause_initial_secondary(
        entries: &mut [TranslateEntry],
        phdata: &PhonemeData,
    ) {
        const PHON_STRESS_2: u8 = 4;
        const PHON_STRESS_P: u8 = 6;
        const PHON_STRESS_P2: u8 = 7;
        const WH: &[&str] = &[
            "when", "where", "what", "who", "why", "how", "which", "while",
        ];

        let word_ix: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == TranslateEntryKind::Word && !e.phonemes.is_empty())
            .map(|(i, _)| i)
            .collect();
        if word_ix.len() < 2 {
            return;
        }
        let i0 = word_ix[0];
        let i1 = word_ix[1];
        let Some(w) = entries[i0].word_lower.as_deref() else {
            return;
        };
        if !WH.iter().any(|&kw| kw == w) {
            return;
        }
        if !entries[i1]
            .phonemes
            .iter()
            .any(|&c| c == PHON_STRESS_P || c == PHON_STRESS_P2)
        {
            return;
        }
        let ph = &entries[i0].phonemes;
        let Some(fv) = ph
            .iter()
            .position(|&c| phdata.get(c).map(|p| p.typ == 2).unwrap_or(false))
        else {
            return;
        };
        if ph[..fv].iter().any(|&c| matches!(c, 4 | 5 | 6 | 7)) {
            return;
        }
        let mut new_ph = ph.to_vec();
        new_ph.insert(fv, PHON_STRESS_2);
        entries[i0].phonemes = new_ph;
    }

    let clause_boundaries: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == TranslateEntryKind::ClauseBoundary)
        .map(|(i, _)| i)
        .collect();

    let lang_for_promo = lang;
    if clause_boundaries.is_empty() {
        promote_clause(entries, phdata, lang_for_promo, outer_continues);
    } else {
        let mut prev_end = 0usize;
        let mut boundaries_with_end = clause_boundaries.clone();
        boundaries_with_end.push(entries.len());
        for &bound in &boundaries_with_end {
            let slice_end = if bound < entries.len() { bound } else { entries.len() };
            if slice_end > prev_end {
                promote_clause(&mut entries[prev_end..slice_end], phdata, lang_for_promo, outer_continues);
            }
            prev_end = if bound < entries.len() { bound + 1 } else { entries.len() };
        }
    }
}

/// English “linking ɹ” after **or** before a vowel-initial word (same condition as IPA post-pass).
pub(crate) fn apply_en_or_linking_r(entries: &mut [TranslateEntry], phdata: &PhonemeData) {
    let r_code = phdata.lookup_phoneme("r");
    if r_code == 0 {
        return;
    }

    fn next_word_vowel_initial(entries: &[TranslateEntry], from: usize, phdata: &PhonemeData) -> bool {
        entries[from + 1..]
            .iter()
            .find(|e| e.kind == TranslateEntryKind::Word && !e.phonemes.is_empty())
            .map(|e| {
                e.phonemes
                    .iter()
                    .find(|&&c| c > 8 && c != 15)
                    .and_then(|&c| phdata.get(c))
                    .map(|ph| ph.typ == 2)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    for i in 0..entries.len() {
        if entries[i].kind != TranslateEntryKind::Word {
            continue;
        }
        if entries[i].word_lower.as_deref() != Some("or") {
            continue;
        }
        if !next_word_vowel_initial(entries, i, phdata) {
            continue;
        }
        while entries[i].phonemes.last() == Some(&0) {
            entries[i].phonemes.pop();
        }
        entries[i].phonemes.push(r_code);
        entries[i].phonemes.push(0);
    }
}

/// Top-level text translator.
///
/// Create with [`Translator::new_default`] for the most common case.
/// Use [`Translator::text_to_ipa`] or [`Translator::translate_to_codes`].
pub struct Translator {
    /// Language and speech-rate configuration.
    pub options: LangOptions,
    /// Resolved espeak-ng data directory.
    data_dir: PathBuf,
}

impl Translator {
    /// Create a new translator for the given language.
    ///
    /// `data_dir` is the path to the espeak-ng data directory.
    /// If `None`, defaults to `/usr/share/espeak-ng-data`.
    pub fn new(lang: &str, data_dir: Option<&Path>) -> Result<Self> {
        let dir = data_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(default_data_dir()));

        // A `+variant` suffix (`en+f3`) only affects the acoustic voice; the
        // base tag selects the language data used for text→phonemes.
        let (base, _variant) = split_voice_variant(lang);
        let mut options = LangOptions::for_lang(base);
        // Honour the voice's `dictrules` directive (Brazilian Portuguese etc.).
        options.dict_condition = crate::voices::voice_dict_condition(&dir, base);
        options.bracket_pause = crate::voices::voice_bracket_pause(&dir, base);
        Ok(Translator { options, data_dir: dir })
    }

    /// Create with default data directory.
    pub fn new_default(lang: &str) -> Result<Self> {
        Self::new(lang, None)
    }

    /// Split `text` into clauses at sentence / phrase boundaries.
    ///
    /// Simplified version of `ReadClause()` in readclause.c.
    pub fn read_clauses(&self, text: &str) -> Result<Vec<Clause>> {
        // Simple split on sentence-ending punctuation.
        let mut clauses = Vec::new();
        let mut current = String::new();

        for c in text.chars() {
            match c {
                '.' | '!' | '?' => {
                    current.push(c);
                    let intonation = match c {
                        '!' => Intonation::Exclamation,
                        '?' => Intonation::Question,
                        _   => Intonation::FullStop,
                    };
                    let text_trim = current.trim().to_string();
                    if !text_trim.is_empty() {
                        clauses.push(Clause {
                            text: text_trim,
                            intonation,
                            clause_type: ClauseType::Sentence,
                            pause_ms: 400,
                        });
                    }
                    current = String::new();
                }
                ',' | ';' | ':' => {
                    current.push(c);
                    // Commas/semicolons continue within the same clause.
                }
                _ => { current.push(c); }
            }
        }

        // Remainder (no final punctuation)
        let text_trim = current.trim().to_string();
        if !text_trim.is_empty() {
            clauses.push(Clause {
                text: text_trim,
                intonation: Intonation::None,
                clause_type: ClauseType::Eof,
                pause_ms: 0,
            });
        }

        if clauses.is_empty() {
            clauses.push(Clause {
                text: text.trim().to_string(),
                intonation: Intonation::None,
                clause_type: ClauseType::Eof,
                pause_ms: 0,
            });
        }

        Ok(clauses)
    }

    /// High-level convenience: translate free text to an IPA string.
    ///
    /// Equivalent to running:
    ///   `espeak-ng -v <lang> -q --ipa <text>`
    ///
    /// This implementation handles plain text (no SSML) and performs:
    ///   1. Tokenization into words / punctuation
    ///   2. Dictionary lookup + rule-based translation per word
    ///   3. Phoneme code → IPA string rendering
    pub fn text_to_ipa(&self, text: &str) -> Result<String> {
        self.text_to_ipa_with_options(text, false, false, false, true)
    }

    /// Each source word with the phonemes it was rendered as — the Rust form of
    /// upstream's proposed `espeak_TextToWordPhonemePairs` (PR #2533).
    ///
    /// The phonemes come from the *same* rendering path as
    /// [`text_to_ipa`](Self::text_to_ipa), so joining them with single spaces
    /// reproduces that output; the alignment is taken from the pipeline rather
    /// than re-derived, which is what makes the awkward cases right — a number
    /// expands to several spoken words (`2.5` → `tˈuː pɔɪnt fˈaɪv`), an
    /// abbreviation to a different word, and a multi-word dictionary entry to
    /// one pronunciation unit.
    ///
    /// `offset`/`length` are byte offsets into `text`, located by searching
    /// forward for the token, so they are exact for text the tokenizer did not
    /// rewrite and `length == 0` where it did (a merged currency amount, a
    /// substituted abbreviation).
    ///
    /// ```no_run
    /// let tr = espeak_ng::Translator::new("en-us", None).unwrap();
    /// for w in tr.text_to_ipa_words("Version 2.5").unwrap() {
    ///     println!("{} -> {}", w.word, w.phonemes);
    /// }
    /// ```
    pub fn text_to_ipa_words(&self, text: &str) -> Result<Vec<WordPhonemes>> {
        let mut track: Vec<TokenCodes> = Vec::new();
        let codes = self.translate_to_codes_tracked(text, true, Some(&mut track))?;
        let mut phdata = PhonemeData::load(&self.data_dir)?;
        select_phoneme_table(&mut phdata, &self.data_dir, &self.options.lang)?;
        let render = |slice: &[PhonemeCode]| {
            phoneme_ipa::codes_to_ipa_opts(
                slice,
                &phdata,
                &self.data_dir,
                &self.options.lang,
                false,
                self.options.ipa_tie,
                self.options.ipa_separator,
            )
        };
        // The flat rendering is authoritative: where the per-word counts add up
        // to it, the words are cut straight out of it.
        let flat = render(&codes);
        let pieces: Vec<&str> = flat.split_whitespace().collect();
        let rendered: Vec<String> = track
            .iter()
            .map(|t| render(&codes[t.codes.clone()]).trim().to_string())
            .collect();
        let counts: Vec<usize> = rendered.iter().map(|r| r.split_whitespace().count()).collect();
        let exact = counts.iter().sum::<usize>() == pieces.len();

        let mut out = Vec::new();
        let mut taken = 0;
        let mut search_from = 0;
        for (t, (phon, n)) in track.iter().zip(rendered.iter().zip(&counts)) {
            let phonemes = if exact {
                pieces[taken..taken + n].join(" ")
            } else {
                phon.clone()
            };
            taken += n;
            if phonemes.is_empty() {
                continue; // punctuation, or a token that says nothing
            }
            let (offset, length) = find_token_span(text, &t.text, search_from);
            search_from = offset + length;
            out.push(WordPhonemes { word: t.text.clone(), offset, length, phonemes });
        }
        Ok(out)
    }

    /// The phonemes this language's dictionary gives for a *character-name* key
    /// (`_@` → "at sign"), or `None` when it has no entry.
    ///
    /// C's cross-language fallback for a character the current language cannot
    /// name is `LookupCharName` in the default voice, which reads exactly this
    /// key — so `he @` is "at sign", not the short reading of `@` ("at") that
    /// re-translating the character itself would give.
    pub(crate) fn char_name_phonemes(&self, c: char) -> Option<Vec<u8>> {
        let dict_stem = resolve_dict_stem(&self.data_dir, &self.options.lang)?;
        let dict_path = dict_path(&self.data_dir, &dict_stem)?;
        let dict_bytes = std::fs::read(&dict_path).ok()?;
        let mut dict = Dictionary::from_bytes(&dict_stem, dict_bytes).ok()?;
        dict.dict_condition = self.options.dict_condition;
        let mut phdata = PhonemeData::load(&self.data_dir).ok()?;
        select_phoneme_table(&mut phdata, &self.data_dir, &self.options.lang).ok()?;
        let stress_opts = StressOpts::for_lang_in(&self.options.lang, &self.data_dir);
        // A letter is named as `LookupLetter` does, ending in the accent tables.
        if c.is_alphabetic() {
            let mut name = letter_name(c, &dict, &phdata, &stress_opts, &self.options);
            if name.is_empty() {
                return None;
            }
            name.push(0);
            return Some(name);
        }
        // The `_<char>` name first, then the character's own entry — the order
        // `LookupCharName` uses, and what makes `_@` "at sign" while `#`, which
        // has no `_#` entry, still reads "hash".
        for key in [format!("_{c}"), c.to_string()] {
            let expect = PosExpect { no_replace: true, ..PosExpect::default() };
            let wr = word_to_phonemes_inner(
                &key, &dict, &phdata, &stress_opts, &self.options, 0, expect,
            );
            if wr.phonemes.iter().any(|&b| b != 0) {
                return Some(wr.phonemes);
            }
        }
        None
    }

    /// What C's `LookupLetter` makes of a symbol this language cannot name, when
    /// English is asked to speak it inside running text: the symbol itself, or
    /// the first word of its replacement text, and `None` when English's own
    /// entry is missing or that word is not in its list.
    pub(crate) fn symbol_switch_word(&self, c: char, by_name: bool) -> Option<String> {
        let dict_stem = resolve_dict_stem(&self.data_dir, &self.options.lang)?;
        let dict_bytes = std::fs::read(dict_path(&self.data_dir, &dict_stem)?).ok()?;
        let mut dict = Dictionary::from_bytes(&dict_stem, dict_bytes).ok()?;
        dict.dict_condition = self.options.dict_condition;
        let ctx = LookupCtx {
            lookup_symbol: true,
            dict_condition: self.options.dict_condition,
            ..Default::default()
        };
        if by_name && lookup(&dict, &format!("_{c}"), &ctx).is_some_and(|r| r.flags1.found()) {
            return Some(format!("_{c}"));
        }
        let entry = lookup(&dict, &c.to_string(), &ctx).filter(|r| r.flags1.found())?;
        if !entry.flags1.textmode() {
            return Some(c.to_string());
        }
        let text = String::from_utf8_lossy(&entry.phonemes).into_owned();
        let words: Vec<&str> = text.trim_end_matches('\0').split_whitespace().collect();
        let listed = lookup(&dict, words.first()?, &LookupCtx::default()).is_some();
        listed.then(|| if by_name { format!("_{c}") } else { words.join(" ") })
    }

    /// Translate free text to IPA and report how the last clause ended.
    ///
    /// The Rust form of upstream's `espeak_TextToPhonemesWithTerminator`
    /// (added in 1.53.0): the phoneme string alone doesn't say whether the text
    /// ended a sentence, asked a question, or simply ran out, which a caller
    /// driving a phonemizer or a prosody model needs.
    ///
    /// ```rust,no_run
    /// # use espeak_ng::translate::{Translator, ClauseTerminator};
    /// let t = Translator::new("en", None)?;
    /// let (ipa, end) = t.text_to_ipa_with_terminator("Hello. Are you there?")?;
    /// assert_eq!(end, ClauseTerminator::Question);
    /// assert!(end.is_sentence());
    /// # let _ = ipa;
    /// # Ok::<(), espeak_ng::Error>(())
    /// ```
    pub fn text_to_ipa_with_terminator(
        &self,
        text: &str,
    ) -> Result<(String, ClauseTerminator)> {
        let ipa = self.text_to_ipa(text)?;
        Ok((ipa, clause_terminator_of(text)))
    }

    /// Translate free text to IPA, interpreting SSML markup (`-m` mode).
    ///
    /// Equivalent to `espeak-ng -v <lang> -q --ipa -m <text>`.  See
    /// [`ssml::strip_markup`] for the supported subset.
    pub fn text_to_ipa_ssml(&self, text: &str) -> Result<String> {
        self.text_to_ipa_with_options(text, false, false, true, true)
    }

    /// `allow_switch` enables `phonSWITCH` language switching (`_^_LL` dict
    /// entries).  It is disabled for the re-translation of an already-switched
    /// word so a switch cannot recurse indefinitely.
    pub(crate) fn text_to_ipa_with_options(
        &self,
        text: &str,
        preserve_punctuation: bool,
        flatten_clauses: bool,
        markup: bool,
        allow_switch: bool,
    ) -> Result<String> {
        // SSML markup mode: parse tags/entities into language-tagged segments.
        let stripped;
        let text = if markup {
            let segments = ssml::process_markup(text);
            // `<voice xml:lang=…>` spans translate their content in another
            // language, each as its own clause.  Delegate per segment.
            if segments.iter().any(|s| s.lang.is_some()) {
                let mut voice_cache: std::collections::HashMap<String, Translator> =
                    std::collections::HashMap::new();
                let sep = if flatten_clauses { " " } else { "\n" };
                let mut parts: Vec<String> = Vec::new();
                for seg in &segments {
                    if seg.text.trim().is_empty() {
                        continue;
                    }
                    // Choose the translator: base (self) or the voice language.
                    // The `<voice>` selector may be a language tag or a voice
                    // *name* ("French (France)"); resolve it, falling back to the
                    // base voice if it doesn't match (never error).
                    let resolved = seg
                        .lang
                        .as_deref()
                        .filter(|l| *l != self.options.lang)
                        .and_then(|l| resolve_voice_lang(l, &self.data_dir))
                        .filter(|lang| *lang != self.options.lang);
                    let translator: &Translator = match resolved {
                        Some(lang) => {
                            if !voice_cache.contains_key(&lang) {
                                if let Ok(tr) = Translator::new(&lang, Some(&self.data_dir)) {
                                    voice_cache.insert(lang.clone(), tr);
                                }
                            }
                            voice_cache.get(&lang).unwrap_or(self)
                        }
                        None => self,
                    };
                    let seg_text =
                        interpret_segment_text(&seg.text, seg.interpret, &translator.options.lang);
                    let ipa = translator.text_to_ipa_with_options(
                        &seg_text, preserve_punctuation, flatten_clauses, false, allow_switch,
                    )?;
                    if !ipa.trim().is_empty() {
                        parts.push(ipa);
                    }
                }
                return Ok(parts.join(sep));
            }
            stripped = segments
                .into_iter()
                .map(|s| interpret_segment_text(&s.text, s.interpret, &self.options.lang))
                .collect::<String>();
            stripped.as_str()
        } else {
            text
        };
        // The clause's finished phoneme-code list *is* the IPA source: C's
        // `GetTranslatedPhonemeString(espeakPHONEMES_IPA)` walks exactly the
        // list `-x` prints, so the two can't disagree about which phonemes a
        // clause contains.  Rendering per word instead — which this used to do —
        // cannot see the cross-word allophones (Spanish `ðˈias`, Ukrainian's
        // voicing assimilation) or the contextual IPA names a phoneme's program
        // selects (Italian `*` is `r` between vowels, `ɾ` elsewhere).
        let codes = self.translate_to_codes_inner(text, allow_switch)?;
        let mut phdata = PhonemeData::load(&self.data_dir)?;
        select_phoneme_table(&mut phdata, &self.data_dir, &self.options.lang)?;
        let mut ipa_out = phoneme_ipa::codes_to_ipa_opts(
            &codes,
            &phdata,
            &self.data_dir,
            &self.options.lang,
            preserve_punctuation,
            self.options.ipa_tie,
            self.options.ipa_separator,
        );
        if flatten_clauses {
            ipa_out = ipa_out.replace('\n', " ");
        }
        Ok(ipa_out)
    }

    /// Translate text into a raw phoneme-code sequence for synthesis.
    ///
    /// Uses the same token → dictionary → **clause-level stress promotion** pipeline as
    /// [`Translator::text_to_ipa`] (including English clause-initial WH secondary stress and
    /// linking **/r/** after **or** before a vowel), then flattens to phoneme bytes.
    /// event (phoneme code + stress level).  This is the intermediate
    /// representation between the dictionary/rule engine and the IPA renderer;
    /// exposing it lets the synthesizer drive waveform generation directly
    /// from espeak-ng's own acoustic data files.
    ///
    /// # Phoneme code conventions (mirroring synthesize.h)
    /// | Code | Meaning                       |
    /// |------|-------------------------------|
    /// | 0    | silence / pause               |
    /// | 1–7  | stress markers                |
    /// | 9    | explicit pause                |
    /// | 12   | length mark (:)               |
    /// | 15   | word boundary (||)            |
    /// | 35+  | actual phoneme                |
    pub fn translate_to_codes(&self, text: &str) -> Result<Vec<PhonemeCode>> {
        self.translate_to_codes_inner(text, true)
    }

    /// [`translate_to_codes`](Self::translate_to_codes) with `phonSWITCH`
    /// language switching selectable — a word reached *through* a switch is
    /// translated with `allow_switch = false`, so a switch can't recurse.
    pub(crate) fn translate_to_codes_inner(
        &self,
        text: &str,
        allow_switch: bool,
    ) -> Result<Vec<PhonemeCode>> {
        self.translate_to_codes_tracked(text, allow_switch, None)
    }

    /// [`translate_to_codes_inner`](Self::translate_to_codes_inner) with an
    /// optional record of **which source token produced which codes**, which is
    /// what the word-level API ([`text_to_ipa_words`](Self::text_to_ipa_words))
    /// needs: the tokens have been through fifteen rewriting passes by the time
    /// they are translated, so the mapping can only be taken from the pipeline
    /// itself.
    pub(crate) fn translate_to_codes_tracked(
        &self,
        text: &str,
        allow_switch: bool,
        track: Option<&mut Vec<TokenCodes>>,
    ) -> Result<Vec<PhonemeCode>> {
        let lang = &self.options.lang;
        // An unknown voice is a *voice* error, not an unimplemented feature:
        // `-v alex` used to report "not yet implemented: … dict not found",
        // which says nothing about the real problem (upstream #2469, where C
        // prints "Unknown phoneme table: ''" and then crashes).
        let dict_stem = resolve_dict_stem(&self.data_dir, lang)
            .ok_or_else(|| Error::VoiceNotFound(lang.to_string()))?;
        let dict_path = dict_path(&self.data_dir, &dict_stem)
            .ok_or_else(|| Error::VoiceNotFound(lang.to_string()))?;
        let phontab_path = self.data_dir.join("phontab");

        let dict_bytes = std::fs::read(&dict_path).map_err(Error::Io)?;
        let mut dict = Dictionary::from_bytes(&dict_stem, dict_bytes)?;
        dict.dict_condition = self.options.dict_condition;

        if !phontab_path.exists() {
            return Err(Error::NotImplemented("translate_to_codes: phontab not found"));
        }
        let mut phdata = PhonemeData::load(&self.data_dir)?;
        // The table is usually named after the language; a few voices redirect
        // it (`phonemes <name>`, e.g. Norwegian Bokmål `nb` → `no`).
        select_phoneme_table(&mut phdata, &self.data_dir, lang)?;
        let stress_opts = StressOpts::for_lang_in(lang, &self.data_dir);

        // Capital-letter indication before a capitalised word (C `option_capitals`):
        //   `-k 1` → the `phonCAPITAL` *sound* (short pause + capital phoneme),
        //   `-k 2` → the spoken "capital" announcement (dict `_cap`).
        // Empty when disabled.
        let cap_codes: Vec<u8> = match self.options.capitals {
            1 => vec![crate::phoneme::PHON_PAUSE_SHORT, crate::phoneme::PHON_CAPITAL],
            2 => word_to_phonemes("_cap", &dict, &phdata, &stress_opts, &self.options).phonemes,
            _ => Vec::new(),
        };

        let text = normalize_apostrophes(text);
        let text = schwa_for_article_apostrophe(&text, lang);
        let number_grammar = with_dict_ordinal_suffixes(&self.options.number_grammar, &text, &dict);
        let mut tokens = split_overlong_clauses(tokenize_opts(&text, &number_grammar));
        split_unrecognised_ordinals(&mut tokens, &dict, &number_grammar);
        merge_digit_follow(&mut tokens, &dict);
        expand_dotted_phrases(&mut tokens, &dict, lang);
        suppress_abbreviation_periods(&mut tokens);
        apply_time_reading(&mut tokens, lang);
        apply_dash_reading(&mut tokens, lang);
        apply_decades(&mut tokens, lang);
        apply_dimensions(&mut tokens, lang);
        apply_space_grouping(&mut tokens, &self.options.number_grammar);
        apply_fractions(&mut tokens, lang);
        apply_exponents(&mut tokens, lang);
        let spelled = split_abandoned_words(&mut tokens, &dict, &self.options);
        let english: std::cell::OnceCell<Option<Translator>> = std::cell::OnceCell::new();
        let english_word = |c: char, by_name: bool| {
            english
                .get_or_init(|| Translator::new("en", Some(&self.data_dir)).ok())
                .as_ref()
                .and_then(|en| en.symbol_switch_word(c, by_name))
        };
        let alts = letters::AltLangs::new(&self.data_dir);
        let mut entries = build_translate_entries(
            &tokens,
            &dict,
            &phdata,
            &stress_opts,
            &self.options,
            &english_word,
            &alts,
            &spelled,
        );
        let outer_continues = self.options.switched && !self.options.switched_last;
        promote_translate_entries(&mut entries, &phdata, lang, outer_continues);
        if primary_bcp47_subtag(lang) == "en" {
            apply_en_or_linking_r(&mut entries, &phdata);
        }

        debug_assert_eq!(tokens.len(), entries.len());
        let mut codes: Vec<PhonemeCode> = Vec::new();
        // Parallel to `codes`: did this phoneme come from an explicit
        // translation (a dictionary entry or a number key) rather than the
        // letter-to-sound rules?  Turkish/Bashkir/Tatar suppress vowel reduction
        // then (`isTranslationGiven`).
        let mut from_dict: Vec<bool> = Vec::new();
        // `phonSWITCH` entries carry `[phonSWITCH, <lang bytes…>]` rather than
        // real phonemes.  Re-translate the word with the target language and
        // wrap its codes in switch markers, so `-x` prints `(en)h@l'oU(ru)`
        // instead of leaking the raw marker — the same thing `text_to_ipa`
        // already does.  Translators are cached per target within the clause.
        let mut switch_cache: std::collections::HashMap<String, Translator> =
            std::collections::HashMap::new();
        let mut open_switch: Option<String> = None;
        let mut stream_tail: Option<letters::StreamTail> = None;

        // Parallel to `codes` when tracking: the token each code came from.
        // Filled one iteration behind — every `continue` in the loop below would
        // otherwise need its own bookkeeping.
        let mut owners: Vec<usize> = Vec::new();
        let tracking = track.is_some();
        let mut owner_now = 0usize;
        // C's `prepause_timeout`: after a `$pause` word, the next two words
        // may not pause again.  Reset at each clause.
        let mut prepause_timeout = 0u8;

        for (token_ix, (token, entry)) in tokens.iter().zip(entries.iter()).enumerate() {
            if tracking {
                owners.resize(codes.len(), owner_now);
                owner_now = token_ix;
            }
            // C's `SFLAG_DICTIONARY`: the *pronunciation* came from the list,
            // not merely some flags — a flags-only `$` entry still leaves the
            // rules to pronounce the word.
            let translation_given = entry.found_in_list;

            if matches!(entry.kind, TranslateEntryKind::LangSwitch) && allow_switch {
                let target = switch_target(&entry.phonemes);
                let word = entry.word_lower.as_deref().unwrap_or("");
                if !word.is_empty() {
                    if !switch_cache.contains_key(&target) {
                        if let Ok(mut tr) = Translator::new(&target, Some(&self.data_dir)) {
                            tr.options.switched = true;
                            switch_cache.insert(target.clone(), tr);
                        }
                    }
                    let last_in_clause = !tokens[token_ix + 1..]
                        .iter()
                        .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
                        .any(|t| matches!(t, Token::Word(_) | Token::Number(_) | Token::InlinePhonemes(_)));
                    if let Some(tr) = switch_cache.get_mut(&target) {
                        tr.options.switched_last = last_in_clause;
                    }
                    // A `_<char>` key is a character *name* to look up, not
                    // text to translate (`_@` tokenises as two symbols).
                    // A one-letter word is spelled, not read: C marks it
                    // `FLAG_TRANSLATOR2` and says the letter's name.
                    let lone = |text: &str| {
                        let mut it = text.chars();
                        it.next().filter(|_| it.next().is_none())
                    };
                    let char_name = word
                        .strip_prefix('_')
                        .and_then(lone)
                        .or_else(|| {
                            lone(word)
                                .filter(|c| c.is_alphabetic())
                                .and_then(|c| c.to_lowercase().next())
                        });
                    // `translate_to_codes_inner` with switching disabled: a
                    // switched word may not switch again.
                    let sw = match char_name {
                        Some(c) => switch_cache.get(&target).and_then(|tr| {
                            tr.char_name_phonemes(c).map(|ph| {
                                ph.iter()
                                    .filter(|&&b| b != 0)
                                    .map(|&b| PhonemeCode { code: b, ..Default::default() })
                                    .collect::<Vec<_>>()
                            })
                        }),
                        None => switch_cache
                            .get(&target)
                            .and_then(|tr| tr.translate_to_codes_inner(word, false).ok()),
                    };
                    // A switch whose target says nothing leaves no markers:
                    // they would print as an empty `(en)(ru)` pair.
                    let sw = sw.filter(|codes| {
                        codes.iter().any(|c| !c.is_boundary && c.marker.is_none())
                    });
                    if let Some(sw) = sw {
                        let hyphen_joined = token_ix > 0 && matches!(tokens[token_ix - 1], Token::WordJoin);
                        if hyphen_joined {
                            if codes.last().is_some_and(|c| c.is_boundary && c.marker.is_none()) {
                                codes.pop();
                            }
                        }
                        let name =entry.phonemes.iter().position(|&b| b == 0).map_or(&[][..], |z| &entry.phonemes[z + 1..]);
                        if !name.is_empty() && open_switch.take().is_some() {
                            codes.push(PhonemeCode {
                                marker: Some(CodeMarker::LangSwitch(self.options.lang.clone())),
                                ..Default::default()
                            });
                        }
                        let word_open = codes
                            .iter()
                            .rposition(|k| k.marker.is_none())
                            .is_some_and(|i| !codes[i].is_boundary);
                        if word_open && open_switch.is_none() && !hyphen_joined {
                            codes.push(PhonemeCode { code: 15, is_boundary: true, ..Default::default() });
                        }
                        codes.extend(name.iter().map(|&b| PhonemeCode { code: b, ..Default::default() }));
                        if open_switch.as_deref() != Some(target.as_str()) {
                            codes.push(PhonemeCode {
                                marker: Some(CodeMarker::LangSwitch(target.clone())),
                                ..Default::default()
                            });
                            open_switch = Some(target.clone());
                        }
                        for c in sw {
                            // Drop the inner clause boundary; this is one word
                            // inside the outer clause.
                            if c.is_boundary && c.code == 0 {
                                continue;
                            }
                            codes.push(c);
                        }
                        codes.push(PhonemeCode { code: 15, is_boundary: true, ..Default::default() });
                        from_dict.resize(codes.len(), false);
                        continue;
                    }
                }
            }
            // An open switch stays open across the separators between words, so
            // a run of switched words shares one `(en)…(ru)` group like
            // upstream's; it closes at the next entry that actually speaks, at a
            // clause boundary — C ends the clause's phoneme list there, so
            // `ru "a , b"` is `(en)'eI(ru) (en)b'i:(ru)`, two groups, not one
            // spanning the comma — and at a bracket, whose pause belongs to the
            // base language (`(en)'eI(ru)_:_: (en)b'i:(ru)`).
            let bracket_here = matches!(token, Token::Punctuation(c) if is_bracket(*c))
                && entry.phonemes.is_empty();
            if open_switch.is_some()
                && (!entry.phonemes.is_empty()
                    || bracket_here
                    || entry.kind == TranslateEntryKind::ClauseBoundary)
            {
                open_switch = None;
                codes.push(PhonemeCode {
                    marker: Some(CodeMarker::LangSwitch(self.options.lang.clone())),
                    ..Default::default()
                });
            }

            match token {
                // An embedded command becomes a marker in the code stream, so
                // the synthesizer can change rate/pitch/amplitude *from here*
                // instead of for the whole utterance.
                Token::Embedded(cmd) => {
                    codes.push(PhonemeCode {
                        marker: Some(CodeMarker::Embedded(*cmd)),
                        ..Default::default()
                    });
                    from_dict.resize(codes.len(), false);
                }
                // Inline phonemes (`[[…]]`) carry pre-parsed phoneme codes and
                // spoken symbols (`&`→"and", `%`→"percent") carry name phonemes
                // in their entry (kind `Word`); emit them like any other word so
                // the `-x` / synthesis path renders them (parity with the
                // `text_to_ipa` / `--ipa` path).  Non-spoken punctuation has an
                // empty entry, so emitting it is a harmless no-op.
                Token::Word(_)
                | Token::Number(_)
                | Token::InlinePhonemes(_)
                | Token::Operator(_)
                | Token::Punctuation(_) => {
                    if let Token::Punctuation(c) | Token::Operator(c) = token {
                        // A bracket or quote is not spoken but does pause:
                        // `langopts.param[LOPT_BRACKET_PAUSE]` is 4, and C's
                        // pre-pause loop spends that as two `phonPAUSE`
                        // (`en "a ( b"` → `a#_:_: b'i:`).  The announced-bracket
                        // variant (`--punct`) is 2, i.e. one pause.  C builds the
                        // pause as the *next* word's pre-pause, so a bracket with
                        // no word after it in the clause — `(hello)` — pauses
                        // only on the opening one.
                        let word_follows = tokens[token_ix + 1..]
                            .iter()
                            .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
                            .zip(entries[token_ix + 1..].iter())
                            .any(|(_, e)| e.phonemes.iter().any(|&b| b != 0));
                        // An apostrophe's pause is C's `pre_pause_add`, which
                        // `words[0].pre_pause = 0` discards at the start of a
                        // clause ("don't add extra pause at beginning of
                        // clause") — so `'hello` has no pause while the curly
                        // `‘hello`, a real bracket, keeps its own.
                        let clause_initial_quote = *c == '\''
                            && !tokens[..token_ix]
                                .iter()
                                .rev()
                                .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
                                .zip(entries[..token_ix].iter().rev())
                                .any(|(_, e)| e.phonemes.iter().any(|&b| b != 0));
                        let spoken_before = !tokens[..token_ix]
                            .iter()
                            .rev()
                            .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
                            .zip(entries[..token_ix].iter().rev())
                            .all(|(_, e)| e.phonemes.iter().all(|&b| b == 0));
                        let pause_dash = *c == '-' && spaced_dash(&tokens, token_ix) && spoken_before;
                        if (is_bracket(*c) || pause_dash)
                            && !clause_initial_quote
                            && entry.phonemes.is_empty()
                            && word_follows
                        {
                            // The pause joins the run of phonemes that is already
                            // open, so take back the word boundary the separator
                            // before the bracket pushed and put it back after.
                            // Markers (a closing language switch) can sit after
                            // that boundary, so look past them, and a switched
                            // word contributes one of its own on top of the
                            // separator's — take every trailing boundary and put
                            // a single one back after the pause.
                            let mut closed = false;
                            while let Some(i) = codes
                                .iter()
                                .rposition(|k| k.marker.is_none())
                                .filter(|&i| {
                                    (codes[i].is_boundary || codes[i].code == crate::phoneme::PHON_END_WORD)
                                        && codes[i].clause_char.is_none()
                                })
                            {
                                codes.remove(i);
                                closed = true;
                            }
                            let (plain, announced) = self.options.bracket_pause;
                            // The pre-pause loop in C spends 2 per `phonPAUSE`.
                            let tight_after_bracket = token_ix
                                .checked_sub(1)
                                .is_some_and(|k| matches!(&tokens[k], Token::Punctuation(p) if is_bracket(*p)));
                            let pause_open = tight_after_bracket
                                && codes
                                    .iter()
                                    .rposition(|k| k.marker.is_none())
                                    .is_some_and(|i| codes[i].code == crate::phoneme::PHON_PAUSE);
                            let pauses = if pause_open {
                                0
                            } else if punct_covers(&self.options, *c) {
                                announced / 2
                            } else {
                                plain / 2
                            };
                            for _ in 0..pauses {
                                codes.push(PhonemeCode {
                                    code: crate::phoneme::PHON_PAUSE,
                                    ..Default::default()
                                });
                            }
                            if closed {
                                codes.push(PhonemeCode {
                                    code: 15, is_boundary: true, ..Default::default()
                                });
                            }
                            from_dict.resize(codes.len(), false);
                        }
                        codes.push(PhonemeCode {
                            marker: Some(CodeMarker::Punctuation(*c)),
                            ..Default::default()
                        });
                    }
                    // C flags the start of every word in the phoneme list.  A
                    // spoken symbol standing tight between two words —
                    // "cat.Dog" → "cat dot dog", "3.14" → "drei punkt vierzehn"
                    // — has no `Space` token on either side, so without this the
                    // three words ran together (`k'atd'0td'0g`).
                    // A `WordJoin` separator (a hyphen) means the two words are
                    // spoken as one, so no boundary goes between them.
                    let joined = matches!(
                        token_ix.checked_sub(1).map(|k| &tokens[k]),
                        Some(Token::WordJoin)
                    ) || (0..token_ix)
                        .rev()
                        .find(|&k| !matches!(tokens[k], Token::Space | Token::WordJoin))
                        .and_then(|k| entries.get(k))
                        .is_some_and(|e| e.no_word_gap);
                    // Does the open run end in phonemes this word should be
                    // separated from?  A run of nothing but pauses doesn't count:
                    // that is a bracket's pre-pause, which belongs to the word
                    // after it as it does in C's phoneme list (`(hello)` is
                    // `_:_:h@l'oU`, one token).  A word whose own reading merely
                    // *ends* in a pause still does (`=` is `_:'i:kw@Lz_:`).
                    let runs_on = codes
                        .iter()
                        .rev()
                        .filter(|c| c.marker.is_none())
                        .take_while(|c| {
                            !c.is_boundary && c.code != crate::phoneme::PHON_END_WORD
                        })
                        .any(|c| c.code != crate::phoneme::PHON_PAUSE);
                    if runs_on && !joined && entry.phonemes.iter().any(|&b| b != 0) {
                        codes.push(PhonemeCode {
                            code: 15, is_boundary: true, clause_char: None, marker: None,
                        });
                    }
                    // `-k`: capital indication before a capitalised word.
                    //   1 → the `phonCAPITAL` sound, which belongs to the
                    //       *word* (C puts it at the head of its phoneme list,
                    //       hence `_#X1h@l'oU` with no gap), repeated for an
                    //       all-caps multi-letter word (C: FLAG_ALL_UPPER +
                    //       IsAlpha(word[1]));
                    //   2 → the spoken announcement, a word of its own;
                    //   ≥3 → raise the pitch for this word by that many Hz
                    //       (3 means C's default 20), restored afterwards.
                    let mut capital_pitch = 0;
                    if let Token::Word(w) = token {
                        if w.chars().next().is_some_and(char::is_uppercase) {
                            if !cap_codes.is_empty() {
                                let alpha = w.chars().filter(|c| c.is_alphabetic());
                                let all_upper = alpha.clone().count() > 1
                                    && alpha.clone().all(|c| c.is_uppercase());
                                let repeats =
                                    if self.options.capitals == 1 && all_upper { 2 } else { 1 };
                                for _ in 0..repeats {
                                    for &b in &cap_codes {
                                        if b != 0 {
                                            codes.push(PhonemeCode {
                                                code: b, is_boundary: false, clause_char: None, marker: None
                                            });
                                        }
                                    }
                                }
                                if self.options.capitals != 1 {
                                    codes.push(PhonemeCode {
                                        code: 15, is_boundary: true, clause_char: None, marker: None
                                    }); // END_WORD
                                }
                            } else if self.options.capitals >= 3 {
                                capital_pitch = if self.options.capitals == 3 {
                                    20
                                } else {
                                    self.options.capitals as i32
                                };
                                codes.push(PhonemeCode {
                                    marker: Some(CodeMarker::Embedded(EmbeddedCmd {
                                        letter: 'P', value: capital_pitch, relative: 1,
                                    })),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                    // `$brk` / `$pause` put a pause *before* the word (C's
                    // `pre_pause`): one `phonPAUSE_NOLINK` for `$brk`, two
                    // `phonPAUSE` for `$pause`.  Swahili's "za" is `$u $brk`,
                    // which is where `hab'aRi_! za` gets its break.
                    if !entry.phonemes.is_empty() {
                        use crate::dictionary::{FLAG_PAUSE1, FLAG_PREPAUSE};
                        let mut pre_pause = 0;
                        prepause_timeout = prepause_timeout.saturating_sub(1);
                        let is_word = |t: &Token| {
                            matches!(t, Token::Word(_) | Token::Number(_) | Token::InlinePhonemes(_))
                        };
                        let after = tokens[token_ix + 1..]
                            .iter()
                            .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
                            .filter(|t| is_word(t))
                            .count();
                        // C drops the `$brk` pause on the last word of a clause.
                        if entry.dict_flags & FLAG_PAUSE1 != 0 && after >= 1 {
                            pre_pause = 1;
                        }
                        // C additionally skips `$pause` at either end of the
                        // clause and right after its first word, so it needs at
                        // least two words before it and one after.
                        if entry.dict_flags & FLAG_PREPAUSE != 0 {
                            let before = tokens[..token_ix]
                                .iter()
                                .rev()
                                .take_while(|t| !matches!(t, Token::ClauseBoundary(_)))
                                .filter(|t| is_word(t))
                                .count();
                            if before >= 2 && after >= 1 && prepause_timeout == 0 {
                                pre_pause = 4;
                                prepause_timeout = 3;
                            }
                        }
                        // The pause goes *before* the word separator, so it
                        // reads as part of the previous word (`hab'aRi_! za`).
                        let at = codes
                            .iter()
                            .rposition(|c| {
                                !c.is_boundary
                                    && c.marker.is_none()
                                    && c.code != crate::phoneme::PHON_END_WORD
                            })
                            .map_or(0, |i| i + 1);
                        let mut n = 0;
                        while pre_pause > 0 {
                            let code = if pre_pause > 1 {
                                pre_pause -= 2;
                                crate::phoneme::PHON_PAUSE
                            } else {
                                pre_pause -= 1;
                                crate::phoneme::PHON_PAUSE_NOLINK
                            };
                            codes.insert(
                                at + n,
                                PhonemeCode {
                                    code, is_boundary: false, clause_char: None, marker: None,
                                },
                            );
                            n += 1;
                        }
                        if n > 0 {
                            from_dict.resize(codes.len(), false);
                        }
                    }
                    if entry.kind == TranslateEntryKind::Stream {
                        letters::push_stream(&mut codes, &entry.phonemes, lang, &mut stream_tail);
                    }
                    let plain = if entry.kind == TranslateEntryKind::Stream {
                        Vec::new()
                    } else {
                        consume_word_break_syllabic(&entry.phonemes, &phdata)
                    };
                    for &b in &plain {
                        // `phonX1` is a marker, not a sound: C reads it as
                        // "double the next word's initial consonant"
                        // (`FLAG_DOUBLING`) and does *not* put it in the phoneme
                        // list.  Emitting it printed a stray `_X1` (Finnish
                        // `k'aikIlle_X1`) and spoke it.
                        if b != 0 && b != crate::phoneme::PHON_X1 {
                            codes.push(PhonemeCode {
                                code:         b,
                                is_boundary:  false,
                                clause_char:  None, marker: None
                            });
                        }
                    }
                    // The pitch raise for a capital is undone after the word,
                    // carried — as in C — on a `phonPAUSE_SHORT`, which is why
                    // `-k 20 "Hello World"` prints `h@l'oU_ w'3:ld_`.
                    if capital_pitch > 0 {
                        codes.push(PhonemeCode {
                            marker: Some(CodeMarker::Embedded(EmbeddedCmd {
                                letter: 'P', value: capital_pitch, relative: -1,
                            })),
                            ..Default::default()
                        });
                        codes.push(PhonemeCode {
                            code: crate::phoneme::PHON_PAUSE_SHORT,
                            is_boundary: false, clause_char: None, marker: None,
                        });
                    }
                    // `-g` word gap: a pause phoneme (`_`) after each word.
                    if self.options.word_gap > 0 && !entry.phonemes.is_empty() {
                        let pause = phdata.lookup_phoneme("_");
                        if pause != 0 {
                            codes.push(PhonemeCode {
                                code: pause, is_boundary: false, clause_char: None, marker: None
                            });
                        }
                    }
                    // (The language's own inter-word gap is inserted later, by
                    // `insert_word_gaps`, which needs the finished code list.)
                }
                // A hyphen-joined pair is spoken as one word: no boundary, no
                // space in the output.
                Token::WordJoin => {}
                Token::Space => {
                    // A sandhi-joined pair is one word, so the separator between
                    // its halves emits no boundary either.
                    let joined = token_ix
                        .checked_sub(1)
                        .and_then(|k| entries.get(k))
                        .is_some_and(|e| e.no_word_gap);
                    if !joined && !only_pauses_since_clause_start(&codes) {
                        codes.push(PhonemeCode {
                            code:         15,
                            is_boundary:  true,
                            clause_char:  None, marker: None
                        }); // END_WORD
                    }
                }
                Token::ClauseBoundary(punct) => {
                    prepause_timeout = 0;
                    if entry.phonemes.is_empty() {
                        codes.push(PhonemeCode {
                            code:         0,
                            is_boundary:  true,
                            clause_char:  Some(*punct), marker: None
                        }); // pause + clause
                    } else {
                        // `--punct`: the boundary became a spoken name.  It
                        // absorbed its surrounding whitespace, so bracket it with
                        // END_WORD markers to keep the name a separate word.
                        codes.push(PhonemeCode { code: 15, is_boundary: true, clause_char: None , marker: None});
                        for &b in &entry.phonemes {
                            if b != 0 {
                                codes.push(PhonemeCode {
                                    code: b, is_boundary: false, clause_char: None, marker: None
                                });
                            }
                        }
                        codes.push(PhonemeCode { code: 15, is_boundary: true, clause_char: None , marker: None});
                    }
                }
            }
            // Positions added during this iteration inherit the entry's flag.
            from_dict.resize(codes.len(), translation_given);
        }

        // A switch left open at the end of the text still closes, so the
        // output reads `(en)h@l'oU(ru)` rather than trailing off.
        if open_switch.take().is_some() {
            codes.push(PhonemeCode {
                marker: Some(CodeMarker::LangSwitch(self.options.lang.clone())),
                ..Default::default()
            });
            from_dict.resize(codes.len(), false);
        }

        // Which source token owns which codes, in *boundary* coordinates: the
        // post-passes below insert and delete phonemes, so an index into
        // `codes` would not survive them — the number of word/clause boundaries
        // before a code does, since no pass adds or removes one.
        let mut groups: Vec<(String, usize)> = Vec::new();
        if tracking {
            owners.resize(codes.len(), owner_now);
            let mut boundaries_before = Vec::with_capacity(codes.len() + 1);
            let mut n = 0usize;
            boundaries_before.push(0);
            for c in &codes {
                if c.is_boundary {
                    n += 1;
                }
                boundaries_before.push(n);
            }
            // The first *sounding* code: a token's own codes often open with the
            // word boundary the separator before it pushed, and counting that
            // boundary as "not yet passed" would start the span one word early.
            let mut first: Vec<Option<usize>> = vec![None; tokens.len()];
            for (i, &o) in owners.iter().enumerate() {
                if !codes[i].is_boundary && o < tokens.len() && first[o].is_none() {
                    first[o] = Some(i);
                }
            }
            for (ix, token) in tokens.iter().enumerate() {
                let Some(f) = first[ix] else { continue };
                let text = token_surface(token, &self.options.number_grammar);
                if !text.is_empty() {
                    groups.push((text, boundaries_before[f]));
                }
            }
        }

        // The post-passes resolve codes against *this* clause's phoneme table, so
        // they must skip anything inside a `phonSWITCH` region — those codes
        // belong to the switched language's table, and rewriting them with this
        // one turned Arabic `H` into `d`.
        let mut out: Vec<PhonemeCode> = Vec::with_capacity(codes.len());
        let mut own: Vec<PhonemeCode> = Vec::new();
        let mut own_flags: Vec<bool> = Vec::new();
        let mut in_switch = false;
        // The voice's own `replace` substitutions match the phonemes the rules
        // wrote, so they run before the phoneme programs change them (C does
        // them at the top of `MakePhonemeList`).
        let replacements: Vec<(u8, u8, u8)> = crate::voices::voice_replacements(&self.data_dir, lang)
            .into_iter()
            .filter_map(|(f, old, new)| {
                let o = phdata.lookup_phoneme(&old);
                let n = if new == "NULL" { 0 } else { phdata.lookup_phoneme(&new) };
                (o != 0).then_some((f, o, n))
            })
            .collect();
        let finish = |own: &mut Vec<PhonemeCode>,
                          own_flags: &mut Vec<bool>,
                          out: &mut Vec<PhonemeCode>| {
            if own.is_empty() {
                return;
            }
            own_flags.resize(own.len(), false);
            collapse_repeated_lengthen(own, own_flags);
            apply_voice_replacements(own, &phdata, &replacements);
            own_flags.resize(own.len(), false);
            // Before the phoneme programs run, as in C's `MakePhonemeList`.
            double_lengthened_consonants(own, own_flags, &phdata);
            resolve_virtual_phonemes_in_codes(own, &phdata, own_flags, stress_opts.reduce);
            apply_list_editing_instructions(
                own,
                &phdata,
                stress_opts.vowel_pause,
                stress_opts.word_gap,
            );
            drop_syllabic_after_vowel(own, &phdata);
            if primary_bcp47_subtag(lang) == "fr" {
                drop_unsurfaced_liaison(own, &phdata);
            }
            out.append(own);
            own_flags.clear();
        };
        for (ix, c) in codes.into_iter().enumerate() {
            if let Some(CodeMarker::LangSwitch(target)) = &c.marker {
                finish(&mut own, &mut own_flags, &mut out);
                in_switch = *target != self.options.lang;
                out.push(c);
                continue;
            }
            if in_switch {
                out.push(c);
            } else {
                own_flags.push(from_dict.get(ix).copied().unwrap_or(false));
                own.push(c);
            }
        }
        finish(&mut own, &mut own_flags, &mut out);

        // Regressive voicing assimilation runs over the whole clause, after the
        // per-word passes — C calls it once on `ph_list2` in `MakePhonemeList`.
        if stress_opts.regressive_voicing != 0 {
            set_regressive_voicing(
                &mut out,
                &phdata,
                stress_opts.regressive_voicing,
                &self.options.lang,
            );
        }
        // The inter-word pause insertion runs over the finished list.
        if self.options.word_gap == 0 {
            insert_word_gaps(&mut out, &phdata, stress_opts.vowel_pause, stress_opts.word_gap);
        }
        promote_clause_tonics(&mut out, &phdata, &self.options.lang, stress_opts.stress_flags & 0x02 != 0);

        // Boundary coordinates back into indices.  A token's codes run from its
        // own first code to the *next* token's, so the separator's boundary and
        // any pause the passes added stay with the word they follow.
        if let Some(track) = track {
            let positions: Vec<usize> = out
                .iter()
                .enumerate()
                .filter(|(_, c)| c.is_boundary)
                .map(|(i, _)| i)
                .collect();
            let start_of = |b: usize| -> usize {
                if b == 0 {
                    0
                } else {
                    positions.get(b - 1).map(|&i| i + 1).unwrap_or(out.len())
                }
            };
            let starts: Vec<usize> = groups.iter().map(|&(_, b)| start_of(b)).collect();
            for (i, (text, _)) in groups.into_iter().enumerate() {
                let start = starts[i];
                let end = starts.get(i + 1).copied().unwrap_or(out.len()).max(start);
                track.push(TokenCodes { text, codes: start..end });
            }
        }
        Ok(out)
    }
}

/// A source word and the phonemes it was rendered as
/// ([`Translator::text_to_ipa_words`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordPhonemes {
    /// The word as it reached translation.
    pub word: String,
    /// Byte offset of the word in the input text (`0` with `length == 0` when
    /// the tokenizer rewrote it and it can no longer be located).
    pub offset: usize,
    /// Its byte length in the input text, or `0` when unknown.
    pub length: usize,
    /// The IPA for this word — the slice of the clause's own rendering.
    pub phonemes: String,
}

/// Locate `needle` in `text` at or after `from`, exactly or ignoring ASCII case
/// (a capitalised sentence opener is the common case).  `length == 0` means it
/// could not be found, which happens for tokens the rewriting passes created.
fn find_token_span(text: &str, needle: &str, from: usize) -> (usize, usize) {
    let from = from.min(text.len());
    if needle.is_empty() || !text.is_char_boundary(from) {
        return (from, 0);
    }
    let hay = &text[from..];
    if let Some(ix) = hay.find(needle) {
        return (from + ix, needle.len());
    }
    let n = needle.len();
    for (i, _) in hay.char_indices() {
        if i + n <= hay.len()
            && hay.is_char_boundary(i + n)
            && hay[i..i + n].eq_ignore_ascii_case(needle)
        {
            return (from + i, n);
        }
    }
    (from, 0)
}

/// One source token and the span of phoneme codes it was translated into.
pub(crate) struct TokenCodes {
    /// The token's text as it appeared (after the number/abbreviation rewrites,
    /// which is how the tokens reach translation).
    pub(crate) text: String,
    /// Its codes in the finished clause.
    pub(crate) codes: std::ops::Range<usize>,
}

/// The text a token stands for, or `""` for one that is not part of the source
/// text (a separator or an embedded command).
fn token_surface(token: &Token, grammar: &NumberGrammar) -> String {
    match token {
        Token::Word(w) => w.clone(),
        // `NumberToken::surface` writes a decimal point; the source may have
        // used the language's own separator (`de "3,14"`).
        Token::Number(NumberToken::Decimal { integer, fractional, .. }) => {
            format!("{integer}{}{fractional}", grammar.decimal_separator)
        }
        Token::Number(n) => n.surface(),
        Token::InlinePhonemes(p) => format!("[[{p}]]"),
        Token::Operator(c) | Token::Punctuation(c) | Token::ClauseBoundary(c) => c.to_string(),
        Token::Space | Token::WordJoin | Token::Embedded(_) => String::new(),
    }
}

/// The pause phoneme C puts between a word ending in a phoneme of type
/// `prev_type` and the next word (`next_type`, `PH_PAUSE` at a clause end), or 0
/// for none.  `next_stressed` is the next word's first vowel carrying primary
/// stress.
fn word_gap_phoneme(
    vowel_pause: u32,
    word_gap: u8,
    prev_type: u8,
    next_type: u8,
    next_stressed: bool,
) -> u8 {
    use crate::phoneme::{PHON_PAUSE, PHON_PAUSE_LONG, PHON_PAUSE_NOLINK, PHON_PAUSE_SHORT,
                         PHON_PAUSE_VSHORT};
    const PH_VOWEL: u8 = 2;
    const PH_PAUSE: u8 = 0;
    // C `pause_phonemes[8]`; index 5 is a glottal stop, which no shipped
    // language selects through `word_gap`.
    const PAUSE_PHONEMES: [u8; 8] = [
        0, PHON_PAUSE_VSHORT, PHON_PAUSE_SHORT, PHON_PAUSE, PHON_PAUSE_LONG,
        0, PHON_PAUSE_LONG, PHON_PAUSE_LONG,
    ];
    let (pt, nt) = (prev_type, next_type);
    let mut insert = 0u8;
    if vowel_pause != 0 && pt != PH_PAUSE {
        if pt != PH_VOWEL && vowel_pause & 0x200 != 0 {
            insert = PHON_PAUSE_NOLINK;
        }
        if nt == PH_VOWEL {
            match vowel_pause & 0x0c {
                0 => {}
                0xc => insert = PHON_PAUSE_NOLINK,
                _ => insert = PHON_PAUSE_VSHORT,
            }
            if pt == PH_VOWEL {
                match vowel_pause & 0x03 {
                    0 => {}
                    2 => insert = PHON_PAUSE_SHORT,
                    _ => insert = PHON_PAUSE_VSHORT,
                }
            }
            if vowel_pause & 0x100 != 0 && next_stressed {
                insert = PHON_PAUSE_SHORT;
            }
        }
    }
    let x = (word_gap & 7) as usize;
    if x != 0 && (x > 1 || (insert != PHON_PAUSE_SHORT && insert != PHON_PAUSE_NOLINK)) {
        insert = PAUSE_PHONEMES[x];
    }
    insert
}

/// Insert the inter-word pause phonemes a language asks for — the
/// `(plist3+1)->sourceix != 0` block of `MakePhonemeList` (phonemelist.c).
///
/// `vowel_pause` is `langopts.vowel_pause`: bit 0-1 chooses the pause between
/// two vowels across a word boundary (Italian's `1` puts a very short one
/// between "casa" and "è"), bits 2-3 one before *any* vowel-initial word,
/// bit 8 one before a *stressed* vowel, bit 9 one after a word ending in a
/// consonant.  `word_gap & 7` then selects a minimum gap from
/// `pause_phonemes[]` for every boundary.
fn insert_word_gaps(
    codes: &mut Vec<PhonemeCode>,
    phdata: &PhonemeData,
    vowel_pause: u32,
    word_gap: u8,
) {
    use crate::phoneme::PHON_END_WORD;
    const PH_PAUSE: u8 = 0;
    if vowel_pause == 0 && word_gap & 7 == 0 {
        return;
    }

    let is_sound = |c: &PhonemeCode| {
        c.marker.is_none()
            && !c.is_boundary
            && c.code > 8
            && c.code != PHON_END_WORD
            && c.code != crate::phoneme::PHON_LENGTHEN
    };
    let real: Vec<usize> = codes
        .iter()
        .enumerate()
        .filter(|(_, c)| is_sound(c))
        .map(|(i, _)| i)
        .collect();
    let (level, _) = stress_levels(codes, phdata);
    let starts = word_starts(codes, &real);

    let typ = |code: u8| phdata.get(code).map(|p| p.typ).unwrap_or(PH_PAUSE);
    let mut inserts: Vec<(usize, u8)> = Vec::new();
    // Every word start is a boundary, and so is the end of a clause: C's phoneme
    // list ends with a pause phoneme that carries `sourceix`, which is why
    // Chinese gets a trailing `_|` after its last word as well.
    for n in 1..=real.len() {
        let at_clause_end = n == real.len()
            || codes[real[n - 1] + 1..real[n]]
                .iter()
                .any(|c| c.is_boundary && c.code == 0);
        if !at_clause_end && !starts[n] {
            continue; // not a word boundary
        }
        let prev = codes[real[n - 1]].code;
        let next = if n < real.len() && !at_clause_end { codes[real[n]].code } else { 0 };
        let next_stressed = n < real.len() && level.get(real[n]).is_some_and(|&l| l >= 4);
        let insert = word_gap_phoneme(vowel_pause, word_gap, typ(prev), typ(next), next_stressed);
        if insert != 0 && phdata.get(insert).is_some() {
            // The gap belongs to the *previous* word — upstream prints
            // `bUondZ'O@-*no_| a`, with the pause before the word separator.
            let mut at = real[n - 1] + 1;
            while codes.get(at).is_some_and(|c| c.code == crate::phoneme::PHON_LENGTHEN && !c.is_boundary) {
                at += 1;
            }
            inserts.push((at, insert));
        }
    }

    for (at, code) in inserts.into_iter().rev() {
        codes.insert(
            at,
            PhonemeCode { code, is_boundary: false, clause_char: None, marker: None },
        );
    }
}

/// Apply the voice file's `replace <flags> <old> <new>` substitutions to the
/// finished code list — the `replace_phonemes[]` loop at the top of
/// `MakePhonemeList` (phonemelist.c).
///
/// `flags` bit 0 restricts the substitution to a word end, bit 1 skips it in a
/// stressed syllable, bit 2 restricts it to a word start; a `NULL` replacement
/// deletes the phoneme.  Fourteen shipped voices use this — Russian's
/// `replace 03 a a#` is what turns a word-final unstressed `a` into `a#`.
fn apply_voice_replacements(
    codes: &mut Vec<PhonemeCode>,
    phdata: &PhonemeData,
    rules: &[(u8, u8, u8)],
) {
    if rules.is_empty() {
        return;
    }
    let is_sound = |c: &PhonemeCode| {
        c.marker.is_none() && !c.is_boundary && c.code > 8 && c.code != crate::phoneme::PHON_END_WORD
    };
    let real: Vec<usize> = codes
        .iter()
        .enumerate()
        .filter(|(_, c)| is_sound(c))
        .map(|(i, _)| i)
        .collect();
    let (level, _) = stress_levels(codes, phdata);
    let starts = word_starts(codes, &real);

    let mut deleted: Vec<usize> = Vec::new();
    for (n, &i) in real.iter().enumerate() {
        // C keeps `stresslevel` on consonants too (it copies the following
        // vowel's); approximate with the next vowel in the same word.
        let stress = if level[i] != NO_STRESS_LEVEL {
            level[i]
        } else {
            real[n + 1..]
                .iter()
                .enumerate()
                .take_while(|(k, _)| !starts[n + 1 + k])
                .find_map(|(_, &j)| (level[j] != NO_STRESS_LEVEL).then(|| level[j]))
                .unwrap_or(1)
        };
        let word_end = match real.get(n + 1) {
            None => true,
            Some(&next) => {
                starts[n + 1] || matches!(phdata.get(codes[next].code), Some(p) if p.typ == 0)
            }
        };
        for &(flags, old, new) in rules {
            if codes[i].code != old {
                continue;
            }
            if flags & 1 != 0 && !word_end {
                continue;
            }
            if flags & 2 != 0 && stress & 7 > 3 {
                continue;
            }
            if flags & 4 != 0 && !starts[n] {
                continue;
            }
            if new == 0 {
                deleted.push(i);
            } else {
                codes[i].code = new;
            }
            break;
        }
    }
    for &i in deleted.iter().rev() {
        codes.remove(i);
    }
}

/// Apply each phoneme's own `ChangePhoneme` program to a finished code list.
///
/// Upstream resolves context-dependent phonemes while *building* the phoneme
/// list, so `-x`, `--ipa` and the synthesizer all see the resolved phoneme.  The
/// port resolved them only when rendering IPA, so `-x -v pt "três vírgula"` printed
/// the unresolved `t#*'es#` (upstream: `t@-*'eZ`) and the synthesizer voiced the
/// virtual phoneme, which has no acoustic data of its own.
/// Apply the list-editing instructions a phoneme's program asks for —
/// `InsertPhoneme`, `AppendPhoneme`, `IfNextVowelAppend` and
/// `ChangeNextPhoneme` — to a finished code list.
///
/// C does this in `MakePhonemeList`; here it gives `-x` and the synthesizer the
/// same phoneme list the IPA renderer builds, so English gets its linking `r-`
/// ("sofa area" → `s'oUf@r- 'e@ri@`) and the `;` glide after `aI` ("my apple" →
/// `maI; 'ap@L`) in the audio too.
/// C's `SetPlist2` reads a syllabic marker as "the previous phoneme is a
/// syllable of its own, with the stress still pending" — and after a `||` word
/// break the pending stress is the default, so the list entry `'I||-b@-*'eve`
/// (Italian `й`) leaves `I` unstressed and puts no `-` in the list.
fn consume_word_break_syllabic(phonemes: &[u8], phdata: &PhonemeData) -> Vec<u8> {
    const PH_STRESS: u8 = 1;
    const PH_VOWEL: u8 = 2;
    let kind = |c: u8| phdata.get(c).map(|p| p.typ);
    let mut out: Vec<u8> = Vec::with_capacity(phonemes.len());
    for &b in phonemes {
        let after_break = out.last() == Some(&crate::phoneme::PHON_END_WORD);
        if b != crate::phoneme::PHON_SYLLABIC || !after_break {
            out.push(b);
            continue;
        }
        let n = out.len();
        let stressed_vowel = n >= 3
            && kind(out[n - 2]) == Some(PH_VOWEL)
            && kind(out[n - 3]) == Some(PH_STRESS);
        if stressed_vowel {
            out.remove(n - 3);
        }
    }
    out
}

/// C's phoneme writer prints the syllabic marker only after a non-vowel, so a
/// letter name coded for another table (`ku ع`) loses a `-` that lands on a vowel.
fn drop_syllabic_after_vowel(codes: &mut Vec<PhonemeCode>, phdata: &PhonemeData) {
    let mut prev_vowel = false;
    codes.retain(|c| {
        let syllabic = !c.is_boundary && c.code == crate::phoneme::PHON_SYLLABIC;
        let keep = !(syllabic && prev_vowel);
        if keep {
            prev_vowel = !c.is_boundary && phdata.get(c.code).is_some_and(|p| p.typ == 2);
        }
        keep
    });
}

/// Drop a French liaison phoneme that doesn't surface.
///
/// `z2`, `t2`, `n2`, `z3` … are consonants that are only pronounced before a
/// vowel ("deux enfants" keeps the /z/, "deux chats" drops it).  The IPA
/// renderer already resolved them; the codes path kept them, so `-x -v fr
/// "3,14"` printed `trwaz2` where upstream has `trwa`.
fn drop_unsurfaced_liaison(codes: &mut Vec<PhonemeCode>, phdata: &PhonemeData) {
    let is_sound = |c: &PhonemeCode| {
        !c.is_boundary && c.code > 8 && c.code != crate::phoneme::PHON_END_WORD
    };
    let real: Vec<usize> = codes.iter().enumerate().filter(|(_, c)| is_sound(c)).map(|(i, _)| i).collect();
    let mut drop: Vec<usize> = Vec::new();
    for (n, &i) in real.iter().enumerate() {
        let Some(ph) = phdata.get(codes[i].code) else { continue };
        let m = ph.mnemonic.to_le_bytes();
        let is_liaison = m[2] == 0 && matches!(m[1], b'2' | b'3') && ph.typ != 2;
        if !is_liaison {
            continue;
        }
        let next_is_vowel = real
            .get(n + 1)
            .and_then(|&k| phdata.get(codes[k].code))
            .is_some_and(|n| liaison_surfaces_before(ph, n));
        if !next_is_vowel {
            drop.push(i);
        }
    }
    for &i in drop.iter().rev() {
        codes.remove(i);
    }
}

/// `SetRegressiveVoicing` (`phonemelist.c`) — make a consonant cluster all
/// voiced or all voiceless, taking its voicing from the *last* member.
///
/// Walks the list backwards: a voiced obstruent sets `voicing = 2` and a
/// voiceless one sets `voicing = 1`; whichever is in force converts the
/// obstruents before it through their `end_type` (the voicing-switch partner
/// stored in the phoneme table).  Ukrainian `світ` is `zB'i:t[` because the `в`
/// that follows is voiced — and with `regression & 0x04` unset the assimilation
/// crosses word boundaries, so `привіт світ` ends the first word in `d[`.
///
/// `regression` bits: `0x02` = `v`/`R` stop propagation, `0x04` = stop at a word
/// boundary, `0x08` = propagate through liquids and nasals (Polish), `0x100` =
/// devoice word-finally.
fn set_regressive_voicing(
    codes: &mut [PhonemeCode],
    phdata: &PhonemeData,
    regression: u32,
    own_lang: &str,
) {
    use crate::phoneme::{PH_FRICATIVE, PH_PAUSE, PH_STOP, PH_VFRICATIVE, PH_VOWEL, PH_VSTOP};

    // Phonemes of a switched-in language keep their own voicing (`SFLAG_SWITCHED_LANG`).
    let mut in_switch = false;
    let switched: Vec<bool> = codes
        .iter()
        .map(|c| {
            if let Some(CodeMarker::LangSwitch(target)) = &c.marker {
                in_switch = target != own_lang;
            }
            in_switch && c.marker.is_none()
        })
        .collect();

    // 0 = nothing in force, 1 = voiceless, 2 = voiced.  The clause ends in a
    // pause, so with word-final devoicing the last consonant starts voiceless.
    let mut voicing = u8::from(regression & 0x100 != 0);
    let mut stop_propagation = false;

    for i in (0..codes.len()).rev() {
        if switched[i] {
            stop_propagation = false;
            voicing = u8::from(regression & 0x100 != 0);
            continue;
        }
        // A word boundary is a marker, not a phoneme; note it and move on.
        if codes[i].is_boundary
            || codes[i].marker.is_some()
            || codes[i].code == crate::phoneme::PHON_END_WORD
        {
            // A *clause* boundary is a pause phoneme in C's list, and a pause
            // resets the voicing in force — which is what devoices Polish
            // `flag.` before a clause starting with a voiced consonant
            // (`fl'ak|z'aZuwts;`, where the port used to read `fl'ag`).
            if codes[i].is_boundary && codes[i].code == 0 {
                voicing = 0;
            }
            if regression & 0x04 != 0 {
                voicing = 0;
            }
            if regression & 0x100 != 0 && voicing == 0 {
                voicing = 1;
            }
            continue;
        }
        let code = codes[i].code;
        let Some(ph) = phdata.get(code) else { continue };

        if regression & 0x02 != 0 {
            // `v` and `R` don't cause regression.
            let first = (ph.mnemonic & 0xff) as u8;
            if first == b'v' || first == b'R' {
                stop_propagation = true;
                if regression & 0x10 != 0 {
                    voicing = 0;
                }
            }
        }

        // `end_type` doubles as the voicing-switch partner for consonants, but
        // only when it really points at an obstruent of the opposite voicing —
        // several tables use the field for something else, and following it
        // blindly turned Czech `milióni` into `miliʲóni`.
        let switch_to = |want_voiced: bool| -> Option<u8> {
            let t = ph.end_type;
            let target = phdata.get(t)?;
            let ok = if want_voiced {
                matches!(target.typ, PH_VSTOP | PH_VFRICATIVE)
            } else {
                matches!(target.typ, PH_STOP | PH_FRICATIVE)
            };
            (t != 0 && ok).then_some(t)
        };

        match ph.typ {
            PH_STOP | PH_FRICATIVE => {
                if voicing == 0 && regression & 0xf != 0 {
                    voicing = 1;
                } else if voicing == 2 {
                    if let Some(t) = switch_to(true) {
                        codes[i].code = t; // → the voiced equivalent
                    }
                }
            }
            PH_VSTOP | PH_VFRICATIVE => {
                if voicing == 0 && regression & 0xf != 0 {
                    voicing = 2;
                } else if voicing == 1 {
                    if let Some(t) = switch_to(false) {
                        codes[i].code = t; // → the voiceless equivalent
                    }
                }
            }
            t => {
                if regression & 0x08 != 0 {
                    // Polish: propagate through liquids and nasals.
                    if t == PH_PAUSE || t == PH_VOWEL {
                        voicing = 0;
                    }
                } else {
                    voicing = 0;
                }
            }
        }

        if stop_propagation {
            voicing = 0;
            stop_propagation = false;
        }
    }
}

/// Which of the phonemes in `real` start a word.
///
/// C marks a word's first phoneme with `sourceix != 0`, and the `*PhW`
/// conditions refuse to look across one: Dutch `Q` deletes itself after an `x`
/// *in the same word* (`IF prevPhW(x) THEN ChangePhoneme(NULL)`), so "twintig
/// graden" must keep its `ɣ`.
pub(crate) fn word_starts(codes: &[PhonemeCode], real: &[usize]) -> Vec<bool> {
    let mut out = Vec::with_capacity(real.len());
    let mut prev_end = 0usize;
    for (n, &i) in real.iter().enumerate() {
        out.push(
            n == 0
                || codes[prev_end..i]
                    .iter()
                    .any(|c| c.is_boundary || c.code == crate::phoneme::PHON_END_WORD),
        );
        prev_end = i + 1;
    }
    out
}

/// Each vowel's 1-based position among the vowels of its word — C's
/// `CountVowelPosition`, which `isFirstVowel`/`isSecondVowel` test.  Non-vowels
/// get `0`.
pub(crate) fn vowel_positions(codes: &[PhonemeCode], phdata: &PhonemeData) -> Vec<u8> {
    let mut out = vec![0u8; codes.len()];
    let mut n = 0u8;
    for (i, c) in codes.iter().enumerate() {
        if c.is_boundary || c.code == crate::phoneme::PHON_END_WORD {
            n = 0;
            continue;
        }
        if matches!(phdata.get(c.code), Some(p) if p.typ == 2 /* phVOWEL */) {
            n = n.saturating_add(1);
            out[i] = n;
        }
    }
    out
}

/// Per-syllable stress levels for a clause's code list.
///
/// `SetWordStress` emits a stress phoneme ahead of every vowel whose level is
/// *diminished* (0) or above *unstressed* (1) — `phonSTRESS_D` for 0,
/// `phonSTRESS_2`/`_3`/`_P`/`_P2`/`_TONIC` for 2…6 — and nothing at all for 1.
/// Reading them back therefore recovers C's `plist->stresslevel` exactly, which
/// is what the phoneme programs' stress conditions test.
///
/// Returns `(level, wordstress)`, both indexed like `codes`: `level[i]` is the
/// vowel's level or [`NO_STRESS_LEVEL`] for anything that is not a vowel, and
/// `wordstress[i]` is the highest level in `i`'s word (C's `plist->wordstress`).
pub(crate) fn stress_levels(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
) -> (Vec<u8>, Vec<u8>) {
    let mut level = vec![NO_STRESS_LEVEL; codes.len()];
    let mut pending = 1u8; // STRESS_IS_UNSTRESSED
    for (i, c) in codes.iter().enumerate() {
        if c.is_boundary || c.code == crate::phoneme::PHON_END_WORD {
            pending = 1;
            continue;
        }
        match c.code {
            PHON_STRESS_D => pending = 0,
            PHON_STRESS_U => pending = 1,
            PHON_STRESS_2 => pending = 2,
            PHON_STRESS_3 => pending = 3,
            PHON_STRESS_P => pending = 4,
            PHON_STRESS_P2 => pending = 5,
            PHON_STRESS_TONIC => pending = 6,
            code => {
                if matches!(phdata.get(code), Some(p) if p.typ == 2 /* phVOWEL */) {
                    level[i] = pending;
                    pending = 1;
                }
            }
        }
    }

    // The word's maximum, broadcast back over the word.
    let mut wordstress = vec![0u8; codes.len()];
    let mut start = 0usize;
    let mut i = 0usize;
    while i <= codes.len() {
        let ends = i == codes.len()
            || codes[i].is_boundary
            || codes[i].code == crate::phoneme::PHON_END_WORD;
        if ends {
            let max = level[start..i]
                .iter()
                .filter(|&&l| l != NO_STRESS_LEVEL)
                .copied()
                .max()
                .unwrap_or(4);
            wordstress[start..i].fill(max);
            start = i + 1;
        }
        i += 1;
    }
    (level, wordstress)
}

/// `stress_levels` marker for a code that is not a vowel.
pub(crate) const NO_STRESS_LEVEL: u8 = u8::MAX;

/// C's `count_pitch_vowels`: a clause with no primary stress puts its tonic on
/// the last syllable of highest stress.  It runs at the intonation stage, after
/// the phoneme programs, so the vowel keeps the quality they gave its old
/// (unstressed) level — Portuguese `ser` is `sˈɨɹ`, not `sˈeɹ`.
fn promote_clause_tonics(codes: &mut Vec<PhonemeCode>, phdata: &PhonemeData, lang: &str, no_dim: bool) {
    let mut own = own_language_mask(codes, lang);
    let mut start = 0;
    while start < codes.len() {
        let mut end = codes[start..]
            .iter()
            .position(|c| c.is_boundary && c.code == 0)
            .map_or(codes.len(), |p| start + p);
        let clause = &codes[start..end];
        if let Some(p) = tonic_position(clause, &own[start..end], phdata, no_dim) {
            let (first, vowel) = promote_vowel(codes, start + p);
            let vowel_own = own[vowel];
            own.splice(first..vowel, [vowel_own]);
            end = end + 1 - (vowel - first);
        }
        start = end + 1;
    }
}

/// Which codes belong to the clause's own phoneme table, i.e. are outside any
/// `phonSWITCH` region of another language.
fn own_language_mask(codes: &[PhonemeCode], lang: &str) -> Vec<bool> {
    let mut own = true;
    codes
        .iter()
        .map(|c| {
            if let Some(CodeMarker::LangSwitch(target)) = &c.marker {
                own = target == lang;
            }
            own
        })
        .collect()
}

/// The vowel the tonic falls on, if the clause has no primary stress: the last
/// one of highest stress.  Stress levels are those of C's phoneme list, where
/// every second vowel of a run of unstressed ones is already diminished
/// (`MakePhonemeList`) unless the language sets `S_NO_DIM`.
fn tonic_position(clause: &[PhonemeCode], own: &[bool], phdata: &PhonemeData, no_dim: bool) -> Option<usize> {
    let (level, _) = stress_levels(clause, phdata);
    let mut run = 0u32;
    let mut best: Option<(u8, usize)> = None;
    let foreign_primary = clause
        .iter()
        .zip(own)
        .any(|(c, &o)| !o && !c.is_boundary && matches!(c.code, PHON_STRESS_P | PHON_STRESS_P2 | PHON_STRESS_TONIC));
    if foreign_primary {
        return None;
    }
    for (i, &l) in level.iter().enumerate().filter(|&(i, &l)| l != NO_STRESS_LEVEL && own[i]) {
        let l = if l > 1 {
            run = 0;
            l
        } else {
            run += 1;
            match (run > 1 && run % 2 == 0, no_dim) {
                (true, true) => { run = 1; l }
                (true, false) => 0,
                _ => l,
            }
        };
        if l >= 4 {
            return None;
        }
        if best.is_none_or(|(m, _)| l >= m) {
            best = Some((l, i));
        }
    }
    best.map(|(_, i)| i)
}

/// Puts a primary-stress mark before `vowel`, replacing the marks already
/// there; returns the replaced range `(first, vowel)`.
fn promote_vowel(codes: &mut Vec<PhonemeCode>, vowel: usize) -> (usize, usize) {
    let is_stress = |c: &PhonemeCode| matches!(c.code, 2..=7 | PHON_STRESS_TONIC) && !c.is_boundary;
    let first = (0..vowel).rev().take_while(|&i| is_stress(&codes[i])).last().unwrap_or(vowel);
    codes.splice(first..vowel, [PhonemeCode { code: PHON_STRESS_P, ..Default::default() }]);
    (first, vowel)
}

/// C keeps the lengthener as a flag on the phoneme before it, so `a::` is `a:`.
fn collapse_repeated_lengthen(codes: &mut Vec<PhonemeCode>, flags: &mut Vec<bool>) {
    let is_mark = |c: &PhonemeCode| {
        !c.is_boundary && c.marker.is_none() && c.code == crate::phoneme::PHON_LENGTHEN
    };
    let keep: Vec<bool> = (0..codes.len())
        .map(|i| !(i > 0 && is_mark(&codes[i]) && is_mark(&codes[i - 1])))
        .collect();
    let mut at = 0;
    codes.retain(|_| {
        at += 1;
        keep[at - 1]
    });
    at = 0;
    flags.retain(|_| {
        at += 1;
        keep[at - 1]
    });
}

/// C's `MakePhonemeList`: a consonant followed by the lengthener (`:`) is
/// lengthened **by doubling it** when it is a fricative, voiced fricative, nasal
/// or liquid, and the lengthener goes away.  Italian writes its geminates that
/// way — `mm` compiles to `m:` and reads `s'omma`, not `s'om:a`.
///
/// Every other type keeps the lengthener, which is why a long vowel still reads
/// `a:` (Dutch `Qr'a:d@n`) and a phoneme that is itself long, like Italian `tS:`,
/// is untouched.
///
/// C inserts the copy *before* the lengthened phoneme and never for the first
/// phoneme of a clause. A word-initial consonant therefore leaves its copy at the
/// end of the previous word, in front of the word boundary.
fn double_lengthened_consonants(
    codes: &mut [PhonemeCode],
    flags: &mut [bool],
    phdata: &PhonemeData,
) {
    use crate::phoneme::{PH_FRICATIVE, PH_LIQUID, PH_NASAL, PH_PAUSE, PH_VFRICATIVE};
    let is_sound = |c: &PhonemeCode| !c.is_boundary && c.marker.is_none() && c.code > 8;
    let is_word_break = |c: &PhonemeCode| c.is_boundary || c.code == crate::phoneme::PHON_END_WORD;
    for i in 1..codes.len() {
        if codes[i].is_boundary
            || codes[i].marker.is_some()
            || codes[i].code != crate::phoneme::PHON_LENGTHEN
        {
            continue;
        }
        let clause_start = codes[..i]
            .iter()
            .rposition(|c| c.is_boundary && c.code == 0)
            .map_or(0, |p| p + 1);
        let Some(prev) = codes[clause_start..i].iter().rposition(is_sound).map(|p| p + clause_start)
        else {
            continue;
        };
        let doubled = phdata
            .get(codes[prev].code)
            .is_some_and(|p| matches!(p.typ, PH_PAUSE | PH_FRICATIVE | PH_VFRICATIVE | PH_NASAL | PH_LIQUID));
        let Some(before) = codes[clause_start..prev].iter().rposition(is_sound).map(|p| p + clause_start)
        else {
            continue;
        };
        if !doubled {
            continue;
        }
        codes[i].code = codes[prev].code;
        if codes[before + 1..prev].iter().any(is_word_break) {
            codes[before + 1..=i].rotate_right(1);
            flags[before + 1..=i].rotate_right(1);
        }
    }
}

fn apply_list_editing_instructions(
    codes: &mut Vec<PhonemeCode>,
    phdata: &PhonemeData,
    vowel_pause: u32,
    word_gap: u8,
) {
    // C turns a lengthen mark into a flag on the vowel before it, so the
    // neighbours of a phoneme never include one (`i:` still sees the next vowel).
    let is_sound = |c: &PhonemeCode| {
        !c.is_boundary
            && c.code > 8
            && c.code != crate::phoneme::PHON_END_WORD
            && c.code != crate::phoneme::PHON_LENGTHEN
    };
    let real: Vec<usize> = codes.iter().enumerate().filter(|(_, c)| is_sound(c)).map(|(i, _)| i).collect();
    if real.is_empty() {
        return;
    }

    let starts = word_starts(codes, &real);
    let (level, _) = stress_levels(codes, phdata);
    // (index to insert at, code, before?) — collected first so indices stay valid.
    let mut edits: Vec<(usize, u8, bool)> = Vec::new();
    let mut replacements: Vec<(usize, u8)> = Vec::new();

    for (n, &i) in real.iter().enumerate() {
        let code = codes[i].code;
        let Some(ph) = phdata.get(code) else { continue };
        if ph.program == 0 {
            continue;
        }
        let prev = n.checked_sub(1).map(|k| codes[real[k]].code).unwrap_or(0);
        let next = real.get(n + 1).map(|&k| codes[k].code).unwrap_or(0);
        let next2 = real.get(n + 2).map(|&k| codes[k].code).unwrap_or(0);
        // A boundary *or* an `END_WORD` between this phoneme and the next means
        // the next one starts a word, so `nextPhW` must not see through it.
        // (Composed numbers join their parts with `END_WORD` alone, and without
        // this "1km" assimilated across the join: `w'0N k'Il@m,i:t3`.)
        let next_wordstart = real.get(n + 1).is_some_and(|&k| {
            codes[i + 1..k]
                .iter()
                .any(|c| c.is_boundary || c.code == crate::phoneme::PHON_END_WORD)
        });
        let next_stressed = real.get(n + 1).is_some_and(|&k| level.get(k).is_some_and(|&l| l >= 4));
        let nb = crate::synthesize::bytecode::Neighbours {
            prev,
            this: code,
            next,
            next2,
            this_wordstart: starts[n],
            prev_wordstart: n.checked_sub(1).is_some_and(|k| starts[k]),
            next_wordstart,
            next2_wordstart: next_wordstart,
            ..Default::default()
        };
        let fx = crate::synthesize::bytecode::interpret_phoneme_ctl(
            ph.program,
            &phdata.phonindex,
            &nb,
            |c| phdata.get(c).cloned(),
            true,
        );

        if let Some(c) = fx.insert_phoneme.filter(|&c| c != 0) {
            edits.push((i, c, true));
        }
        // A pause between words takes the slot C's `pd_APPENDPHONEME` would
        // use, so the Italian `dI; esse` glide is `dI_| esse`.
        let word_gap_fills_slot = {
            let at_clause_end = real.get(n + 1).is_none_or(|&k| {
                codes[i + 1..k].iter().any(|c| c.is_boundary && c.code == 0)
            });
            (at_clause_end || next_wordstart)
                && word_gap_phoneme(
                    vowel_pause,
                    word_gap,
                    ph.typ,
                    if at_clause_end { 0 } else { phdata.get(next).map_or(0, |p| p.typ) },
                    !at_clause_end && next_stressed,
                ) != 0
        };
        let appended = fx.append_phoneme.filter(|&c| c != 0).or_else(|| {
            fx.append_if_next_vowel
                .filter(|&c| c != 0)
                .filter(|_| matches!(phdata.get(next), Some(n) if n.typ == 2 /* phVOWEL */))
        });
        let appended = appended.filter(|_| !word_gap_fills_slot);
        if let Some(c) = appended {
            edits.push((i, c, false));
        }
        if let Some(c) = fx.replace_next_phoneme.filter(|&c| c != 0) {
            if let Some(&k) = real.get(n + 1) {
                replacements.push((k, c));
            }
        }
    }

    for (i, c) in replacements {
        codes[i].code = c;
    }
    if edits.is_empty() {
        return;
    }
    // Apply back to front so earlier indices stay valid.
    edits.sort_by(|a, b| b.0.cmp(&a.0).then(b.2.cmp(&a.2)));
    let mut inserted = vec![false; codes.len()];
    for (i, code, before) in edits {
        let mut at = if before { i } else { i + 1 };
        while !before && codes.get(at).is_some_and(|c| c.code == crate::phoneme::PHON_LENGTHEN && !c.is_boundary) {
            at += 1;
        }
        codes.insert(at, PhonemeCode { code, is_boundary: false, clause_char: None , marker: None});
        inserted.insert(at, true);
    }
    change_inserted_phonemes(codes, &inserted, phdata);
}

/// C interprets an inserted phoneme like any other in the list loop
/// (`MakePhonemeList`), so its own `ChangePhoneme` applies: the English
/// linking `r-` becomes plain `r` unless it follows `@` or `3`.
fn change_inserted_phonemes(codes: &mut [PhonemeCode], inserted: &[bool], phdata: &PhonemeData) {
    let is_sound = |c: &PhonemeCode| {
        !c.is_boundary && c.code > 8 && c.code != crate::phoneme::PHON_END_WORD
    };
    let real: Vec<usize> = codes.iter().enumerate().filter(|(_, c)| is_sound(c)).map(|(i, _)| i).collect();
    let starts = word_starts(codes, &real);
    for (n, &i) in real.iter().enumerate().filter(|(_, &i)| inserted[i]) {
        let Some(ph) = phdata.get(codes[i].code).filter(|p| p.program != 0) else { continue };
        let at = |k: usize| real.get(k).map(|&j| codes[j].code).unwrap_or(0);
        let next_wordstart = real.get(n + 1).is_some_and(|&k| {
            codes[i + 1..k]
                .iter()
                .any(|c| c.is_boundary || c.code == crate::phoneme::PHON_END_WORD)
        });
        let nb = crate::synthesize::bytecode::Neighbours {
            prev: n.checked_sub(1).map_or(0, at),
            this: codes[i].code,
            next: at(n + 1),
            next2: at(n + 2),
            this_wordstart: starts[n],
            prev_wordstart: n.checked_sub(1).is_some_and(|k| starts[k]),
            next_wordstart,
            next2_wordstart: next_wordstart,
            ..Default::default()
        };
        let fx = crate::synthesize::bytecode::interpret_phoneme_ctl(
            ph.program,
            &phdata.phonindex,
            &nb,
            |c| phdata.get(c).cloned(),
            true,
        );
        if let Some(c) = fx.change_phoneme_code.filter(|&c| c > 1) {
            codes[i].code = c;
        }
    }
}

fn after_stress(
    i: usize,
    n: usize,
    real: &[usize],
    level: &[u8],
    crosses: &dyn Fn(usize, usize) -> bool,
) -> bool {
    let mut from = i;
    for &at in real[..n].iter().rev() {
        if crosses(at, from) {
            return false;
        }
        if level[at] != NO_STRESS_LEVEL && level[at] >= 4 {
            return true;
        }
        from = at;
    }
    false
}

fn resolve_virtual_phonemes_in_codes(
    codes: &mut Vec<PhonemeCode>,
    phdata: &PhonemeData,
    from_dict: &[bool],
    reduce: u32,
) {
    // Stress-conditioned allophones (`ChangeIfDiminished(…)` and friends) are
    // applied by the interpreter below, which evaluates the real condition
    // against the syllable's stress level — Russian "молоко" reduces to
    // `mVlVk'o`, while English "unseen" keeps its unreduced `V` because that
    // syllable is *unstressed*, not *diminished*.
    let (level, wordstress) = stress_levels(codes, phdata);
    let vowel_pos = vowel_positions(codes, phdata);

    // Index of the next/previous code that is a real sound (not a boundary).
    // The length mark is *not* one: C turns a `phonLENGTHEN` into an
    // `SFLAG_LENGTHEN` flag on the phoneme before it and never puts it in the
    // list, so a program's `nextPh`/`next2Ph` looks straight past it.  Keeping
    // it visible made Turkish `teşekkür` read `teSEk:'YR` — the `e` before
    // `k:` saw "two non-vowels" where C sees the vowel after it.
    let real: Vec<usize> = codes
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            !c.is_boundary
                && c.code > 8
                && c.code != crate::phoneme::PHON_END_WORD
                && c.code != crate::phoneme::PHON_LENGTHEN
        })
        .map(|(i, _)| i)
        .collect();
    let starts = word_starts(codes, &real);
    let mut deleted: Vec<usize> = Vec::new();
    for (n, &i) in real.iter().enumerate() {
        let code = codes[i].code;
        let Some(ph) = phdata.get(code) else { continue };
        if ph.program == 0 {
            continue;
        }
        // (Consonant changes apply here too — `l` → `l/2` in "world".  They are
        // invisible in `-x` because espeak drops a mnemonic's `/` variant
        // indicator when printing, which is why upstream shows a plain `l`.)
        // A clause boundary is a *pause phoneme* in C's list, so a neighbour on
        // the other side of one is not visible as a sound: `es "a , b"` is the
        // stop `b'e`, not the post-vocalic fricative `B'e` the port read by
        // looking past the comma.
        let clause_break =
            |a: usize, b: usize| codes[a + 1..b].iter().any(|c| c.is_boundary && c.code == 0);
        let prev_n = (0..n).rev().find(|&k| !deleted.contains(&real[k]));
        let prev = prev_n
            .map(|k| real[k])
            .filter(|&k| !clause_break(k, i))
            .map(|k| codes[k].code)
            .unwrap_or(0);
        let next = real
            .get(n + 1)
            .copied()
            .filter(|&k| !clause_break(i, k))
            .map(|k| codes[k].code)
            .unwrap_or(0);
        let next2 = real
            .get(n + 2)
            .copied()
            .filter(|&k| !clause_break(i, k))
            .map(|k| codes[k].code)
            .unwrap_or(0);
        // A boundary marker between this phoneme and the next one means the next
        // phoneme starts a new word (`nextPhW` must not see through it).
        // A boundary *or* an `END_WORD` between this phoneme and the next means
        // the next one starts a word, so `nextPhW` must not see through it.
        // (Composed numbers join their parts with `END_WORD` alone, and without
        // this "1km" assimilated across the join: `w'0N k'Il@m,i:t3`.)
        let next_wordstart = real.get(n + 1).is_some_and(|&k| {
            codes[i + 1..k]
                .iter()
                .any(|c| c.is_boundary || c.code == crate::phoneme::PHON_END_WORD)
        });
        let next_ix = real.get(n + 1).copied();
        // C's `nextVowel` scans forward to the next vowel and gives up at a
        // word boundary; `prevVowel` is the word's previous vowel
        // (`worddata->prev_vowel`).  Both need the whole word, not a window.
        const PH_VOWEL: u8 = 2;
        let crosses = |a: usize, b: usize| {
            codes[a + 1..b]
                .iter()
                .any(|c| c.is_boundary || c.code == crate::phoneme::PHON_END_WORD)
        };
        let (mut next_vowel, mut next_vowel_stress) = (0u8, NO_STRESS_LEVEL);
        let mut from = i;
        for k in (n + 1)..real.len() {
            let at = real[k];
            if crosses(from, at) {
                break;
            }
            let c = codes[at].code;
            if phdata.get(c).is_some_and(|p| p.typ == PH_VOWEL) {
                next_vowel = c;
                next_vowel_stress = level[at];
                break;
            }
            from = at;
        }
        let (mut prev_vowel, mut prev_vowel_stress) = (0u8, NO_STRESS_LEVEL);
        let mut from = i;
        for k in (0..n).rev() {
            let at = real[k];
            if deleted.contains(&at) {
                continue;
            }
            if crosses(at, from) {
                break;
            }
            let c = codes[at].code;
            if phdata.get(c).is_some_and(|p| p.typ == PH_VOWEL) {
                prev_vowel = c;
                prev_vowel_stress = level[at];
                break;
            }
            from = at;
        }
        let nb = crate::synthesize::bytecode::Neighbours {
            prev,
            this: code,
            next,
            next2,
            stress: level[i],
            next_stress: next_ix.map(|k| level[k]).unwrap_or(NO_STRESS_LEVEL),
            prev_stress: prev_n.map(|k| level[real[k]]).unwrap_or(NO_STRESS_LEVEL),
            next2_stress: real.get(n + 2).map(|&k| level[k]).unwrap_or(NO_STRESS_LEVEL),
            wordstress: wordstress[i],
            vowel_position: vowel_pos[i],
            reduce,
            translation_given: from_dict.get(i).copied().unwrap_or(false),
            this_wordstart: starts[n],
            prev_wordstart: n.checked_sub(1).is_some_and(|k| starts[k]),
            next_wordstart,
            next2_wordstart: next_wordstart,
            next_vowel,
            next_vowel_stress,
            prev_vowel,
            prev_vowel_stress,
            vowels_tracked: true,
            after_stress: after_stress(i, n, &real, &level, &crosses),
        };
        let extract = crate::synthesize::bytecode::interpret_phoneme_ctl(
            ph.program,
            &phdata.phonindex,
            &nb,
            |c| phdata.get(c).cloned(),
            true,
        );
        if let Some(changed) = extract.change_phoneme_code.filter(|&c| c != 0 && c != code) {
            if changed == 1 {
                // The NULL phoneme: `ChangePhoneme(NULL)` deletes this phoneme.
                deleted.push(i);
            } else {
                // A vowel turned into a consonant (French `i` → `j`) is no
                // longer a syllable, so the stress mark before it goes.
                if ph.typ == 2 && phdata.get(changed).is_some_and(|c| c.typ != 2) {
                    let marks = codes[..i]
                        .iter()
                        .rev()
                        .take_while(|c| {
                            !c.is_boundary
                                && matches!(
                                    c.code,
                                    PHON_STRESS_D | PHON_STRESS_U | PHON_STRESS_2 | PHON_STRESS_3
                                        | PHON_STRESS_P | PHON_STRESS_P2 | PHON_STRESS_TONIC
                                )
                        })
                        .count();
                    deleted.extend(i - marks..i);
                }
                codes[i].code = changed;
            }
        }
    }
    deleted.sort_unstable();
    deleted.dedup();
    for &i in deleted.iter().rev() {
        codes.remove(i);
    }
}

/// A single phoneme event in the synthesizer's input stream.
/// The target language of a `phonSWITCH` entry (`[phonSWITCH, <lang bytes…>]`).
///
/// A bare `phonSWITCH` with no name switches to the default voice: the
/// dictionary compiler omits the name when it equals `ESPEAKNG_DEFAULT_VOICE`.
fn switch_target(phonemes: &[u8]) -> String {
    let target: String = phonemes
        .iter()
        .skip(1)
        .take_while(|&&b| b.is_ascii_alphanumeric() || b == b'-')
        .map(|&b| (b as char).to_ascii_lowercase())
        .collect();
    if target.is_empty() { "en".to_string() } else { target }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhonemeCode {
    /// espeak-ng phoneme code.  See `synthesize.h` and the phoneme data files.
    pub code: u8,
    /// True if this is a boundary marker (word boundary, clause boundary).
    pub is_boundary: bool,
    /// When this item ends a clause (`Token::ClauseBoundary`), the punctuation
    /// character (`,`, `.`, etc.).  Used for CLI phoneme output formatting.
    pub clause_char: Option<char>,
    /// A non-phoneme marker in the code stream.  The marker itself has
    /// `code == 0` and `is_boundary == false`, so anything walking the codes for
    /// synthesis skips it unless it looks here.
    pub marker: Option<CodeMarker>,
}

/// A marker carried alongside the phoneme codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeMarker {
    /// A `phonSWITCH`: the codes that follow belong to this language's phoneme
    /// table, not the clause's, until the next marker.
    ///
    /// A marker carrying the clause's own language closes the switch —
    /// mirroring how upstream prints `(en)h@l'oU(ru)` and clears `phoneme_tab`
    /// on a table switch.
    LangSwitch(String),
    /// An inline embedded command (`\x01[±]<value><letter>`): a rate, pitch or
    /// amplitude change that applies to everything **after** this point.
    Embedded(EmbeddedCmd),
    /// A punctuation character that is not spoken.  Only the phonemizer-style
    /// IPA output (`preserve_punctuation`) prints it; every other consumer skips
    /// the marker like any other.
    Punctuation(char),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the C `SelectTranslator()` "missing break" bug class
    /// (upstream #2476): assert each language's number grammar is its *own*, with
    /// no cross-contamination between neighbours and no leak to unknown languages.
    #[test]
    fn lang_number_grammars_are_independent() {
        let g = |lang: &str| LangOptions::for_lang(lang).number_grammar;

        // Each language gets its specific distinguishing config …
        assert!(g("en").hundreds.use_conjunction_with_remainder, "en: 'and' before remainder");
        assert_eq!(g("es").tens, TensGrammar::WithConjunction, "es: 'treinta y cuatro'");
        assert!(g("es").hundreds.omit_one_prefix);
        assert!(g("fr").hundreds.omit_one_prefix, "fr: 'cent' not 'un cent'");
        assert_eq!(g("de").tens, TensGrammar::UnitsThenConjunction, "de: 'vier und dreißig'");
        assert!(g("de").ordinals.dot_marks_ordinal, "de: '3.' is ordinal");

        // … and does NOT inherit a neighbour's config.
        assert!(!g("fr").ordinals.dot_marks_ordinal, "fr must not take de's dot-ordinal");
        assert_ne!(g("fr").tens, TensGrammar::UnitsThenConjunction, "fr must not take de's tens order");
        assert_ne!(g("en").tens, TensGrammar::WithConjunction, "en must not take es's tens conjunction");
        // English keeps the "one" before hundred/thousand; de/fr/es/ru omit it.
        assert!(!g("en").hundreds.omit_one_prefix, "en keeps 'one hundred'");
        assert!(!g("en").thousands.omit_one_prefix, "en keeps 'one thousand'");
        // German keeps its "ein" (upstream sets neither `NUM_OMIT_1_HUNDRED` nor
        // `NUM_OMIT_1_THOUSAND`), but takes the combining `_1a` form before a
        // scale word: 100 is "einhundert", 1000 "ein tausend", 1 "eins".
        assert!(!g("de").hundreds.omit_one_prefix && !g("de").thousands.omit_one_prefix, "de keeps 'ein'");
        assert!(g("es").hundreds.omit_one_prefix && g("es").thousands.omit_one_prefix, "es: 'cien'/'mil'");

        // Languages sharing a match arm are intentionally identical in their
        // *number* config.  nl additionally carries the Dutch-IJ tokenizer flag
        // and the Germanic compound-ordinal flag (eenentwintigste), both
        // Dutch-only, so normalise them out before comparing to Maltese.
        assert!(g("nl").ordinals.compound_cardinal_suffix, "nl: compound ordinals suffix the cardinal");
        assert!(!g("mt").ordinals.compound_cardinal_suffix, "mt: does not");
        let mut nl = g("nl");
        nl.dutch_ij = false;
        nl.ordinals.compound_cardinal_suffix = false;
        (nl.c_numbers, nl.c_numbers2) = (g("mt").c_numbers, g("mt").c_numbers2);
        // Dutch is also one of C's `NUM_1900` languages ("negentienhonderd"),
        // Maltese is not.
        assert!(g("nl").year_1900 && !g("mt").year_1900, "nl reads 19xx as a year");
        nl.year_1900 = false;
        // Dutch writes the decimal as a comma, Maltese as a point (C `NUM_DECIMAL_COMMA`).
        assert_eq!(g("nl").decimal_separator, ',');
        assert_eq!(g("mt").decimal_separator, '.');
        (nl.group_separator, nl.decimal_separator, nl.space_group) =
            (g("mt").group_separator, g("mt").decimal_separator, g("mt").space_group);
        assert_eq!(nl, g("mt"), "nl and mt share the same number arm (apart from Dutch-only flags)");
        // Danish/Faroese/Slovenian are units-first ("enogtyve"), so they differ
        // from the tens-first Finnish/Estonian they used to share an arm with.
        assert_eq!(g("da").tens, TensGrammar::UnitsThenConjunction, "da: 'enogtyve'");
        assert!(g("da").ordinals.compound_cardinal_suffix, "da: compound ordinals suffix the cardinal");
        assert_eq!(g("fi").tens, TensGrammar::Standard, "fi: tens-first 'kaksikymmentäyksi'");
        assert_ne!(g("da"), g("fi"), "da (units-first) must differ from fi (tens-first)");
        assert_eq!(g("fi"), g("et"), "fi and et still share the tens-first arm");
        // … but distinct from an unrelated arm.
        assert_ne!(g("nl"), g("de"), "nl (own arm) differs from de");
        assert_ne!(g("da"), g("nl"), "da differs from nl's fuller config");

        // Unknown / untabulated languages fall back to the default, never a
        // neighbour's config (what the missing-break bug produced in C).
        // C's default translator groups thousands with a comma.
        let default = NumberGrammar { group_separator: Some(','), ..NumberGrammar::default() };
        assert_eq!(g("zz"), default, "unknown lang → default grammar");
        assert_eq!(g("xyz"), default);
    }

    fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        let mut needle_ix = 0;
        for &byte in haystack {
            if byte == needle[needle_ix] {
                needle_ix += 1;
                if needle_ix == needle.len() {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn translator_new_default_succeeds() {
        // Should succeed even if data files don't exist (just builds the struct)
        let t = Translator::new_default("en").unwrap();
        assert_eq!(t.options.lang, "en");
        assert_eq!(t.options.rate, 175);
    }

    #[test]
    fn normalize_voice_tag_us_underscore() {
        assert_eq!(normalize_voice_tag("en_US"), "en-us");
        assert_eq!(normalize_voice_tag(" EN-us "), "en-us");
    }

    #[test]
    fn split_voice_variant_cases() {
        assert_eq!(split_voice_variant("en"), ("en", None));
        assert_eq!(split_voice_variant("en+f3"), ("en", Some("f3")));
        assert_eq!(split_voice_variant("en-us+m3"), ("en-us", Some("m3")));
        assert_eq!(split_voice_variant("en+whisper"), ("en", Some("whisper")));
        // Empty / degenerate variant → None (still usable base).
        assert_eq!(split_voice_variant("en+"), ("en", None));
        // Leading `+` (no base) is left intact rather than yielding an empty base.
        assert_eq!(split_voice_variant("+f3"), ("+f3", None));
    }

    #[test]
    fn translator_new_strips_variant() {
        // `en+f3` must resolve the `en` language data, not a nonexistent
        // `en+f3` dict — the variant is acoustic-only.
        let t = Translator::new_default("en+f3").unwrap();
        assert_eq!(t.options.lang, "en");
    }

    #[test]
    fn lang_options_en_us_matches_en_number_grammar() {
        let o = LangOptions::for_lang("en-US");
        assert_eq!(o.lang, "en-us");
        assert_eq!(o.number_grammar, LangOptions::for_lang("en").number_grammar);
    }

    #[test]
    fn translator_new_default_normalizes_voice_tag() {
        let t = Translator::new_default("en_US").unwrap();
        assert_eq!(t.options.lang, "en-us");
    }

    #[test]
    fn tokenize_hello_world() {
        let tokens = tokenize("hello world");
        assert_eq!(tokens, vec![
            Token::Word("hello".to_string()),
            Token::Space,
            Token::Word("world".to_string()),
        ]);
    }

    #[test]
    fn tokenize_with_punctuation() {
        let tokens = tokenize("hello, world!");
        assert!(tokens.iter().any(|t| t == &Token::Word("hello".to_string())));
        assert!(tokens.iter().any(|t| t == &Token::Word("world".to_string())));
        assert!(tokens.iter().any(|t| t == &Token::ClauseBoundary(',')));
        assert!(tokens.iter().any(|t| t == &Token::ClauseBoundary('!')));
    }

    #[test]
    fn comma_groups_thousands_for_english() {
        let en = NumberGrammar::for_lang("en");
        // Well-formed grouping → a single number token with the commas removed.
        assert_eq!(
            tokenize_opts("1,000", &en),
            vec![Token::Number(NumberToken::Cardinal("1000".into()))]
        );
        assert_eq!(
            tokenize_opts("1,234,567", &en),
            vec![Token::Number(NumberToken::Cardinal("1234567".into()))]
        );
        // A decimal after a group still parses.
        assert_eq!(
            tokenize_opts("1,000.5", &en),
            vec![Token::Number(NumberToken::Decimal { integer: "1000".into(), fractional: "5".into(), grouped: true })]
        );
        // Malformed groups (not exactly three digits) keep the comma as a boundary.
        assert!(
            tokenize_opts("1,0000", &en).iter().any(|t| t == &Token::ClauseBoundary(',')),
            "4-digit group must not be absorbed"
        );
        // A language without a group separator (the default) is unchanged.
        assert!(
            tokenize_opts("1,000", &NumberGrammar::default())
                .iter()
                .any(|t| t == &Token::ClauseBoundary(',')),
            "no grouping without a configured separator"
        );
    }


    #[test]
    fn numeric_exponent_is_spoken() {
        let mut t = tokenize_opts("2^10", &NumberGrammar::for_lang("en"));
        apply_exponents(&mut t, "en");
        let words: Vec<String> = t.iter().filter_map(|t| match t {
            Token::Word(w) => Some(w.clone()),
            Token::Number(NumberToken::Cardinal(n)) => Some(n.clone()),
            _ => None,
        }).collect();
        assert_eq!(words, vec!["2", "to", "the", "power", "of", "10"]);
        // A non-numeric base (`x^2`) is left alone (no spurious rewrite).
        let mut t2 = tokenize_opts("x^2", &NumberGrammar::for_lang("en"));
        apply_exponents(&mut t2, "en");
        assert!(t2.iter().any(|t| matches!(t, Token::Punctuation('^'))));
    }

    #[test]
    fn simple_fractions_are_spoken() {
        let rewrite = |s: &str, lang: &str| {
            let mut t = tokenize_opts(s, &NumberGrammar::for_lang(lang));
            apply_fractions(&mut t, lang);
            t
        };
        let words = |s: &str| -> Vec<String> {
            rewrite(s, "en").iter().filter_map(|t| match t {
                Token::Word(w) => Some(w.clone()),
                Token::Number(NumberToken::Cardinal(n)) => Some(n.clone()),
                _ => None,
            }).collect()
        };
        assert_eq!(words("1/2"), vec!["1", "half"]);
        assert_eq!(words("3/4"), vec!["3", "quarters"]);
        assert_eq!(words("2/3"), vec!["2", "thirds"]);
        // Mixed numbers: "1 1/2" → "one and one half".
        assert_eq!(words("1 1/2"), vec!["1", "and", "1", "half"]);
        assert_eq!(words("2 3/4"), vec!["2", "and", "3", "quarters"]);
        // A date-like `a/b/c` is not a fraction.
        assert!(rewrite("3/4/2024", "en").iter().any(|t| matches!(t, Token::Punctuation('/'))));
        // A large denominator is left as a slash.
        assert!(rewrite("1/12", "en").iter().any(|t| matches!(t, Token::Punctuation('/'))));
        // Non-English is untouched.
        assert!(rewrite("1/2", "de").iter().any(|t| matches!(t, Token::Punctuation('/'))));
    }

    #[test]
    fn leading_decimal_point_is_a_bare_separator() {
        let en = NumberGrammar::for_lang("en");
        assert_eq!(
            tokenize_opts(".5", &en),
            vec![Token::Number(NumberToken::Bare { separator: '.', digits: "5".into() })]
        );
        assert_eq!(
            tokenize_opts(".25", &en),
            vec![Token::Number(NumberToken::Bare { separator: '.', digits: "25".into() })]
        );
        // A dot NOT followed by a digit is still a clause boundary.
        assert!(
            tokenize_opts("5.", &en).iter().any(|t| matches!(t, Token::ClauseBoundary('.'))),
            "trailing dot must stay a sentence boundary"
        );
        // Works for the language's own decimal separator (German comma).
        let de = NumberGrammar::for_lang("de");
        assert_eq!(
            tokenize_opts(",5", &de),
            vec![Token::Number(NumberToken::Bare { separator: ',', digits: "5".into() })]
        );
    }

    #[test]
    fn native_script_digits_become_numbers() {
        // Arabic-Indic, Devanagari, Persian, Thai, Bengali → ASCII digits, so
        // they tokenize as numbers instead of being dropped.
        assert_eq!(native_digit_to_ascii('٥'), Some('5')); // Arabic-Indic
        assert_eq!(native_digit_to_ascii('५'), Some('5')); // Devanagari
        assert_eq!(native_digit_to_ascii('۵'), Some('5')); // Persian
        assert_eq!(native_digit_to_ascii('๕'), Some('5')); // Thai
        assert_eq!(native_digit_to_ascii('a'), None);
        // Consecutive native digits form a single multi-digit number.
        assert_eq!(normalize_number_symbols("١٢٣"), "123");
        let g = NumberGrammar::default();
        assert_eq!(tokenize_opts("٥", &g), vec![Token::Number(NumberToken::Cardinal("5".into()))]);
        assert_eq!(tokenize_opts("١٢٣", &g), vec![Token::Number(NumberToken::Cardinal("123".into()))]);

        // Arabic native separators normalise to their Latin equivalents.
        assert_eq!(normalize_number_symbols("١٫٥"), "1.5");   // decimal ٫
        assert_eq!(normalize_number_symbols("١٬٠٠٠"), "1,000"); // thousands ٬
        assert_eq!(normalize_number_symbols("٥٪"), "5%");      // percent ٪

        // Full-width (CJK-compatibility) forms → ASCII.
        assert_eq!(fullwidth_to_ascii('５'), Some('5'));
        assert_eq!(fullwidth_to_ascii('Ａ'), Some('A'));
        assert_eq!(fullwidth_to_ascii('＄'), Some('$'));
        assert_eq!(fullwidth_to_ascii('\u{3000}'), Some(' ')); // ideographic space
        assert_eq!(fullwidth_to_ascii('5'), None);
        assert_eq!(normalize_number_symbols("Ｈｅｌｌｏ"), "Hello");
        assert_eq!(normalize_number_symbols("１２３"), "123");

        // Enclosed digits and Roman-numeral codepoints → their value.
        assert_eq!(enclosed_number_value('①'), Some(1));
        assert_eq!(enclosed_number_value('⑩'), Some(10));
        assert_eq!(enclosed_number_value('Ⅳ'), Some(4));
        assert_eq!(enclosed_number_value('Ⅿ'), Some(1000));
        assert_eq!(enclosed_number_value('5'), None);
        assert!(tokenize_opts("①", &g).iter()
            .any(|t| t == &Token::Number(NumberToken::Cardinal("1".into()))));
    }

    #[test]
    fn unicode_number_symbols_are_normalized() {
        use std::borrow::Cow;
        // Superscripts read as plain digits; subscripts too (H₂O).
        assert_eq!(normalize_number_symbols("x²"), "x 2");
        assert_eq!(normalize_number_symbols("10³"), "10 3");
        assert_eq!(normalize_number_symbols("H₂O"), "H 2O");
        // Vulgar fractions stay symbols: the dictionary names them.
        assert_eq!(normalize_number_symbols("½"), "½");
        // Plain text is returned borrowed (no allocation).
        assert!(matches!(normalize_number_symbols("hello"), Cow::Borrowed("hello")));

        // Stylized (mathematical/circled) letters and digits → plain ASCII.
        assert_eq!(normalize_number_symbols("𝐇𝐞𝐥𝐥𝐨"), "Hello"); // bold
        assert_eq!(normalize_number_symbols("𝓗𝓮𝓵𝓵𝓸"), "Hello"); // bold script
        assert_eq!(normalize_number_symbols("𝔻𝕒𝕥𝕒"), "Data");    // double-struck
        assert_eq!(normalize_number_symbols("Ⓗⓘ"), "Hi");         // circled
        assert_eq!(normalize_number_symbols("𝟏𝟐𝟑"), "123");       // bold digits
        assert_eq!(normalize_number_symbols("ℝℤℕ"), "RZN");        // letterlike holes
        assert_eq!(stylized_to_ascii('𝐀'), Some('A'));
        assert_eq!(stylized_to_ascii('𝐳'), Some('z'));
        assert_eq!(stylized_to_ascii('A'), None); // plain ASCII untouched

        // End-to-end: `x²` yields a number token; so does a subscript `H₂`.
        let en = NumberGrammar::for_lang("en");
        assert!(
            tokenize_opts("x²", &en).iter().any(|t| t == &Token::Number(NumberToken::Cardinal("2".into()))),
            "superscript ² should yield a number token"
        );
        assert!(
            tokenize_opts("H₂", &en).iter().any(|t| t == &Token::Number(NumberToken::Cardinal("2".into()))),
            "subscript should still yield a number token"
        );
    }

    #[test]
    fn non_ordinal_suffix_keeps_the_number() {
        let en = NumberGrammar::for_lang("en");
        // A plural "s" (decades) is not an ordinal → the number is still a number,
        // and the "s" is a separate word (previously the number was swallowed).
        assert_eq!(
            tokenize_opts("1990s", &en),
            vec![
                Token::Number(NumberToken::Cardinal("1990".into())),
                Token::Word("s".into()),
            ]
        );
        // A unit suffix likewise stays a separate word.
        assert_eq!(
            tokenize_opts("5km", &en),
            vec![
                Token::Number(NumberToken::Cardinal("5".into())),
                Token::Word("km".into()),
            ]
        );
        // Genuine ordinal suffixes still parse as ordinals.
        assert!(matches!(
            tokenize_opts("3rd", &en).as_slice(),
            [Token::Number(NumberToken::Ordinal(o))] if o.digits == "3"
        ));
        assert!(matches!(
            tokenize_opts("21st", &en).as_slice(),
            [Token::Number(NumberToken::Ordinal(o))] if o.digits == "21"
        ));
        // A st/nd/rd/th suffix that does not match the number is still an
        // ordinal, as in upstream ("2st" is "secst").
        for (input, num) in [("2st", "2"), ("1nd", "1"), ("3th", "3"), ("11st", "11")] {
            assert!(
                matches!(
                    tokenize_opts(input, &en).as_slice(),
                    [Token::Number(NumberToken::Ordinal(o))] if o.digits == num
                ),
                "{input} should be an ordinal",
            );
        }
        // …but the correct suffix for the same tens still parses (11th, 111th).
        assert!(matches!(
            tokenize_opts("11th", &en).as_slice(),
            [Token::Number(NumberToken::Ordinal(o))] if o.digits == "11"
        ));
    }

    #[test]
    fn european_decimal_and_grouping_are_swapped() {
        let de = NumberGrammar::for_lang("de");
        assert_eq!((de.decimal_separator, de.group_separator), (',', Some('.')));
        // German uses `.` to group and `,` for the decimal.
        assert_eq!(
            tokenize_opts("1.000", &de),
            vec![Token::Number(NumberToken::Cardinal("1000".into()))]
        );
        assert_eq!(
            tokenize_opts("3,14", &de),
            vec![Token::Number(NumberToken::Decimal { integer: "3".into(), fractional: "14".into(), grouped: false })]
        );
        assert_eq!(
            tokenize_opts("1.000,5", &de),
            vec![Token::Number(NumberToken::Decimal { integer: "1000".into(), fractional: "5".into(), grouped: true })]
        );

        // English is the mirror image and stays as before.
        let en = NumberGrammar::for_lang("en");
        assert_eq!((en.decimal_separator, en.group_separator), ('.', Some(',')));
        assert_eq!(
            tokenize_opts("3.14", &en),
            vec![Token::Number(NumberToken::Decimal { integer: "3".into(), fractional: "14".into(), grouped: false })]
        );
    }

    #[test]
    fn tokenize_empty() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn tokenize_apostrophe() {
        let tokens = tokenize("it's");
        assert_eq!(tokens, vec![Token::Word("it's".to_string())]);
    }

    #[test]
    fn clause_flags_fields_do_not_overlap() {
        assert!(
            (ClauseFlags::PAUSE_MASK & ClauseFlags::INTONATION_MASK).is_empty()
        );
        assert!(
            (ClauseFlags::INTONATION_MASK & ClauseFlags::TYPE_MASK).is_empty()
        );
    }

    #[test]
    fn read_clauses_basic() {
        let t = Translator::new_default("en").unwrap();
        let clauses = t.read_clauses("Hello world. How are you?").unwrap();
        assert_eq!(clauses.len(), 2);
        assert_eq!(clauses[0].intonation, Intonation::FullStop);
        assert_eq!(clauses[1].intonation, Intonation::Question);
    }

    #[test]
    fn read_clauses_no_punctuation() {
        let t = Translator::new_default("en").unwrap();
        let clauses = t.read_clauses("hello world").unwrap();
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].text, "hello world");
    }

    // ── phonemes_to_ipa ────────────────────────────────────────────────────

    fn make_phdata() -> Option<PhonemeData> {
        let dir = std::path::Path::new("/usr/share/espeak-ng-data");
        if !dir.join("phontab").exists() { return None; }
        let mut phdata = PhonemeData::load(dir).ok()?;
        phdata.select_table_by_name("en").ok()?;
        Some(phdata)
    }

    #[test]
    fn phonemes_to_ipa_the() {
        // "the" dict phonemes: [87, 115] = [D, @2] → ðə
        let phdata = match make_phdata() { Some(d) => d, None => return };
        let (ipa, _) = phonemes_to_ipa(&[87, 115], &phdata, PendingStress::None, false);
        assert_eq!(ipa, "ðə");
    }

    #[test]
    fn phonemes_to_ipa_be() {
        // "be" dict phonemes: [72, 137] = [b, i:] → biː
        let phdata = match make_phdata() { Some(d) => d, None => return };
        let (ipa, _) = phonemes_to_ipa(&[72, 137], &phdata, PendingStress::None, false);
        assert_eq!(ipa, "biː");
    }

    #[test]
    fn phonemes_to_ipa_with_stress() {
        // "not" dict: [4, 50, 129, 47] = [STRESS_2, n, 0, t]
        // secondary stress before the vowel (ɒ), consonant onset comes before stress mark
        // → "nˌɒt" (stress mark immediately before the stressed vowel)
        let phdata = match make_phdata() { Some(d) => d, None => return };
        let (ipa, _) = phonemes_to_ipa(&[4, 50, 129, 47], &phdata, PendingStress::None, false);
        assert_eq!(ipa, "nˌɒt");
    }

    #[test]
    fn text_to_ipa_be() {
        let t = Translator::new_default("en").unwrap();
        if !Path::new("/usr/share/espeak-ng-data/en_dict").exists() { return; }
        let ipa = t.text_to_ipa("be").unwrap();
        // "be" in isolation gets primary stress via clause-level promotion
        assert_eq!(ipa, "bˈiː");
    }

    #[test]
    fn text_to_ipa_en_us_shares_en_dict_but_not_its_phoneme_table() {
        let data_dir = Path::new("espeak-ng-data");
        let data_dir = if data_dir.join("en_dict").exists() {
            data_dir
        } else if Path::new("/usr/share/espeak-ng-data/en_dict").exists() {
            Path::new("/usr/share/espeak-ng-data")
        } else {
            return;
        };
        let t = Translator::new("en_US", Some(data_dir)).unwrap();
        assert_eq!(t.options.lang, "en-us");
        let ipa = t.text_to_ipa("hello").unwrap();
        assert!(!ipa.is_empty());
        let t_en = Translator::new("en", Some(data_dir)).unwrap();
        // en-US has no en-us_dict, so it shares en_dict — but it has its own
        // *phoneme table*, and the GOAT vowel is where the two part company.
        // (This used to assert the two were equal, which was the bug: the
        // en-GB IPA overrides outranked the en-us table's own phondata.)
        assert_eq!(ipa, "həlˈoʊ");
        assert_eq!(t_en.text_to_ipa("hello").unwrap(), "həlˈəʊ");
    }

    #[test]
    fn word_to_ipa_unseen_retranslates_stem_after_un_prefix() {
        let data_dir = Path::new("espeak-ng-data");
        let data_dir = if data_dir.join("en_dict").exists() {
            data_dir
        } else if Path::new("/usr/share/espeak-ng-data/en_dict").exists() {
            Path::new("/usr/share/espeak-ng-data")
        } else {
            return;
        };
        let t = Translator::new("en-us", Some(data_dir)).unwrap();
        let ipa = t.text_to_ipa("unseen").unwrap();
        assert!(
            ipa.contains("sˈiːn") || ipa.contains("siːn"),
            "expected 'seen' syllable in output: {ipa:?}"
        );
    }

    #[test]
    fn word_to_ipa_seen_or_unseen_us_has_no_control_chars() {
        let data_dir = Path::new("espeak-ng-data");
        let data_dir = if data_dir.join("en_dict").exists() {
            data_dir
        } else if Path::new("/usr/share/espeak-ng-data/en_dict").exists() {
            Path::new("/usr/share/espeak-ng-data")
        } else {
            return;
        };
        let t = Translator::new("en-us", Some(data_dir)).unwrap();
        let ipa = t.text_to_ipa("seen or unseen,").unwrap();
        assert!(
            !ipa.chars().any(|c| c.is_control()),
            "IPA must not contain C0 controls: {ipa:?}"
        );
    }

    /// GitHub #4: `translate_to_codes` must apply the same clause promotions as `text_to_ipa`
    /// (WH-word secondary stress; linking **r** after **or**).
    #[test]
    fn github_issue4_translate_codes_matches_ipa_pipeline() {
        let data_dir = Path::new("espeak-ng-data");
        let data_dir = if data_dir.join("en_dict").exists() {
            data_dir
        } else if Path::new("/usr/share/espeak-ng-data/en_dict").exists() {
            Path::new("/usr/share/espeak-ng-data")
        } else {
            return;
        };
        let mut phdata = PhonemeData::load(data_dir).unwrap();
        phdata.select_table_by_name("en-us").unwrap();
        let r_code = phdata.lookup_phoneme("r");
        assert!(r_code > 0, "en-us phontab should define r");

        let t = Translator::new("en-us", Some(data_dir)).unwrap();
        let wh = t.translate_to_codes("When choices cease").unwrap();
        let flat: Vec<u8> = wh
            .iter()
            .filter(|c| !c.is_boundary)
            .map(|c| c.code)
            .collect();
        assert!(
            flat.len() >= 3 && flat[0] == 58 && flat[1] == 4,
            "expected w then secondary stress (4) for clause-initial when: {flat:?}"
        );

        let or_line = t.translate_to_codes("seen or unseen").unwrap();
        let flat: Vec<u8> = or_line
            .iter()
            .filter(|c| !c.is_boundary)
            .map(|c| c.code)
            .collect();
        assert!(
            flat.windows(2).any(|w| w[0] == 140 && w[1] == r_code),
            "expected linking /r/ phoneme after dict tail of or (140): {flat:?}"
        );
    }

    #[test]
    fn text_to_ipa_he() {
        let t = Translator::new_default("en").unwrap();
        if !Path::new("/usr/share/espeak-ng-data/en_dict").exists() { return; }
        let ipa = t.text_to_ipa("he").unwrap();
        // "he" in isolation gets primary stress via clause-level promotion
        assert_eq!(ipa, "hˈiː");
    }

    #[test]
    fn text_to_ipa_do() {
        let t = Translator::new_default("en").unwrap();
        if !Path::new("/usr/share/espeak-ng-data/en_dict").exists() { return; }
        let ipa = t.text_to_ipa("do").unwrap();
        assert_eq!(ipa, "dˈuː");
    }

    #[test]
    fn text_to_ipa_the() {
        let t = Translator::new_default("en").unwrap();
        if !Path::new("/usr/share/espeak-ng-data/en_dict").exists() { return; }
        let ipa = t.text_to_ipa("the").unwrap();
        // "the" in isolation gets primary stress via clause-level promotion (matches C oracle)
        assert_eq!(ipa, "ðˈə");
    }

    // ── CJK tokenize ────────────────────────────────────────────────────

    #[test]
    fn tokenize_chinese_chars_are_individual_words() {
        // Each ideograph is its own word *and* is separated like a
        // whitespace-delimited one — upstream breaks words between ideographs,
        // which is what puts the `_|` gap between syllables in `-x`.
        let tokens = tokenize("你好世界");
        assert_eq!(tokens, vec![
            Token::Word("你".to_string()),
            Token::Space,
            Token::Word("好".to_string()),
            Token::Space,
            Token::Word("世".to_string()),
            Token::Space,
            Token::Word("界".to_string()),
        ]);
    }

    #[test]
    fn tokenize_cjk_with_spaces() {
        let tokens = tokenize("你好 世界");
        assert_eq!(tokens, vec![
            Token::Word("你".to_string()),
            Token::Space,
            Token::Word("好".to_string()),
            Token::Space,
            Token::Word("世".to_string()),
            Token::Space,
            Token::Word("界".to_string()),
        ]);
    }

    #[test]
    fn tokenize_mixed_cjk_and_latin() {
        let tokens = tokenize("Hello你好World世界");
        assert_eq!(tokens, vec![
            Token::Word("Hello".to_string()),
            Token::Word("你".to_string()),
            Token::Space,
            Token::Word("好".to_string()),
            Token::Word("World".to_string()),
            Token::Word("世".to_string()),
            Token::Space,
            Token::Word("界".to_string()),
        ]);
    }

    #[test]
    fn tokenize_single_cjk_char() {
        let tokens = tokenize("你");
        assert_eq!(tokens, vec![Token::Word("你".to_string())]);
    }

    #[test]
    fn tokenize_cjk_with_punctuation() {
        let tokens = tokenize("你好，世界！");
        assert!(tokens.contains(&Token::Word("你".to_string())));
        assert!(tokens.contains(&Token::Word("好".to_string())));
        assert!(tokens.contains(&Token::Word("世".to_string())));
        assert!(tokens.contains(&Token::Word("界".to_string())));
    }

    // ── ordinal numbers ───────────────────────────────────────────────────

    fn run_ipa_table(lang: &str, dict_name: &str, cases: &[(&str, &str)]) {
        let dict_path = format!("espeak-ng-data/{dict_name}");
        if !Path::new(&dict_path).exists() { return; }
        let t = Translator::new_default(lang).unwrap();
        for &(input, expected) in cases {
            let ipa = t.text_to_ipa(input).unwrap();
            assert_eq!(ipa, expected, "lang={lang} input={input:?}");
        }
    }

    #[test]
    fn text_to_ipa_english_rule_regressions() {
        run_ipa_table("en", "en_dict", &[
            ("sky", "skˈaɪ"),
            ("caused", "kˈɔːzd"),
            ("reflection", "ɹɪflˈɛkʃən"),
            ("droplets", "dɹˈɒplɪts"),
            ("appearing", "ɐpˈiəɹɪŋ"),
            ("meteorological", "mˌiːtɪˌɔːɹəlˈɒdʒɪkəl"),
        ]);
    }

    #[test]
    fn text_to_ipa_english_sentence_weak_forms() {
        let t = Translator::new_default("en").unwrap();
        if !Path::new("/usr/share/espeak-ng-data/en_dict").exists() { return; }
        let ipa = t.text_to_ipa("A rainbow is a meteorological phenomenon that is caused by reflection, refraction and dispersion of light in water droplets resulting in a spectrum of light appearing in the sky.").unwrap();
        assert_eq!(
            ipa,
            "ɐ ɹˈeɪnbəʊ ɪz ɐ mˌiːtɪˌɔːɹəlˈɒdʒɪkəl fɪnˈɒmɪnən ðat ɪz kˈɔːzd baɪ ɹɪflˈɛkʃən\nɹɪfɹˈakʃən and dɪspˈɜːʃən ɒv lˈaɪt ɪn wˈɔːtə dɹˈɒplɪts ɹɪzˈʌltɪŋ ɪn ɐ spˈɛktɹəm ɒv lˈaɪt ɐpˈiəɹɪŋ ɪnðə skˈaɪ"
        );
    }

    #[test]
    fn ordinals_english() {
        run_ipa_table("en", "en_dict", &[
            ("1st",  "fˈɜːst"),
            ("2nd",  "sˈɛkənd"),
            ("3rd",  "θˈɜːd"),
            ("4th",  "fˈɔːθ"),
            ("21st", "twˈɛnti fˈɜːst"),
            ("100th","wˈɒnhˈʌndɹɪd θ"),
        ]);
    }

    #[test]
    fn ordinals_english_large_scales() {
        run_ipa_table("en", "en_dict", &[
            ("1000th",    "wˈɒn θˈaʊzənd θ"),
            ("1001st",    "wˈɒn θˈaʊzənd ən fˈɜːst"),
            ("1000000th", "wˈɒn mˈɪliən θ"),
        ]);
    }

    #[test]
    fn ordinals_spanish() {
        run_ipa_table("es", "es_dict", &[
            ("1º",   "pɾimˈɛɾo"),
            ("21º",  "βixˌɛsimˌopɾimˈɛɾo"),
            ("100º", "θɛntˈɛsim o"),
        ]);
    }

    #[test]
    fn ordinals_spanish_large_scale_do_not_use_hundred_root() {
        let dict_path = "espeak-ng-data/es_dict";
        if !Path::new(dict_path).exists() { return; }
        let data_dir = Path::new("espeak-ng-data");
        let dict = Dictionary::load("es", data_dir).unwrap();
        let mut phdata = PhonemeData::load(data_dir).unwrap();
        phdata.select_table_by_name("es").unwrap();
        let stress_opts = StressOpts::for_lang("es");
        let grammar = LangOptions::for_lang("es").number_grammar;
        let ordinal = OrdinalNumber {
            digits: "1000000".to_string(),
            marker: OrdinalMarker::Suffix("º".to_string()),
        };
        let result = try_ordinal_number(&ordinal, &dict, &phdata, &stress_opts, &grammar).unwrap();
        let hundred_ordinal_lookup = lookup_num_phonemes(&dict, "_0Co");
        let hundred_ordinal = trim_lookup(&hundred_ordinal_lookup);
        assert!(
            !contains_subsequence(&result.phonemes, hundred_ordinal),
            "1000000º should not be built from the hundredth root",
        );
    }

    #[test]
    fn ordinals_dutch() {
        // ordinal_indicator="e" mechanism
        run_ipa_table("nl", "nl_dict", &[
            ("1e", " ˈɪːrstə"),
            ("3e", "dˈɛrdə"),
        ]);
    }

    #[test]
    fn ordinals_german_dot() {
        // NUM_ORDINAL_DOT mechanism.  A compound ordinal takes the suffix on the
        // whole cardinal ("einundzwanzigste"), not tens-ord + units-ord (which
        // gave the wrong two-word "zwanzig erste").
        run_ipa_table("de", "de_dict", &[
            ("1. mal",  "ˈeːɾstə mˈɑːl"),
            ("3. mal",  "dɾˈɪtə mˈɑːl"),
            ("20. mal", "tsvˈantsɪçstə mˈɑːl"),
            // "einundzwanzigste" — the unit "1" apocopates to "ein" (not "eins").
            ("21. mal", "ˈaɪn ʊnttsvˈantsɪçstə mˈɑːl"),
            // Before the end of the clause C drops the dot: a cardinal.
            ("3.", "dɾˈaɪ"),
        ]);
    }

    #[test]
    fn cardinals_1234567() {
        // C espeak-ng oracle output for "1234567".
        // Remaining diffs from oracle are stress placement (ˈ vs ˌ) and minor
        // phoneme variations, not number structure issues.
        let cases: &[(&str, &str, &str, &str)] = &[
            // (lang, dict, rust_output, c_oracle)
            ("en", "en_dict",
             "wˈɒn mˈɪliən tˈuːhˈʌndɹɪdən θˈɜːti fˈɔː θˈaʊzənd fˈaɪvhˈʌndɹɪdən sˈɪksti sˈɛvən",
             "wˈɒn mˈɪliən tˈuːhˈʌndɹɪdən θˈɜːti fˈɔː θˈaʊzənd fˈaɪvhˈʌndɹɪdən sˈɪksti sˈɛvən"),
            ("es", "es_dict",
             "ˈunmiʝˈon dosθjˈentos tɾˌeɪntaikwˈatɾo mˈil kinjˈɛntos sɛsˌɛntaisjˈete",
             "ˈunmiʝˈon dosθjˈentos tɾˌeɪntaikwˈatɾo mˈil kinjˈɛntos sɛsˌɛntaisjˈete"),
            ("fr", "fr_dict",
             "œ̃ miljˈɔ̃ døsɑ̃ tʁɑ̃tkatʁ mˈil sɛ̃ksɑ̃ swasɑ̃tsˈɛt",
             "œ̃ miljˈɔ̃ døsɑ̃ tʁɑ̃tkatʁ mˈil sɛ̃ksɑ̃ swasɑ̃tsˈɛt"),
            ("de", "de_dict",
             "ˈaɪnə mɪljˈoːn tsvˈaɪhˈʊndɜt fˈiːɾ ʊntdɾˈaɪsɪç tˈaʊzənt fˈʏnfhˈʊndɜt zˈiːbən ʊntzˈɛçtsɪç",
             "ˈaɪnə mɪljˈoːn tsvˈaɪhˈʊndɜt fˈiːɾ ʊntdɾˈaɪsɪç tˈaʊzənt fˈynfhˈʊndɜt zˈiːbən ʊntzˈɛçtsɪç"),
            ("nl", "nl_dict",
             " ˈeːn mˌiljun tʋˈeːhˌɔndərt vˌirɛndˌɛrtəx dˌœyzɛnt vˈɛɪfhˌɔndərt zˌeːvənɛnzˌɛstəx",
             "ˈeːn mˌiljun tʋˈeːhˌɔndərt vˌirɛndˌɛrtəx dˌœyzɛnt vˈɛɪfhˌɔndərt zˌeːvənɛnzˌɛstəx"),
        ];
        for &(lang, dict, expected, _oracle) in cases {
            let dict_path = format!("espeak-ng-data/{dict}");
            if !Path::new(&dict_path).exists() { continue; }
            let t = Translator::new_default(lang).unwrap();
            let ipa = t.text_to_ipa("1234567").unwrap();
            assert_eq!(ipa, expected, "lang={lang} input=\"1234567\"");
        }
    }

    #[test]
    fn cardinals_english_billion_scale() {
        let dict_path = "espeak-ng-data/en_dict";
        if !Path::new(dict_path).exists() { return; }
        let dict = Dictionary::load("en", Path::new("espeak-ng-data")).unwrap();
        let grammar = LangOptions::for_lang("en").number_grammar;
        let pronunciation = cardinal_pronunciation("1000000000", &dict, &grammar).unwrap();
        let billion_lookup = lookup_num_phonemes(&dict, "_0M3");
        let billion = trim_lookup(&billion_lookup);
        assert!(!billion.is_empty(), "en_dict is missing _0M3");
        let trimmed = &pronunciation.bytes[..pronunciation.trimmed_len()];
        assert!(
            trimmed.windows(billion.len()).any(|window| window == billion),
            "1000000000 should include the billion scale phonemes",
        );
    }

    #[test]
    fn cardinals_french() {
        let dict_path = "espeak-ng-data/fr_dict";
        if !Path::new(dict_path).exists() { return; }
        let t = Translator::new_default("fr").unwrap();
        for input in ["1", "2", "3", "4", "20", "80", "87", "100", "101"] {
            let ipa = t.text_to_ipa(input).unwrap();
            assert!(!ipa.is_empty(), "fr {input} produced empty IPA");
            assert!(!ipa.chars().any(|c| c.is_ascii_digit()),
                "fr {input} has raw digits in IPA: {ipa}");
        }
    }

    #[test]
    fn portuguese_cardinal_generation() {
        // The pt dict lacks number keys, so cardinals are generated as text and
        // pronounced by the rules (data-free check of the generator).
        let cases = [
            ("0", "zero"), ("1", "um"), ("3", "três"), ("5", "cinco"),
            ("15", "quinze"), ("16", "dezasseis"), ("21", "vinte e um"),
            ("100", "cem"), ("101", "cento e um"), ("123", "cento e vinte e três"),
            ("200", "duzentos"), ("234", "duzentos e trinta e quatro"),
            ("999", "novecentos e noventa e nove"),
            ("1000", "mil"), ("1001", "mil e um"), ("1100", "mil e cem"),
            ("1120", "mil cento e vinte"),        // no "e" (120 not <100/round)
            ("1500", "mil e quinhentos"),         // "e" (round hundred)
            ("2000", "dois mil"), ("2020", "dois mil e vinte"),
            ("2234", "dois mil duzentos e trinta e quatro"),
            ("100000", "cem mil"),
            ("1000000", "um milhão"), ("2000000", "dois milhões"),
            ("1500000", "um milhão e quinhentos mil"),
            ("2000001", "dois milhões e um"),
        ];
        for (digits, want) in cases {
            assert_eq!(portuguese_cardinal_word(digits).as_deref(), Some(want), "pt {digits}");
        }
        // Above the covered range → None (caller keeps the existing path).
        assert_eq!(portuguese_cardinal_word("1000000000"), None);
    }

    #[test]
    fn comma_decimal_languages_are_configured() {
        // These use a comma decimal (period/space thousands); without it, "3,14"
        // read as "three [pause] fourteen".
        for lang in ["sv", "pl", "cs", "ro", "el", "lv", "is", "ca", "eu", "sq",
                     "bs", "id", "vi", "az", "hy"] {
            let g = NumberGrammar::for_lang(lang);
            assert_eq!(g.decimal_separator, ',', "{lang} decimal");
            // C: Czech and Bosnian have no thousands separator at all.
            let group = if lang == "cs" || lang == "bs" { None } else { Some('.') };
            assert_eq!(g.group_separator, group, "{lang} group");
            // C `NUM_ALLOW_SPACE`: only these accept a space between groups.
            let space = ["sv", "pl", "ro", "id", "az", "hy"].contains(&lang);
            assert_eq!(g.space_group, space, "{lang} space grouping");
        }
        // Period-decimal languages are unchanged — including af, ka and kk, which
        // read "3,14" as two numbers (verified against upstream).
        for lang in ["af", "ka", "kk"] {
            assert_eq!(NumberGrammar::for_lang(lang).decimal_separator, '.', "{lang} decimal");
        }
        assert_eq!(NumberGrammar::for_lang("en").decimal_separator, '.');
        assert_eq!(NumberGrammar::for_lang("en").group_separator, Some(','));
    }
}
