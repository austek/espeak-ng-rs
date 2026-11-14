//! The 32-bit clause terminator of C `ReadClause` (translate.h `CLAUSE_*`): pause in units of
//! 10 ms, intonation type, optional-space flag, phrase type and the flag bits.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Terminator(u32);

const PAUSE_MASK: u32 = 0x0000_0fff;
const INTONATION_MASK: u32 = 0x0000_7000;
const TYPE_EOF: u32 = 0x0001_0000;
const TYPE_VOICE_CHANGE: u32 = 0x0002_0000;
const TYPE_SENTENCE: u32 = 0x0008_0000;
const INTONATION_COMMA: u32 = 0x1000;
const INTONATION_QUESTION: u32 = 0x2000;
const INTONATION_EXCLAMATION: u32 = 0x3000;
const INTONATION_NONE: u32 = 0x4000;
const TYPE_CLAUSE: u32 = 0x0004_0000;

impl Terminator {
    pub const NONE: Terminator = Terminator(INTONATION_NONE);
    pub const PARAGRAPH: Terminator = Terminator(70 | TYPE_SENTENCE);
    pub const EOF: Terminator = Terminator(40 | TYPE_SENTENCE | TYPE_EOF);
    pub const VOICE: Terminator = Terminator(INTONATION_NONE | TYPE_VOICE_CHANGE);
    pub const PERIOD: Terminator = Terminator(40 | TYPE_SENTENCE);
    pub const COMMA: Terminator = Terminator(20 | INTONATION_COMMA | TYPE_CLAUSE);
    pub const SHORTCOMMA: Terminator = Terminator(4 | INTONATION_COMMA | TYPE_CLAUSE);
    pub const SHORTFALL: Terminator = Terminator(4 | TYPE_CLAUSE);
    pub const QUESTION: Terminator = Terminator(40 | INTONATION_QUESTION | TYPE_SENTENCE);
    pub const EXCLAMATION: Terminator = Terminator(45 | INTONATION_EXCLAMATION | TYPE_SENTENCE);
    pub const COLON: Terminator = Terminator(30 | TYPE_CLAUSE);
    pub const SEMICOLON: Terminator = Terminator(30 | INTONATION_COMMA | TYPE_CLAUSE);

    pub const OPTIONAL_SPACE_AFTER: Terminator = Terminator(0x0000_8000);
    pub const PUNCTUATION_IN_WORD: Terminator = Terminator(0x0010_0000);
    pub const SPEAK_PUNCTUATION_NAME: Terminator = Terminator(0x0020_0000);
    pub const DOT_AFTER_LAST_WORD: Terminator = Terminator(0x0040_0000);
    pub const PAUSE_LONG: Terminator = Terminator(0x0080_0000);

    pub const fn from_bits(bits: u32) -> Terminator {
        Terminator(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn with(self, flag: Terminator) -> Terminator {
        Terminator(self.0 | flag.0)
    }

    pub const fn without(self, flag: Terminator) -> Terminator {
        Terminator(self.0 & !flag.0)
    }

    pub const fn has(self, flag: Terminator) -> bool {
        self.0 & flag.0 != 0
    }

    pub const fn plus_pause(self, units: u32) -> Terminator {
        Terminator(self.0.wrapping_add(units))
    }

    pub const fn pause_units(self) -> u32 {
        self.0 & PAUSE_MASK
    }

    pub const fn pause_ms(self) -> u32 {
        let ms = self.pause_units() * 10;
        if self.has(Self::PAUSE_LONG) { ms * 32 } else { ms }
    }

    pub const fn intonation(self) -> u8 {
        ((self.0 & INTONATION_MASK) >> 12) as u8
    }

    pub const fn is_sentence(self) -> bool {
        self.0 & TYPE_SENTENCE != 0
    }

    pub const fn is_eof(self) -> bool {
        self.0 & TYPE_EOF != 0
    }

    pub const fn is_voice_change(self) -> bool {
        self.0 & TYPE_VOICE_CHANGE != 0
    }

    pub const fn dot_after_last_word(self) -> bool {
        self.has(Self::DOT_AFTER_LAST_WORD)
    }

    /// C `clause_type_from_codepoint`; `None` where C returns `CLAUSE_NONE`.
    pub fn from_codepoint(c: u32) -> Option<Terminator> {
        let (kind, opt, in_word) = properties(c)?;
        let base = match kind {
            Kind::Period => Self::PERIOD,
            Kind::Question => Self::QUESTION,
            Kind::Exclamation => Self::EXCLAMATION,
            Kind::Comma => Self::COMMA,
            Kind::Colon => Self::COLON,
            Kind::Semicolon | Kind::Dash => Self::SEMICOLON,
            Kind::Ellipsis => Self::SEMICOLON.with(Self::SPEAK_PUNCTUATION_NAME).with(Self::OPTIONAL_SPACE_AFTER),
            Kind::Inverted => Self::SEMICOLON.with(Self::OPTIONAL_SPACE_AFTER),
            Kind::Paragraph => Self::PARAGRAPH,
        };
        let base = if opt { base.with(Self::OPTIONAL_SPACE_AFTER) } else { base };
        Some(if in_word { base.with(Self::PUNCTUATION_IN_WORD) } else { base })
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Period,
    Question,
    Exclamation,
    Comma,
    Colon,
    Semicolon,
    Dash,
    Ellipsis,
    Inverted,
    Paragraph,
}

/// Kind, `Optional_Space_After` and `Punctuation_In_Word` of the ucd-tools clause properties.
/// Combinations the C switch does not list (U+2048, U+2049) are absent.
fn properties(c: u32) -> Option<(Kind, bool, bool)> {
    use Kind::*;
    let plain = |k| Some((k, false, false));
    let opt = |k| Some((k, true, false));
    match c {
        0x589 | 0x964 | 0xDF4 | 0xF0D | 0x3002 | 0xFF0E => opt(Period),
        0x3001 | 0xFF0C => opt(Comma),
        0xFF01 => opt(Exclamation),
        0xFF1F => opt(Question),
        0xFF1A => opt(Colon),
        0xFF1B => opt(Semicolon),
        0xA1 | 0xBF => plain(Inverted),
        0x55B | 0x55C => Some((Exclamation, false, true)),
        0x55E => Some((Question, false, true)),
        0x2E | 0x6D4 | 0x701 | 0x704 | 0x1362 | 0x166E | 0x1803 | 0x1809 | 0x2CF9 | 0x2CFE | 0x2E33
        | 0x2E3C | 0xA4FF | 0xA60E | 0xA6F3 | 0xFE12 | 0xFE52 | 0xFF61 | 0x16AF5 | 0x1BC9F | 0x1DA88
        | 0x1F100 | 0xE002E | 0x2488..=0x249B => plain(Period),
        0x3F | 0x37E | 0x61F | 0x709 | 0x1367 | 0x1945 | 0x2047 | 0xA60F | 0xA6F7 | 0xFE16 | 0xFE56
        | 0x11143 | 0x1E95F | 0xE003F | 0x2753..=0x2754 | 0x2CFA..=0x2CFB => plain(Question),
        0x21 | 0x703 | 0x7F9 | 0x1944 | 0x203C | 0x2755 | 0x2757 | 0xFE15 | 0xFE57 | 0x1E95E | 0xE0021
        | 0x2762..=0x2763 => plain(Exclamation),
        0x2C | 0x55D | 0x60C | 0x702 | 0x7F8 | 0x1363 | 0x1802 | 0x1808 | 0x2E32 | 0x2E34 | 0x2E41
        | 0xA4FE | 0xA60D | 0xA6F5 | 0xF14 | 0xFF64 | 0x1144D | 0x1DA87 | 0xE002C | 0xFE10..=0xFE11
        | 0xFE50..=0xFE51 | 0x1F101..=0x1F10A => plain(Comma),
        0x3A | 0x1804 | 0x2982 | 0xA6F4 | 0xE003A | 0xFE13 | 0xFE55 | 0x1DA8A | 0x706..=0x707
        | 0x1365..=0x1366 | 0x12471..=0x12472 => plain(Colon),
        0x3B | 0x387 | 0x61B | 0x708 | 0x1364 | 0x204F | 0x2E35 | 0xA6F6 | 0xE003B | 0xFE14 | 0xFE54
        | 0x1DA89 => plain(Semicolon),
        0x2013..=0x2014 | 0x2E3A..=0x2E3B | 0xFE31..=0xFE32 => plain(Dash),
        0xEAF | 0x1801 | 0x2026 | 0x22EE..=0x22F1 | 0xFE19 => plain(Ellipsis),
        0x700 | 0x965 | 0xF0E | 0x10FB | 0x1368 | 0x2029 => plain(Paragraph),
        _ => None,
    }
}
