//! The text decoder `ReadClause` reads from. The CLI hands C the text with its terminating NUL,
//! so the decoder always ends in one `\0`; the last real character is read as `c1` only because
//! that NUL is read as `c2`.

pub struct Decoder {
    chars: Vec<char>,
    pos: usize,
}

impl Decoder {
    pub fn new(text: &str) -> Decoder {
        let mut chars: Vec<char> = text.chars().collect();
        chars.push('\0');
        Decoder { chars, pos: 0 }
    }

    pub fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    pub fn getc(&mut self) -> u32 {
        let c = self.chars.get(self.pos).map_or(0, |&c| c as u32);
        self.pos = (self.pos + 1).min(self.chars.len());
        c
    }

    pub fn peekc(&self) -> u32 {
        self.chars.get(self.pos).map_or(0, |&c| c as u32)
    }
}
