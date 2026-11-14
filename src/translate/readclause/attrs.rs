//! SSML attribute helpers from `ssml.c`: `GetSsmlAttribute`, `attrcmp`,
//! `attrlookup`, `attrnumber`, `attrcopy_utf8` and `attr_prosody_value`.

/// C `iswspace` in a UTF-8 locale.
pub fn is_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{1680}' | '\u{2000}'..='\u{2006}' | '\u{2008}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{205f}' | '\u{3000}')
}

/// The text of an attribute value up to the end of the tag buffer, plus the
/// quote that opened it (C reads this back as `pw[-1]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrVal<'a> {
    pub rest: &'a [char],
    pub quote: Option<char>,
}

impl AttrVal<'_> {
    fn at(&self, ix: usize) -> char {
        self.rest.get(ix).copied().unwrap_or('\0')
    }
}

const EMPTY: AttrVal<'static> = AttrVal { rest: &[], quote: None };

fn is_quote(c: char) -> bool {
    c == '"' || c == '\''
}

fn skip_space(buf: &[char], mut p: usize) -> usize {
    while buf.get(p).is_some_and(|&c| is_space(c)) {
        p += 1;
    }
    p
}

fn value_at(buf: &[char], p: usize) -> AttrVal<'_> {
    let p = skip_space(buf, p);
    let p = if buf.get(p) == Some(&'=') { p + 1 } else { p };
    let p = skip_space(buf, p);
    match buf.get(p).copied() {
        Some(q) if is_quote(q) => AttrVal { rest: &buf[p + 1..], quote: Some(q) },
        Some(c) if is_space(c) || c == '/' => EMPTY,
        _ => AttrVal { rest: buf.get(p..).unwrap_or(&[]), quote: None },
    }
}

/// `GetSsmlAttribute`: `buf` is the whole tag buffer and `from` the index
/// after the tag name. Only the prefix of the attribute name is compared, as in C.
pub fn get_ssml_attribute<'a>(buf: &'a [char], from: usize, name: &str) -> Option<AttrVal<'a>> {
    let name: Vec<char> = name.chars().collect();
    let mut p = from;
    while p < buf.len() {
        if p > 0 && is_space(buf[p - 1]) {
            let mut ix = 0;
            while p < buf.len() && ix < name.len() && buf[p] == name[ix] {
                p += 1;
                ix += 1;
            }
            if ix == name.len() {
                return Some(value_at(buf, p));
            }
        }
        if p >= buf.len() {
            break;
        }
        p += 1;
    }
    None
}

/// `attrcmp(...) == 0`: the value equals `name` and is closed by a quote.
pub fn attr_eq(attr: Option<&AttrVal>, name: &str) -> bool {
    let Some(a) = attr else { return false };
    let mut ix = 0;
    for b in name.chars() {
        if a.at(ix) != b {
            return false;
        }
        ix += 1;
    }
    is_quote(a.at(ix))
}

/// `attrlookup`: first table entry the value matches, else `default`.
pub fn attr_lookup(attr: Option<&AttrVal>, table: &[(&str, i32)], default: i32) -> i32 {
    table.iter().find(|(name, _)| attr_eq(attr, name)).map_or(default, |&(_, v)| v)
}

/// `attrnumber`; `seconds` is C `type == 1`: an `s` suffix means seconds.
pub fn attr_number(attr: Option<&AttrVal>, default: i32, seconds: bool) -> i32 {
    let Some(a) = attr.filter(|a| a.at(0).is_ascii_digit()) else { return default };
    let mut ix = 0;
    let mut value: i32 = 0;
    while a.at(ix).is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add(a.at(ix) as i32 - '0' as i32);
        ix += 1;
    }
    let s_suffix = a.at(ix).to_lowercase().next() == Some('s');
    if seconds && s_suffix { value.wrapping_mul(1000) } else { value }
}

/// `attrcopy_utf8`: the value as UTF-8, at most `len - 4` bytes (C buffer size `len`).
pub fn attr_copy(attr: Option<&AttrVal>, len: usize) -> String {
    let Some(a) = attr else { return String::new() };
    let mut out = String::new();
    let mut prev = '\0';
    for &c in a.rest {
        if out.len() >= len.saturating_sub(4) || c == '\0' {
            break;
        }
        match a.quote {
            None if is_space(c) || c == '/' => break,
            Some(q) if c == q && prev != '\\' => break,
            _ => {}
        }
        out.push(c);
        prev = c;
    }
    out
}

/// Length in chars of the `strtod` number at the start of `s`, and its value.
fn parse_double(s: &[char]) -> Option<(f64, usize)> {
    let mut p = 0;
    if matches!(s.first(), Some('+' | '-')) {
        p = 1;
    }
    let digits_from = p;
    while s.get(p).is_some_and(char::is_ascii_digit) {
        p += 1;
    }
    let int_digits = p - digits_from;
    let mut frac_digits = 0;
    if s.get(p) == Some(&'.') {
        let q = p + 1;
        let mut e = q;
        while s.get(e).is_some_and(char::is_ascii_digit) {
            e += 1;
        }
        frac_digits = e - q;
        if int_digits + frac_digits > 0 {
            p = e;
        }
    }
    if int_digits + frac_digits == 0 {
        return None;
    }
    let p = p + exponent_len(&s[p..]);
    let text: String = s[..p].iter().collect();
    text.parse::<f64>().ok().map(|v| (v, p))
}

fn exponent_len(s: &[char]) -> usize {
    if !matches!(s.first(), Some('e' | 'E')) {
        return 0;
    }
    let mut p = 1;
    if matches!(s.get(p), Some('+' | '-')) {
        p += 1;
    }
    let from = p;
    while s.get(p).is_some_and(char::is_ascii_digit) {
        p += 1;
    }
    if p > from { p } else { 0 }
}

/// `attr_prosody_value`: returns `(sign, value)`; `sign == 2` is a percentage.
pub fn prosody_value(is_rate: bool, a: &AttrVal) -> (i32, i32) {
    let mut p = 0;
    while a.rest.get(p).is_some_and(|&c| is_space(c)) {
        p += 1;
    }
    let mut sign = 0;
    if a.rest.get(p) == Some(&'+') {
        p += 1;
        sign = 1;
    }
    if a.rest.get(p) == Some(&'-') {
        p += 1;
        sign = -1;
    }
    while a.rest.get(p).is_some_and(|&c| is_space(c)) {
        p += 1;
    }
    let tail = a.rest.get(p..).unwrap_or(&[]);
    let Some((value, n)) = parse_double(tail) else { return (2, 100) };
    prosody_scaled(is_rate, sign, value, tail.get(n..).unwrap_or(&[]))
}

fn prosody_scaled(is_rate: bool, sign: i32, value: f64, tail: &[char]) -> (i32, i32) {
    if tail.first() == Some(&'%') {
        let v = if sign != 0 { 100.0 + f64::from(sign) * value } else { value };
        return (2, v as i32);
    }
    if tail.starts_with(&['s', 't']) {
        let x = 2f64.powf(value * f64::from(sign) / 12.0) * 100.0;
        return (2, x as i32);
    }
    if is_rate {
        let v = if sign == 0 { (value * 100.0) as i32 } else { 100 + (f64::from(sign) * value * 100.0) as i32 };
        return (2, v);
    }
    (sign, value as i32)
}
