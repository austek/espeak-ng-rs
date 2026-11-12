//! Pure construction of the callback blocks C delivers: audio cut into
//! fixed-size buffers per clause, with each event attached to its buffer.

use espeak_ng::engine::{EventKind, SynthEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvKind {
    SampleRate(i32),
    Sentence(i32),
    Word(i32),
    Mark(String),
    End,
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

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

fn char_index(text: &str, byte: usize) -> usize {
    text.get(..byte.min(text.len())).map_or(0, |s| s.chars().count())
}

fn word_length(text: &str, byte: usize) -> i32 {
    text.get(byte..)
        .map_or(0, |s| s.chars().take_while(|&c| is_word_char(c)).count() as i32)
}

fn ms_to_sample(ms: u32, rate: u32) -> i32 {
    (u64::from(ms) * u64::from(rate)).div_ceil(1000) as i32
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

fn text_events(text: &str, events: &[SynthEvent], rate: u32) -> Vec<Ev> {
    let mut sentence = 0;
    let mut out: Vec<Ev> = events
        .iter()
        .filter_map(|e| {
            let sample = ms_to_sample(e.audio_position_ms, rate);
            let pos = char_index(text, e.text_position) as i32 + 1;
            match &e.kind {
                EventKind::Word(i) => Some(Ev {
                    kind: EvKind::Word(*i as i32 + 1),
                    position: pos,
                    length: word_length(text, e.text_position),
                    sample,
                }),
                EventKind::Mark(name) => Some(Ev {
                    kind: EvKind::Mark(name.clone()),
                    position: pos,
                    length: 0,
                    sample,
                }),
                EventKind::Sentence => {
                    sentence += 1;
                    Some(Ev { kind: EvKind::Sentence(sentence), position: 1, length: 0, sample })
                }
                _ => None,
            }
        })
        .collect();
    fix_sentence_positions(&mut out);
    out
}

fn fix_sentence_positions(events: &mut [Ev]) {
    for i in 0..events.len() {
        if matches!(events[i].kind, EvKind::Sentence(_)) {
            let next = events[i..].iter().find(|e| matches!(e.kind, EvKind::Word(_)));
            events[i].position = next.map_or(events[i].position, |w| w.position);
        }
    }
}

fn rank(kind: &EvKind) -> u8 {
    match kind {
        EvKind::SampleRate(_) => 0,
        EvKind::End => 1,
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
    let size = size.max(1);
    let mut evs = text_events(text, events, rate);
    evs.sort_by_key(|e| (e.sample, rank(&e.kind)));
    let ends = clause_end_positions(text);
    let text_len = text.chars().count() as i32;
    let mut out: Vec<Block> = Vec::new();
    let mut start = 0i32;
    for (k, chunk) in chunks.iter().enumerate() {
        let end = start + chunk.len() as i32;
        let mut blocks = split_chunk(chunk, size);
        let last_chunk = k + 1 == chunks.len();
        let end_ms = i64::from(end) * 1000 / i64::from(rate.max(1));
        let before_end = |e: &Ev| i64::from(e.sample) * 1000 / i64::from(rate.max(1)) < end_ms;
        let (mine, rest): (Vec<Ev>, Vec<Ev>) =
            evs.into_iter().partition(|e| last_chunk || before_end(e));
        evs = rest;
        mine.into_iter().for_each(|e| attach(&mut blocks, e, start, size, false));
        let position = ends.get(k).copied().unwrap_or(text_len);
        let end_ev = Ev { kind: EvKind::End, position, length: 0, sample: end };
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
}
