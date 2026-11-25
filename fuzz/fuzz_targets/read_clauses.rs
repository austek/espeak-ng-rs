#![no_main]

use espeak_ng::translate::readclause::{
    ClauseReader, DefaultLang, ReaderFlags, ReaderOptions, TagProcessor, Terminator,
};
use libfuzzer_sys::fuzz_target;

struct Tags;

impl TagProcessor for Tags {
    fn process_tag(&mut self, tag: &[char], out: &mut String, _: &mut ReaderFlags) -> Option<Terminator> {
        out.push(' ');
        match tag.first() {
            Some('p') => Some(Terminator::PARAGRAPH),
            Some('s') => Some(Terminator::PERIOD),
            Some('b') => Some(Terminator::COLON),
            _ => None,
        }
    }

    fn current_voice_id(&self) -> String {
        String::new()
    }
}

// The first four bytes pick the reader options; the rest is the text. Any
// input must split into clauses without a panic.
fuzz_target!(|data: &[u8]| {
    let [flags, line, punct, buf, rest @ ..] = data else { return };
    let options = ReaderOptions {
        ssml: flags & 1 != 0,
        phoneme_input: flags & 2 != 0,
        linelength: i32::from(*line),
        punctuation: punct % 3,
        punctlist: if flags & 4 != 0 { vec!['.', ',', '!', '?'] } else { Vec::new() },
        capitals: u32::from(flags >> 3 & 3),
        n_buf: 16 + usize::from(*buf) * 3,
        ..ReaderOptions::default()
    };
    let text = String::from_utf8_lossy(rest);
    let _ = ClauseReader::new(&text, options, Tags).read_all(&DefaultLang);
});
