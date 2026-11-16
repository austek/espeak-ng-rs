//! Pure construction of the callback blocks C delivers: audio cut into
//! fixed-size buffers per clause, with each event attached to its buffer.

use espeak_ng::engine::{EventKind, SynthEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvKind {
    SampleRate(i32),
    Sentence(i32),
    Word(i32),
    Mark(String),
    /// End of a clause; the payload is the number of sentences so far.
    End(i32),
}

/// What the callback blocks need of a [`SynthEvent`].
#[derive(Clone, Debug)]
pub struct Source {
    pub kind: EventKind,
    pub text_position: u32,
    pub length: u32,
    pub sample: u64,
}

impl From<&SynthEvent> for Source {
    fn from(e: &SynthEvent) -> Self {
        Source { kind: e.kind.clone(), text_position: e.text_position, length: e.length, sample: e.sample }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ev {
    pub kind: EvKind,
    pub position: i32,
    pub length: i32,
    pub sample: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Block {
    pub samples: Vec<i16>,
    pub events: Vec<Ev>,
}

/// C's `outbuf_size / 2` for a buffer of `ms` milliseconds (at least 60).
pub fn buffer_samples(ms: i32, rate: u32) -> usize {
    let ms = ms.max(60) as u64;
    let milli = ms * u64::from(rate);
    ((milli + 1000 - milli % 1000) / 500 / 2) as usize
}


/// 1-based character position just after each clause-ending punctuation mark.
pub fn clause_end_positions(text: &str) -> Vec<i32> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .iter()
        .enumerate()
        .filter(|&(i, &c)| {
            matches!(c, '.' | ',' | ';' | ':' | '!' | '?')
                && chars.get(i + 1).is_none_or(|n| n.is_whitespace())
        })
        .map(|(i, _)| (i as i32 + 2).min(chars.len() as i32))
        .collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

fn byte_offset(text: &str, pos: u32) -> usize {
    let skip = (pos as usize).saturating_sub(1);
    text.char_indices().nth(skip).map_or(text.len(), |(b, _)| b)
}

/// The characters of the word at 1-based `pos`: what a plain-text word
/// event reports until the engine gives its length.
fn word_length(text: &str, pos: u32) -> i32 {
    text.get(byte_offset(text, pos)..)
        .map_or(0, |s| s.chars().take_while(|&c| is_word_char(c)).count() as i32)
}

fn text_events(text: &str, events: &[Source]) -> Vec<Ev> {
    events
        .iter()
        .filter_map(|e| {
            let (kind, length) = match &e.kind {
                EventKind::Word(i) => {
                    let length = if e.length > 0 { e.length as i32 } else { word_length(text, e.text_position) };
                    (EvKind::Word(*i as i32), length)
                }
                EventKind::Mark(name) => (EvKind::Mark(name.clone()), e.length as i32),
                EventKind::Sentence(n) => (EvKind::Sentence(*n as i32), e.length as i32),
                _ => return None,
            };
            Some(Ev { kind, position: e.text_position as i32, length, sample: e.sample as i32 })
        })
        .collect()
}

fn rank(kind: &EvKind) -> u8 {
    match kind {
        EvKind::SampleRate(_) => 0,
        EvKind::End(_) => 1,
        EvKind::Sentence(_) => 2,
        EvKind::Mark(_) => 3,
        EvKind::Word(_) => 4,
    }
}

fn split_chunk(chunk: &[i16], size: usize) -> Vec<Block> {
    if chunk.is_empty() {
        return vec![Block::default()];
    }
    chunk
        .chunks(size)
        .map(|c| Block { samples: c.to_vec(), events: Vec::new() })
        .collect()
}

fn attach(blocks: &mut [Block], ev: Ev, start: i32, size: usize, to_last: bool) {
    let last = blocks.len().saturating_sub(1);
    let ix = if to_last { last } else { (((ev.sample - start).max(0)) as usize / size).min(last) };
    blocks[ix].events.push(ev);
}

/// Cut `chunks` (one per clause) into buffers of `size` samples and attach
/// the events.  The first buffer carries the sample-rate event.
pub fn build(text: &str, chunks: &[Vec<i16>], events: &[SynthEvent], rate: u32, size: usize) -> Vec<Block> {
    let sources: Vec<Source> = events.iter().map(Source::from).collect();
    build_from(text, chunks, &sources, rate, size)
}

fn sentences_in(events: &[Ev]) -> i32 {
    events.iter().filter_map(|e| match e.kind {
        EvKind::Sentence(n) => Some(n),
        _ => None,
    }).max().unwrap_or(0)
}

fn build_from(text: &str, chunks: &[Vec<i16>], events: &[Source], rate: u32, size: usize) -> Vec<Block> {
    let size = size.max(1);
    let mut evs = text_events(text, events);
    evs.sort_by_key(|e| (e.sample, rank(&e.kind)));
    let ends = clause_end_positions(text);
    let text_len = text.chars().count() as i32;
    let mut out: Vec<Block> = Vec::new();
    let mut start = 0i32;
    let mut sentences = 0;
    for (k, chunk) in chunks.iter().enumerate() {
        let end = start + chunk.len() as i32;
        let mut blocks = split_chunk(chunk, size);
        let last_chunk = k + 1 == chunks.len();
        let (mine, rest): (Vec<Ev>, Vec<Ev>) =
            evs.into_iter().partition(|e| last_chunk || e.sample < end);
        evs = rest;
        sentences = sentences.max(sentences_in(&mine));
        mine.into_iter().for_each(|e| attach(&mut blocks, e, start, size, false));
        let position = ends.get(k).copied().unwrap_or(text_len);
        let end_ev = Ev { kind: EvKind::End(sentences), position, length: 0, sample: end };
        attach(&mut blocks, end_ev, start, size, true);
        blocks.iter_mut().for_each(|b| b.events.sort_by_key(|e| (e.sample, rank(&e.kind))));
        out.extend(blocks);
        start = end;
    }
    if let Some(first) = out.first_mut() {
        let sr = Ev { kind: EvKind::SampleRate(rate as i32), position: 0, length: 0, sample: 0 };
        first.events.insert(0, sr);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_is_60ms_at_22050() {
        assert_eq!(buffer_samples(0, 22050), 1324);
    }

    #[test]
    fn clause_ends_follow_terminators() {
        assert_eq!(clause_end_positions("Hello world, this is a test. Second one."), vec![13, 29, 40]);
    }

    #[test]
    fn short_text_end_position_is_clamped() {
        assert_eq!(clause_end_positions("Hi."), vec![3]);
    }

    fn src(kind: EventKind, text_position: u32, length: u32, sample: u64) -> Source {
        Source { kind, text_position, length, sample }
    }

    fn events(blocks: &[Block]) -> Vec<&Ev> {
        blocks.iter().flat_map(|b| b.events.iter()).collect()
    }

    // C `evflat` for `One. "Two" three.` (en, 0x1000), espeak-ng master.
    fn two_sentences() -> Vec<Block> {
        let text = "One. \"Two\" three.";
        let sources = [
            src(EventKind::Sentence(1), 1, 0, 0),
            src(EventKind::Word(1), 1, 3, 0),
            src(EventKind::Sentence(2), 7, 0, 16318),
            src(EventKind::Word(2), 7, 3, 16318),
            src(EventKind::Word(3), 12, 5, 24347),
        ];
        build_from(text, &[vec![0; 13894], vec![0; 37839 - 13894]], &sources, 22050, 1324)
    }

    #[test]
    fn events_keep_the_sample_and_length_of_the_engine() {
        let blocks = two_sentences();
        let all = events(&blocks);
        let word = all.iter().find(|e| e.kind == EvKind::Word(3)).expect("word 3");
        assert_eq!((word.position, word.length, word.sample), (12, 5, 24347));
    }

    #[test]
    fn a_word_the_engine_gave_no_length_is_measured_in_the_text() {
        let sources = [src(EventKind::Word(1), 1, 0, 0)];
        let blocks = build_from("Hi.", &[vec![0; 2000]], &sources, 22050, 1324);
        let word = events(&blocks).into_iter().find(|e| e.kind == EvKind::Word(1)).expect("word").length;
        assert_eq!(word, 2);
    }

    #[test]
    fn an_engine_length_wins_over_the_text() {
        let sources = [src(EventKind::Word(1), 1, 7, 0)];
        let blocks = build_from("Hi.", &[vec![0; 2000]], &sources, 22050, 1324);
        let word = events(&blocks).into_iter().find(|e| e.kind == EvKind::Word(1)).expect("word").length;
        assert_eq!(word, 7);
    }

    #[test]
    fn end_events_carry_the_sentence_count_so_far() {
        let blocks = two_sentences();
        let ends: Vec<(i32, i32, i32)> = events(&blocks)
            .iter()
            .filter_map(|e| match e.kind {
                EvKind::End(n) => Some((e.position, n, e.sample)),
                _ => None,
            })
            .collect();
        assert_eq!(ends, vec![(5, 1, 13894), (17, 2, 37839)]);
    }

    #[test]
    fn a_sentence_keeps_the_position_the_engine_gave_it() {
        let sources = [
            src(EventKind::Sentence(1), 3, 0, 0),
            src(EventKind::Word(1), 9, 2, 0),
        ];
        let blocks = build_from("  a  Hi", &[vec![0; 2000]], &sources, 22050, 1324);
        let sentence = events(&blocks).into_iter().find(|e| e.kind == EvKind::Sentence(1)).expect("sentence").position;
        assert_eq!(sentence, 3);
    }
}
