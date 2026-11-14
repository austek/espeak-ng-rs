//! Wide-character classes `ReadClause` applies to code points held as `u32` (C `int`).

pub fn ch(c: u32) -> char {
    char::from_u32(c).unwrap_or('\u{fffd}')
}

fn of(c: u32) -> Option<char> {
    char::from_u32(c)
}

pub fn alnum(c: u32) -> bool {
    of(c).is_some_and(char::is_alphanumeric)
}

pub fn alpha(c: u32) -> bool {
    of(c).is_some_and(char::is_alphabetic)
}

pub fn upper(c: u32) -> bool {
    of(c).is_some_and(char::is_uppercase)
}

pub fn lower(c: u32) -> bool {
    of(c).is_some_and(char::is_lowercase)
}

pub fn digit(c: u32) -> bool {
    (0x30..=0x39).contains(&c)
}

pub fn space(c: u32) -> bool {
    of(c).is_some_and(|c| c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}'))
}

pub fn punct(c: u32) -> bool {
    of(c).is_some_and(|c| !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control())
}

/// C `IsAlpha`.
pub fn is_alpha(c: u32) -> bool {
    of(c).is_some_and(crate::translate::ucd_alpha::is_alpha)
}

/// C `IsBracket`.
pub fn is_bracket(c: u32) -> bool {
    c == 0xe000 + '<' as u32 || of(c).is_some_and(crate::translate::is_bracket)
}
