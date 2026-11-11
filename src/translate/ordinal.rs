// src/translate/ordinal.rs
//
// Ordinal numbers, ported from espeak-ng `numbers.c` (`LookupNum2`,
// `LookupNum3`).  The per-language switches are C's `langopts.numbers` and
// `langopts.numbers2` bitmasks, so a language gets the composition C gives it
// (Spanish "vigésimo primero", German "einundzwanzigste", English "hundred
// and first") without a per-language special case here.

use super::{lookup_num_phonemes, trim_lookup, PHON_END_WORD};
use crate::dictionary::file::Dictionary;
use crate::phoneme::load::PhonemeData;
use crate::phoneme::{PHON_PAUSE_SHORT, PHON_STRESS_3, PHON_STRESS_P, PH_VOWEL};

pub(super) const NUM_SWAP_TENS: u32 = 0x10;
pub(super) const NUM_AND_UNITS: u32 = 0x20;
pub(super) const NUM_HUNDRED_AND: u32 = 0x40;
pub(super) const NUM_SINGLE_AND: u32 = 0x80;
pub(super) const NUM_SINGLE_STRESS: u32 = 0x100;
pub(super) const NUM_SINGLE_VOWEL: u32 = 0x200;
pub(super) const NUM_OMIT_1_HUNDRED: u32 = 0x400;
pub(super) const NUM_ORDINAL_DOT: u32 = 0x1_0000;
pub(super) const NUM_AND_HUNDRED: u32 = 0x4_0000;
pub(super) const NUM_THOUSAND_AND: u32 = 0x8_0000;
pub(super) const NUM_VIGESIMAL: u32 = 0x10_0000;
pub(super) const NUM_ZERO_HUNDRED: u32 = 0x40_0000;
pub(super) const NUM_HUNDRED_AND_DIGIT: u32 = 0x80_0000;
pub(super) const NUM_SINGLE_STRESS_L: u32 = 0x1000_0000;

const NUM2_ORDINAL_NO_AND: u32 = 0x800;
const NUM2_MULTIPLE_ORDINAL: u32 = 0x1000;
const NUM2_NO_TEEN_ORDINALS: u32 = 0x2000;
const NUM2_OMIT_1_HUNDRED_ONLY: u32 = 0x2_0000;
const NUM2_ORDINAL_AND_THOUSANDS: u32 = 0x4_0000;
const NUM2_ORDINAL_DROP_VOWEL: u32 = 0x8_0000;
const NUM2_ZERO_TENS: u32 = 0x10_0000;

/// Default `langopts.numbers`: number processing on, nothing else.
const NUM_DEFAULT: u32 = 1;

/// C's `langopts.numbers` and `langopts.numbers2` per language, from
/// `tr_languages.c` (`SetTranslator`).
const C_NUMBER_FLAGS: &[(&str, u32, u32)] = &[
    ("af", 0x10008d0, 0x0),
    ("am", 0x400, 0x0),
    ("an", 0x9200528, 0x800),
    ("ar", 0x2c0470, 0x0),
    ("as", 0x10, 0x0),
    ("az", 0x205508, 0x0),
    ("be", 0x408, 0x42),
    ("bg", 0xb0014e8, 0x0),
    ("bn", 0x10, 0x0),
    ("bpy", 0x1, 0x200),
    ("bs", 0x200454c, 0x14a),
    ("ca", 0x5208528, 0x1800),
    ("cmn", 0xa408, 0x0),
    ("cs", 0x1004400, 0x88),
    ("cy", 0x400, 0x0),
    ("da", 0xb010c58, 0x0),
    ("de", 0x1011018, 0x0),
    ("el", 0x108, 0x1802),
    ("en", 0x1000840, 0x0),
    ("eo", 0x1001408, 0x0),
    ("es", 0x5208528, 0x1800),
    ("et", 0x15008, 0x0),
    ("eu", 0x300548, 0x0),
    ("fa", 0x60, 0x0),
    ("fi", 0x15008, 0x0),
    ("fo", 0xb010c58, 0x0),
    ("fr", 0x7129508, 0x0),
    ("ga", 0x200400, 0x0),
    ("gd", 0x200400, 0x0),
    ("grc", 0x108, 0x1802),
    ("gu", 0x10, 0x0),
    ("he", 0x40e0, 0x0),
    ("hi", 0x10, 0x0),
    ("hr", 0x200454c, 0x14a),
    ("ht", 0x1128500, 0x0),
    ("hu", 0xb21b400, 0x0),
    ("hy", 0x1408, 0x0),
    ("ia", 0x5200408, 0x1800),
    ("id", 0x1001008, 0x0),
    ("io", 0x1001468, 0x0),
    ("is", 0x8e8, 0x2),
    ("it", 0xb002608, 0x2000),
    ("ka", 0x130a420, 0x0),
    ("kk", 0xc400, 0x0),
    ("kl", 0xb010c58, 0x0),
    ("kn", 0x1, 0x0),
    ("ko", 0x400, 0x4000),
    ("ku", 0x40460, 0x0),
    ("ky", 0x1, 0x0),
    ("la", 0x1000000, 0x0),
    ("lt", 0x18408, 0x100),
    ("ltg", 0x18408, 0x0),
    ("lv", 0x18408, 0x0),
    ("mk", 0x204428, 0x8a),
    ("ml", 0x200400, 0x20000),
    ("mn", 0xc400, 0x0),
    ("mr", 0x10, 0x0),
    ("ms", 0x1001008, 0x0),
    ("mt", 0x1, 0x0),
    ("nb", 0x11848, 0x0),
    ("ne", 0x10, 0x0),
    ("nl", 0x211c18, 0x0),
    ("no", 0x11848, 0x0),
    ("om", 0x440, 0x200),
    ("or", 0x10, 0x0),
    ("pa", 0x10, 0x0),
    ("pap", 0x5208528, 0x1800),
    ("pl", 0x5008, 0xc0),
    ("pt", 0x2004068, 0x3800),
    ("ro", 0x1007028, 0x1e),
    ("ru", 0xa408, 0x20_0042),
    ("rup", 0x7008, 0x0),
    ("sd", 0x10, 0x0),
    ("shn", 0x1, 0x0),
    ("si", 0x1020e000, 0x10000),
    ("sk", 0x1004400, 0x80),
    ("sl", 0x1015418, 0x100),
    ("smj", 0x15418, 0x0),
    ("sq", 0x8068, 0x0),
    ("sr", 0x200454c, 0x14a),
    ("sv", 0x1908, 0x0),
    ("sw", 0x4e0, 0x0),
    ("ta", 0x200000, 0x40000),
    ("te", 0x1, 0x80000),
    ("tn", 0x4e0, 0x0),
    ("tr", 0x204508, 0x0),
    ("tt", 0x208508, 0x0),
    ("uk", 0xa408, 0x20_0042),
    ("ur", 0x10, 0x0),
    ("vi", 0xc08008, 0x0),
    ("yue", 0xa408, 0x100000),
    ("zh", 0xa408, 0x0),
];

/// C's `(langopts.numbers, langopts.numbers2)` for the primary language subtag.
pub(super) fn c_number_flags(lang: &str) -> (u32, u32) {
    C_NUMBER_FLAGS
        .iter()
        .find(|(name, _, _)| *name == lang)
        .map_or((NUM_DEFAULT, 0), |&(_, n1, n2)| (n1, n2))
}

const NUM_THOUS_SPACE: u32 = 0x4;
const NUM_DECIMAL_COMMA: u32 = 0x8;
const NUM_ALLOW_SPACE: u32 = 0x1000;

/// C's `(thousands_sep, decimal_sep)` and whether a space also separates
/// thousands.  `SetTranslator` sets Hungarian, Czech and Slovak by hand;
/// `ProcessLanguageOptions` then lets `NUM_DECIMAL_COMMA` and `NUM_THOUS_SPACE`
/// override whatever came before.
pub(super) fn c_separators(lang: &str, numbers: u32) -> (Option<char>, char, bool) {
    let (mut thousands, mut decimal) = match lang {
        "hu" => (Some(' '), ','),
        "cs" | "sk" => (None, ','),
        _ => (Some(','), '.'),
    };
    if numbers & NUM_DECIMAL_COMMA != 0 {
        (thousands, decimal) = (Some('.'), ',');
    }
    if numbers & NUM_THOUS_SPACE != 0 {
        thousands = None;
    }
    let space = thousands == Some(' ') || numbers & NUM_ALLOW_SPACE != 0;
    (thousands.filter(|&c| c != ' '), decimal, space)
}

/// Everything C's ordinal composition reads: the language switches and the
/// ordinal suffix (`_#<suffix>`) the number carries.
pub(super) struct OrdinalNumbers<'a> {
    pub dict: &'a Dictionary,
    pub phdata: &'a PhonemeData,
    pub numbers: u32,
    pub numbers2: u32,
    pub ph_ordinal2: Vec<u8>,
    pub ph_ordinal2x: Vec<u8>,
    /// Whether the number is an ordinal (C's ordinal bit in `LookupNum3`).
    pub ordinal: bool,
    /// Pronunciation of the last digits found together with the next word
    /// (C `digit_lookup`), and how many digits it covers (`n_digit_lookup`).
    pub digit_lookup: Vec<u8>,
    pub digit_lookup_len: usize,
}

fn is_stress_code(code: u8) -> bool {
    (2..=8).contains(&code)
}

impl OrdinalNumbers<'_> {
    fn lk(&self, key: &str) -> Vec<u8> {
        trim_lookup(&lookup_num_phonemes(self.dict, key)).to_vec()
    }

    /// C's `Lookup`: the entry replaces `slot` only when it exists.
    fn lk_into(&self, key: &str, slot: &mut Vec<u8>) -> bool {
        let found = self.lk(key);
        let hit = !found.is_empty();
        if hit {
            *slot = found;
        }
        hit
    }

    fn has(&self, flag: u32) -> bool {
        self.numbers & flag != 0
    }

    fn has2(&self, flag: u32) -> bool {
        self.numbers2 & flag != 0
    }

    fn is_vowel(&self, code: u8) -> bool {
        self.phdata.get(code).is_some_and(|p| p.typ == PH_VOWEL)
    }

    fn limit_stress(&self, out: &mut [u8]) {
        let demote = |out: &mut [u8], order: &mut dyn Iterator<Item = usize>| {
            let mut seen = false;
            for ix in order {
                if out[ix] == PHON_STRESS_P {
                    if seen {
                        out[ix] = PHON_STRESS_3;
                    }
                    seen = true;
                }
            }
        };
        let len = out.len();
        if self.has(NUM_SINGLE_STRESS_L) {
            demote(out, &mut (0..len));
        } else if self.has(NUM_SINGLE_STRESS) {
            demote(out, &mut (0..len).rev());
        }
    }

    /// Tens and units joined by the ordinal rules; returns the phonemes and
    /// whether a conjunction was used.  C `LookupNum2`.
    pub(super) fn num2(&self, value: u32, thousandplex: u32, control: u32) -> (Vec<u8>, bool) {
        let tens = value / 10;
        let mut units = value % 10;
        let ord_type = if control & 0x20 != 0 { 'q' } else { 'o' };
        let is_ordinal = control & 1 != 0;
        let (mut ph_ordinal, mut ph_tens, mut ph_digits) = (Vec::new(), Vec::new(), Vec::new());
        let (mut found, mut found_ordinal) = (false, false);

        if control & 2 != 0 && self.digit_lookup_len == 2 {
            let mut out = self.digit_lookup.clone();
            self.limit_stress(&mut out);
            return (out, false);
        }
        let digit_lookup = !self.digit_lookup.is_empty();

        if digit_lookup {
            // The last digits already have a pronunciation: speak tens and units.
        } else if control & 8 != 0 {
            found = self.lk_into(&format!("_{value}fx"), &mut ph_digits)
                || self.lk_into(&format!("_{value}f"), &mut ph_digits);
        } else if is_ordinal {
            ph_ordinal = self.ph_ordinal2.clone();
            if control & 4 != 0
                && self.lk_into(&format!("_{value}{ord_type}x"), &mut ph_digits)
            {
                found = true;
                if !self.ph_ordinal2x.is_empty() {
                    ph_ordinal = self.ph_ordinal2x.clone();
                }
            }
            if !found {
                found = self.lk_into(&format!("_{value}{ord_type}"), &mut ph_digits);
            }
            found_ordinal = found;
        }

        if !found && !digit_lookup {
            if control & 2 == 0 {
                let variant = if self.has2(NUM2_ORDINAL_AND_THOUSANDS) && thousandplex <= 1 {
                    'o'
                } else {
                    'a'
                };
                found = self.lk_into(&format!("_{value}{variant}"), &mut ph_digits);
            }
            if !found && !(is_ordinal && self.has2(NUM2_NO_TEEN_ORDINALS)) {
                found = self.lk_into(&format!("_{value}"), &mut ph_digits);
            }
        }

        if value < 10 && control & 0x10 != 0 {
            ph_tens = self.lk("_0");
        } else if !found {
            if is_ordinal && self.lk_into(&format!("_{tens}X{ord_type}"), &mut ph_tens) {
                found_ordinal = true;
                if units != 0 && self.has2(NUM2_MULTIPLE_ORDINAL) {
                    ph_tens.extend_from_slice(&self.ph_ordinal2);
                }
            }
            if !found_ordinal {
                let key = if control & 0x200 != 0 { format!("_{tens}Xf") } else { format!("_{tens}X") };
                ph_tens = self.lk(&key);
            }
            if ph_tens.is_empty() && self.has(NUM_VIGESIMAL) {
                units = value % 20;
                ph_tens = self.lk(&format!("_{}X", tens & 0xfe));
            }
            ph_digits.clear();
            if units > 0 && control & 2 != 0 && digit_lookup {
                ph_digits = self.digit_lookup.clone();
                found_ordinal = true;
                ph_ordinal.clear();
            } else if units > 0 {
                let mut hit = false;
                if control & 8 != 0 {
                    hit = self.lk_into(&format!("_{units}f"), &mut ph_digits);
                }
                if is_ordinal && !self.has(NUM_SWAP_TENS)
                    && self.lk_into(&format!("_{units}{ord_type}"), &mut ph_digits)
                {
                    hit = true;
                    found_ordinal = true;
                }
                if !hit && (control & 2 == 0 || self.has(NUM_SWAP_TENS)) {
                    let variant = if self.has2(NUM2_ORDINAL_AND_THOUSANDS) && thousandplex <= 1 {
                        'o'
                    } else {
                        'a'
                    };
                    hit = self.lk_into(&format!("_{units}{variant}"), &mut ph_digits);
                }
                if !hit {
                    self.lk_into(&format!("_{units}"), &mut ph_digits);
                }
            }
        }

        if is_ordinal && !found_ordinal && ph_ordinal.is_empty() {
            if value >= 20 && (value.is_multiple_of(10) || self.has(NUM_SWAP_TENS)) {
                ph_ordinal = self.lk("_ord20");
            }
            if ph_ordinal.is_empty() {
                ph_ordinal = self.lk("_ord");
            }
        }

        let mut used_and = false;
        let mut out;
        if self.has(NUM_SWAP_TENS | NUM_AND_UNITS) && !ph_tens.is_empty() && !ph_digits.is_empty() {
            let mut ph_and = self.lk("_0and");
            if is_ordinal && self.has2(NUM2_ORDINAL_NO_AND) {
                ph_and.clear();
            }
            let (first, second) = if self.has(NUM_SWAP_TENS) {
                (&ph_digits, &ph_tens)
            } else {
                (&ph_tens, &ph_digits)
            };
            out = [first.as_slice(), &ph_and, second, &ph_ordinal].concat();
            used_and = true;
        } else {
            if self.has(NUM_SINGLE_VOWEL) && !ph_digits.is_empty() {
                let next = if is_stress_code(ph_digits[0]) { ph_digits.get(1) } else { ph_digits.first() };
                let next_is_vowel = next.is_some_and(|&c| self.is_vowel(c));
                if next_is_vowel && ph_tens.last().is_some_and(|&c| self.is_vowel(c)) {
                    ph_tens.pop();
                }
            }
            out = [ph_tens.as_slice(), &ph_digits].concat();
            let drop_vowel = self.has2(NUM2_ORDINAL_DROP_VOWEL) && !ph_ordinal.is_empty();
            if drop_vowel && out.last().is_some_and(|&c| self.is_vowel(c)) {
                out.pop();
            }
            out.extend_from_slice(&ph_ordinal);
        }
        self.limit_stress(&mut out);
        (out, used_and)
    }

    /// A number below 1000 as the last word of an ordinal.  The result keeps
    /// C's word separators: hundreds and conjunction, then the tens and units.
    /// C `LookupNum3` with the ordinal bit set; `previous_thousands` says a
    /// higher group has already been spoken.
    pub(super) fn num3(&self, value: u32, previous_thousands: bool, suppress_null: bool) -> Vec<u8> {
        let control = 2 | u32::from(previous_thousands);
        let hundreds = value / 100;
        let tensunits = value % 100;
        let say_zero_hundred = self.has(NUM_ZERO_HUNDRED) && previous_thousands;
        let mut suppress_null = suppress_null;
        let (mut buf1, mut ph_thousand_and) = (Vec::new(), Vec::new());

        if hundreds > 0 || say_zero_hundred {
            let mut ph_100 = Vec::new();
            let mut found = self.ordinal && tensunits == 0 && self.lk_into("_0Co", &mut ph_100);
            if !found {
                found = tensunits == 0 && self.lk_into("_0C0", &mut ph_100);
                if !found {
                    ph_100 = self.lk("_0C");
                }
            }
            let mut ph_digits = Vec::new();
            if self.has(NUM_AND_HUNDRED) && previous_thousands {
                ph_thousand_and = self.lk("_0and");
            }
            suppress_null = true;
            found = false;
            if self.ordinal && (tensunits == 0 || self.has2(NUM2_MULTIPLE_ORDINAL)) {
                found = self.lk_into(&format!("_{hundreds}Co"), &mut ph_digits);
                if self.has2(NUM2_MULTIPLE_ORDINAL) && tensunits > 0 {
                    ph_digits.extend_from_slice(&self.ph_ordinal2);
                }
            }
            if hundreds == 0 && say_zero_hundred {
                ph_digits = self.lk("_0");
            } else {
                let only_after_thousands =
                    hundreds == 1 && self.has2(NUM2_OMIT_1_HUNDRED_ONLY) && !previous_thousands;
                if !only_after_thousands {
                    if !found && tensunits == 0 {
                        found = self.lk_into(&format!("_{hundreds}C0"), &mut ph_digits);
                    }
                    if !found {
                        found = self.lk_into(&format!("_{hundreds}C"), &mut ph_digits);
                    }
                }
                if found {
                    ph_100.clear();
                } else if !(hundreds == 1 && self.has(NUM_OMIT_1_HUNDRED)) {
                    ph_digits = self.num2(hundreds, 0, 0).0;
                }
            }
            buf1 = [ph_thousand_and.as_slice(), &ph_digits, &ph_100].concat();
        }

        let mut ph_hundred_and = Vec::new();
        if tensunits > 0 && !(self.ordinal && self.has2(NUM2_MULTIPLE_ORDINAL)) {
            let wants_and = self.has(NUM_HUNDRED_AND)
                || (self.has(NUM_HUNDRED_AND_DIGIT) && tensunits < 10);
            if (value > 100 || previous_thousands) && wants_and {
                ph_hundred_and = self.lk("_0and");
            }
            if self.has(NUM_THOUSAND_AND) && hundreds == 0 && previous_thousands {
                ph_hundred_and = self.lk("_0and");
            }
        }

        let mut buf2 = Vec::new();
        if tensunits != 0 || !suppress_null {
            let mut x = if self.ordinal { 3 } else { 2 };
            if value < 100 && !previous_thousands {
                x |= 4;
            }
            if self.has2(NUM2_ZERO_TENS) && (previous_thousands || hundreds > 0) {
                x |= 0x10;
            }
            let (ph, used_and) = self.num2(tensunits, 0, x | (control & 0x100));
            buf2 = ph;
            if used_and && self.has(NUM_SINGLE_AND) {
                ph_hundred_and.clear();
            }
        } else if !self.ph_ordinal2.is_empty() {
            if buf1.last() == Some(&PHON_PAUSE_SHORT) {
                buf1.pop();
            }
            buf2 = self.ph_ordinal2.clone();
        }

        [buf1.as_slice(), &ph_hundred_and, &[PHON_END_WORD], &buf2].concat()
    }
}
