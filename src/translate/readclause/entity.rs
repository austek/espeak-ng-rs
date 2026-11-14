//! C `ParseSsmlReference` (ssml.c:1010): XML character and entity references.

const ENTITIES: [(&str, u32); 6] = [
    ("gt", '>' as u32),
    ("lt", 0xe000 + '<' as u32),
    ("amp", '&' as u32),
    ("quot", '"' as u32),
    ("nbsp", ' ' as u32),
    ("apos", '\'' as u32),
];

/// The replacement character and the adjusted following character, or `None` when `name` is no
/// reference. A named entity turns a NUL `c2` into a space.
pub fn parse_reference(name: &[u32], c2: u32) -> Option<(u32, u32)> {
    let text: String = name.iter().map(|&c| char::from_u32(c).unwrap_or('\0')).collect();
    match text.strip_prefix('#') {
        Some(num) => parse_number(num).map(|c| (c, c2)),
        None => ENTITIES
            .iter()
            .find(|(n, _)| *n == text)
            .map(|&(_, c)| (c, if c2 == 0 { ' ' as u32 } else { c2 })),
    }
}

fn parse_number(num: &str) -> Option<u32> {
    let (digits, radix) = match num.strip_prefix('x') {
        Some(hex) => (hex.strip_prefix("0x").or_else(|| hex.strip_prefix("0X")).unwrap_or(hex), 16),
        None => (num, 10),
    };
    let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    Some(u64::from_str_radix(&digits[..end], radix).map_or(u32::MAX, |v| v as u32))
}
