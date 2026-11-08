//! The accented-letter tables of C's `LookupAccentedLetter` (`numbers.c`).
//!
//! A letter such as `é` has no name of its own: it is spoken as the name of its
//! base letter plus the name of its accent (`e` + "acute"), both taken from
//! the language's dictionary (`_e`, `_acu`).

const M_LIGATURE: u16 = 0x8000;

pub const M_NAME: u16 = 0;
const M_SMALLCAP: u16 = 1;
const M_TURNED: u16 = 2;
const M_REVERSED: u16 = 3;
const M_CURL: u16 = 4;
const M_ACUTE: u16 = 5;
const M_BREVE: u16 = 6;
const M_CARON: u16 = 7;
const M_CEDILLA: u16 = 8;
const M_CIRCUMFLEX: u16 = 9;
const M_DIAERESIS: u16 = 10;
const M_DOUBLE_ACUTE: u16 = 11;
const M_DOT_ABOVE: u16 = 12;
const M_GRAVE: u16 = 13;
const M_MACRON: u16 = 14;
const M_OGONEK: u16 = 15;
const M_RING: u16 = 16;
const M_STROKE: u16 = 17;
const M_TILDE: u16 = 18;
const M_BAR: u16 = 19;
const M_RETROFLEX: u16 = 20;
const M_HOOK: u16 = 21;
const M_MIDDLE_DOT: u16 = M_DOT_ABOVE;
const M_IMPLOSIVE: u16 = M_HOOK;

/// Dictionary keys of the accent names, indexed by modifier; the flag is
/// C's `accent_flags` bit 0 (say the accent before the letter name).
const ACCENT_NAMES: [(&str, bool); 22] = [
    ("_lig", true), ("_smc", false), ("_tur", false), ("_rev", false), ("_crl", false),
    ("_acu", false), ("_brv", false), ("_hac", false), ("_ced", false), ("_cir", false),
    ("_dia", false), ("_ac2", false), ("_dot", false), ("_grv", false), ("_mcn", false),
    ("_ogo", false), ("_rng", false), ("_stk", false), ("_tld", false), ("_bar", false),
    ("_rfx", false), ("_hok", false),
];

const L_ALPHA: u8 = 60;
const L_SCHWA: u8 = 61;
const L_OPEN_E: u8 = 62;
const L_GAMMA: u8 = 63;
const L_IOTA: u8 = 64;
const L_PHI: u8 = 67;
const L_ESH: u8 = 68;
const L_UPSILON: u8 = 69;
const L_EZH: u8 = 70;
const L_GLOTTAL: u8 = 71;
const L_RTAP: u8 = 72;
const L_RLONG: u8 = 73;

/// Letters beyond ASCII that a table entry's base letter can name.
const NON_ASCII: [u32; 15] = [
    0, 0x3b1, 0x259, 0x25b, 0x3b3, 0x3b9, 0x153, 0x3c9, 0x3c6, 0x283, 0x3c5, 0x292, 0x294, 0x27e,
    0x27c,
];

const fn letter(ch: u8, mod1: u16, mod2: u16) -> u16 {
    (ch as u16 - 59) + (mod1 << 6) + (mod2 << 11)
}

const fn ligature(ch1: u8, ch2: u8, mod1: u16) -> u16 {
    (ch1 as u16 - 59) + ((ch2 as u16 - 59) << 6) + (mod1 << 12) + M_LIGATURE
}

/// U+00E0 to U+017F; `0` marks a capital or a character with no entry.
const LETTER_ACCENTS_0E0: [u16; 160] = [
    letter(b'a', M_GRAVE, 0), letter(b'a', M_ACUTE, 0), letter(b'a', M_CIRCUMFLEX, 0),
    letter(b'a', M_TILDE, 0), letter(b'a', M_DIAERESIS, 0), letter(b'a', M_RING, 0),
    ligature(b'a', b'e', 0), letter(b'c', M_CEDILLA, 0), letter(b'e', M_GRAVE, 0),
    letter(b'e', M_ACUTE, 0), letter(b'e', M_CIRCUMFLEX, 0), letter(b'e', M_DIAERESIS, 0),
    letter(b'i', M_GRAVE, 0), letter(b'i', M_ACUTE, 0), letter(b'i', M_CIRCUMFLEX, 0),
    letter(b'i', M_DIAERESIS, 0), letter(b'd', M_NAME, 0), letter(b'n', M_TILDE, 0),
    letter(b'o', M_GRAVE, 0), letter(b'o', M_ACUTE, 0), letter(b'o', M_CIRCUMFLEX, 0),
    letter(b'o', M_TILDE, 0), letter(b'o', M_DIAERESIS, 0), 0, letter(b'o', M_STROKE, 0),
    letter(b'u', M_GRAVE, 0), letter(b'u', M_ACUTE, 0), letter(b'u', M_CIRCUMFLEX, 0),
    letter(b'u', M_DIAERESIS, 0), letter(b'y', M_ACUTE, 0), letter(b't', M_NAME, 0),
    letter(b'y', M_DIAERESIS, 0),
    0, letter(b'a', M_MACRON, 0), 0, letter(b'a', M_BREVE, 0), 0, letter(b'a', M_OGONEK, 0),
    0, letter(b'c', M_ACUTE, 0), 0, letter(b'c', M_CIRCUMFLEX, 0), 0, letter(b'c', M_DOT_ABOVE, 0),
    0, letter(b'c', M_CARON, 0), 0, letter(b'd', M_CARON, 0),
    0, letter(b'd', M_STROKE, 0), 0, letter(b'e', M_MACRON, 0), 0, letter(b'e', M_BREVE, 0),
    0, letter(b'e', M_DOT_ABOVE, 0), 0, letter(b'e', M_OGONEK, 0), 0, letter(b'e', M_CARON, 0),
    0, letter(b'g', M_CIRCUMFLEX, 0), 0, letter(b'g', M_BREVE, 0),
    0, letter(b'g', M_DOT_ABOVE, 0), 0, letter(b'g', M_CEDILLA, 0), 0, letter(b'h', M_CIRCUMFLEX, 0),
    0, letter(b'h', M_STROKE, 0), 0, letter(b'i', M_TILDE, 0), 0, letter(b'i', M_MACRON, 0),
    0, letter(b'i', M_BREVE, 0), 0, letter(b'i', M_OGONEK, 0),
    0, letter(b'i', M_NAME, 0), 0, ligature(b'i', b'j', 0), 0, letter(b'j', M_CIRCUMFLEX, 0),
    0, letter(b'k', M_CEDILLA, 0), letter(b'k', M_NAME, 0), 0, letter(b'l', M_ACUTE, 0),
    0, letter(b'l', M_CEDILLA, 0), 0, letter(b'l', M_CARON, 0), 0, letter(b'l', M_MIDDLE_DOT, 0),
    0, letter(b'l', M_STROKE, 0), 0, letter(b'n', M_ACUTE, 0), 0, letter(b'n', M_CEDILLA, 0),
    0, letter(b'n', M_CARON, 0), letter(b'n', M_NAME, 0), 0, letter(b'n', M_NAME, 0),
    0, letter(b'o', M_MACRON, 0), 0, letter(b'o', M_BREVE, 0),
    0, letter(b'o', M_DOUBLE_ACUTE, 0), 0, ligature(b'o', b'e', 0), 0, letter(b'r', M_ACUTE, 0),
    0, letter(b'r', M_CEDILLA, 0), 0, letter(b'r', M_CARON, 0), 0, letter(b's', M_ACUTE, 0),
    0, letter(b's', M_CIRCUMFLEX, 0), 0, letter(b's', M_CEDILLA, 0),
    0, letter(b's', M_CARON, 0), 0, letter(b't', M_CEDILLA, 0), 0, letter(b't', M_CARON, 0),
    0, letter(b't', M_STROKE, 0), 0, letter(b'u', M_TILDE, 0), 0, letter(b'u', M_MACRON, 0),
    0, letter(b'u', M_BREVE, 0), 0, letter(b'u', M_RING, 0),
    0, letter(b'u', M_DOUBLE_ACUTE, 0), 0, letter(b'u', M_OGONEK, 0), 0, letter(b'w', M_CIRCUMFLEX, 0),
    0, letter(b'y', M_CIRCUMFLEX, 0), 0, 0, letter(b'z', M_ACUTE, 0), 0, letter(b'z', M_DOT_ABOVE, 0),
    0, letter(b'z', M_CARON, 0), letter(b's', M_NAME, 0),
];

/// U+0250 to U+02A8.
const LETTER_ACCENTS_250: [u16; 89] = [
    letter(b'a', M_TURNED, 0), letter(L_ALPHA, 0, 0), letter(L_ALPHA, M_TURNED, 0),
    letter(b'b', M_IMPLOSIVE, 0), 0, letter(b'c', M_CURL, 0), letter(b'd', M_RETROFLEX, 0),
    letter(b'd', M_IMPLOSIVE, 0), letter(b'e', M_REVERSED, 0), 0, letter(L_SCHWA, M_HOOK, 0), 0,
    letter(L_OPEN_E, M_REVERSED, 0), letter(L_OPEN_E, M_HOOK, M_REVERSED), 0, letter(b'j', M_BAR, 0),
    letter(b'g', M_IMPLOSIVE, 0), letter(b'g', 0, 0), letter(b'g', M_SMALLCAP, 0),
    letter(L_GAMMA, 0, 0), 0, letter(b'h', M_TURNED, 0), letter(b'h', M_HOOK, 0), 0,
    letter(b'i', M_BAR, 0), letter(L_IOTA, 0, 0), letter(b'i', M_SMALLCAP, 0),
    letter(b'l', M_TILDE, 0), letter(b'l', M_BAR, 0), letter(b'l', M_RETROFLEX, 0),
    ligature(b'l', b'z', 0), letter(b'm', M_TURNED, 0), 0, letter(b'm', M_HOOK, 0), 0,
    letter(b'n', M_RETROFLEX, 0), letter(b'n', M_SMALLCAP, 0), letter(b'o', M_BAR, 0),
    ligature(b'o', b'e', M_SMALLCAP), 0, letter(L_PHI, 0, 0), letter(b'r', M_TURNED, 0),
    letter(L_RLONG, M_TURNED, 0), letter(b'r', M_RETROFLEX, M_TURNED), 0,
    letter(b'r', M_RETROFLEX, 0), 0, letter(L_RTAP, M_REVERSED, 0), letter(b'r', M_SMALLCAP, 0),
    letter(b'r', M_TURNED, M_SMALLCAP), letter(b's', M_RETROFLEX, 0), 0, letter(b'j', M_HOOK, 0),
    letter(L_ESH, M_REVERSED, 0), letter(L_ESH, M_CURL, 0), letter(b't', M_TURNED, 0),
    letter(b't', M_RETROFLEX, 0), letter(b'u', M_BAR, 0), letter(L_UPSILON, 0, 0),
    letter(b'v', M_HOOK, 0), letter(b'v', M_TURNED, 0), letter(b'w', M_TURNED, 0),
    letter(b'y', M_TURNED, 0), letter(b'y', M_SMALLCAP, 0), letter(b'z', M_RETROFLEX, 0),
    letter(b'z', M_CURL, 0), 0, letter(L_EZH, M_CURL, 0), 0, letter(L_GLOTTAL, M_REVERSED, 0),
    letter(L_GLOTTAL, M_TURNED, 0), 0, 0, letter(b'b', M_SMALLCAP, 0), 0,
    letter(b'g', M_IMPLOSIVE, M_SMALLCAP), letter(b'h', M_SMALLCAP, 0), letter(b'j', M_CURL, 0),
    letter(b'k', M_TURNED, 0), letter(b'l', M_SMALLCAP, 0), letter(b'q', M_HOOK, 0),
    letter(L_GLOTTAL, M_STROKE, 0), letter(L_GLOTTAL, M_STROKE, M_REVERSED), ligature(b'd', b'z', 0),
    0, ligature(b'd', b'z', M_CURL), ligature(b't', b's', 0), 0, ligature(b't', b's', M_CURL),
];

/// How a letter decomposes into base letter(s) and accent names.
#[derive(Debug, PartialEq, Eq)]
pub struct AccentSpec {
    /// The base letter whose name is spoken.
    pub basic: u32,
    /// Second letter of a ligature.
    pub second: Option<u32>,
    /// Dictionary key and say-before flag of the main accent.
    pub accent1: (&'static str, bool),
    /// Dictionary key of the second accent, if any.
    pub accent2: Option<&'static str>,
}

fn base_letter(code: u16) -> u32 {
    let n = code + 59;
    if n < u16::from(b'a') { NON_ASCII[usize::from(n - 59)] } else { u32::from(n) }
}

/// The table entry for `letter`, or `None` when it is a capital, has no entry
/// or is only a letter name (`M_NAME`).
pub fn accent_spec(letter: u32) -> Option<AccentSpec> {
    let data = match letter {
        0xe0..=0x17e => LETTER_ACCENTS_0E0[(letter - 0xe0) as usize],
        0x250..=0x2a8 => LETTER_ACCENTS_250[(letter - 0x250) as usize],
        _ => 0,
    };
    if data == 0 {
        return None;
    }
    let basic = base_letter(data & 0x3f);
    let name = |ix: u16| ACCENT_NAMES[usize::from(ix)];
    if data & M_LIGATURE != 0 {
        let second = base_letter((data >> 6) & 0x3f);
        let accent2 = (data >> 12) & 7;
        return Some(AccentSpec {
            basic,
            second: Some(second),
            accent1: name(0),
            accent2: (accent2 != 0).then(|| name(accent2).0),
        });
    }
    let accent1 = (data >> 6) & 0x1f;
    let accent2 = (data >> 11) & 0xf;
    (accent1 != 0).then(|| AccentSpec {
        basic,
        second: None,
        accent1: name(accent1),
        accent2: (accent2 != 0).then(|| name(accent2).0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_cover_their_ranges() {
        assert_eq!(LETTER_ACCENTS_0E0.len(), 0x180 - 0xe0);
        assert_eq!(LETTER_ACCENTS_250.len(), 0x2a9 - 0x250);
    }

    #[test]
    fn e_acute_is_e_plus_acute() {
        let s = accent_spec(0xe9).unwrap();
        assert_eq!((s.basic, s.accent1.0, s.second), (u32::from(b'e'), "_acu", None));
    }

    #[test]
    fn ae_is_a_ligature() {
        let s = accent_spec(0xe6).unwrap();
        assert_eq!((s.basic, s.second), (u32::from(b'a'), Some(u32::from(b'e'))));
    }

    #[test]
    fn capitals_and_names_have_no_spec() {
        assert!(accent_spec(0xc9).is_none());
        assert!(accent_spec(0xf0).is_none());
        assert!(accent_spec(0x131).is_none());
    }
}
