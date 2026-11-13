//! `clause_type_from_codepoint` (readclause.c): the pause, intonation and
//! sentence flag C gives each clause terminator, from the `ESPEAKNG_PROPERTY_*`
//! clause properties of `ucd-tools/src/proplist.c`.

/// What ends a clause, grouped by the `CLAUSE_*` constant C returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Period,
    Question,
    Exclamation,
    Comma,
    Colon,
    Semicolon,
    Paragraph,
}

/// Clause pause in ms, intonation type and whether the clause ends a sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClauseType {
    pub pause: i32,
    pub tone: usize,
    pub sentence: bool,
}

const fn ct(pause: i32, tone: usize, sentence: bool) -> ClauseType {
    ClauseType { pause, tone, sentence }
}

fn kind_of(c: char) -> Option<Kind> {
    use Kind::*;
    let kind = match c as u32 {
        0x2E | 0x6D4 | 0x701 | 0x704 | 0x1362 | 0x166E | 0x1803 | 0x1809 | 0x2CF9 | 0x2CFE | 0x2E33
        | 0x2E3C | 0xA4FF | 0xA60E | 0xA6F3 | 0xFE12 | 0xFE52 | 0xFF61 | 0x16AF5 | 0x1BC9F
        | 0x1DA88 | 0x1F100 | 0xE002E | 0x2488..=0x249B => Period,
        0x589 | 0x964 | 0xDF4 | 0xF0D | 0x3002 | 0xFF0E => Period,
        0x3F | 0x37E | 0x61F | 0x709 | 0x1367 | 0x1945 | 0x2047 | 0xA60F | 0xA6F7 | 0xFE16 | 0xFE56
        | 0x11143 | 0x1E95F | 0xE003F | 0x2753..=0x2754 | 0x2CFA..=0x2CFB | 0x55E | 0xFF1F => Question,
        0x21 | 0x703 | 0x7F9 | 0x1944 | 0x203C | 0x2755 | 0x2757 | 0xFE15 | 0xFE57 | 0x1E95E
        | 0xE0021 | 0x2762..=0x2763 | 0x55B..=0x55C | 0xFF01 => Exclamation,
        0x2C | 0x55D | 0x60C | 0x702 | 0x7F8 | 0x1363 | 0x1802 | 0x1808 | 0x2E32 | 0x2E34 | 0x2E41
        | 0xA4FE | 0xA60D | 0xA6F5 | 0xF14 | 0xFF64 | 0x1144D | 0x1DA87 | 0xE002C | 0xFE10..=0xFE11
        | 0xFE50..=0xFE51 | 0x1F101..=0x1F10A | 0x3001 | 0xFF0C => Comma,
        0x3A | 0x1804 | 0x2982 | 0xA6F4 | 0xE003A | 0xFE13 | 0xFE55 | 0x1DA8A | 0xFF1A
        | 0x706..=0x707 | 0x1365..=0x1366 | 0x12471..=0x12472 => Colon,
        0x3B | 0x387 | 0x61B | 0x708 | 0x1364 | 0x204F | 0x2E35 | 0xA6F6 | 0xE003B | 0xFE14 | 0xFE54
        | 0x1DA89 | 0x2013..=0x2014 | 0x2E3A..=0x2E3B | 0xFE31..=0xFE32 | 0xA1 | 0xBF | 0xFF1B
        | 0xEAF | 0x1801 | 0x2026 | 0x22EE..=0x22F1 | 0xFE19 => Semicolon,
        0x700 | 0x965 | 0xF0E | 0x10FB | 0x1368 | 0x2029 => Paragraph,
        _ => return None,
    };
    Some(kind)
}

/// The clause type C assigns to `c`, or `None` when `c` ends no clause.
pub fn clause_type(c: char) -> Option<ClauseType> {
    kind_of(c).map(|k| match k {
        Kind::Period => ct(400, 0, true),
        Kind::Question => ct(400, 2, true),
        Kind::Exclamation => ct(450, 3, true),
        Kind::Comma => ct(200, 1, false),
        Kind::Colon => ct(300, 0, false),
        Kind::Semicolon => ct(300, 1, false),
        Kind::Paragraph => ct(700, 0, true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_c_constants() {
        assert_eq!(clause_type('，'), Some(ct(200, 1, false)));
        assert_eq!(clause_type('！'), Some(ct(450, 3, true)));
        assert_eq!(clause_type('॥'), Some(ct(700, 0, true)));
        assert_eq!(clause_type('…'), Some(ct(300, 1, false)));
        assert_eq!(clause_type('a'), None);
    }
}
