//! Pure construction of the callback blocks C delivers: audio cut into
//! fixed-size buffers per clause, with each engine event attached to the
//! buffer it was generated in.

use espeak_ng::engine::{EventKind, SynthEvent};

/// The fields of a [`SynthEvent`] the callback needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ev {
    pub kind: EventKind,
    pub position: i32,
    pub length: i32,
    pub sample: i32,
    pub audio_ms: i32,
}

impl Ev {
    /// The all-zero event that terminates a list and `MsgTerminated` itself.
    pub fn zero() -> Self {
        Ev { kind: EventKind::MsgTerminated, position: 0, length: 0, sample: 0, audio_ms: 0 }
    }
}

impl From<&SynthEvent> for Ev {
    fn from(e: &SynthEvent) -> Self {
        Ev {
            kind: e.kind.clone(),
            position: e.text_position as i32,
            length: e.length as i32,
            sample: e.sample as i32,
            audio_ms: e.audio_position_ms as i32,
        }
    }
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

fn split_chunk(chunk: &[i16], size: usize) -> Vec<Block> {
    if chunk.is_empty() {
        return vec![Block::default()];
    }
    chunk
        .chunks(size)
        .map(|c| Block { samples: c.to_vec(), events: Vec::new() })
        .collect()
}

/// The clause an event belongs to: C closes a clause with its `End` event, so
/// every event before the n-th `End` (and the `End` itself) is the n-th
/// clause's.  An event past the end of its clause's audio moves on.
fn clause_of(ends_seen: usize, sample: i32, ends: &[i32]) -> usize {
    let last = ends.len().saturating_sub(1);
    let mut k = ends_seen.min(last);
    while k < last && sample > ends[k] {
        k += 1;
    }
    k
}

/// A voice change reports its `SampleRate` after the `End` of the clause it
/// follows, in that clause's last buffer.
fn open_clause(kind: &EventKind, ends_seen: usize) -> usize {
    match kind {
        EventKind::SampleRate(_) => ends_seen.saturating_sub(1),
        _ => ends_seen,
    }
}

/// Cut `chunks` (one per clause) into buffers of `size` samples and attach the
/// events in the order the engine produced them.  An event at an exact buffer
/// boundary opens the next buffer; the last buffer of a clause also takes the
/// events at its end.  `MsgTerminated` is not part of any block.  The result
/// has the buffers of each clause apart.
pub fn build(chunks: &[Vec<i16>], events: &[Ev], size: usize) -> Vec<Vec<Block>> {
    let size = size.max(1);
    let mut starts = Vec::with_capacity(chunks.len());
    let mut ends = Vec::with_capacity(chunks.len());
    let mut at = 0i32;
    for chunk in chunks {
        starts.push(at);
        at += chunk.len() as i32;
        ends.push(at);
    }
    let mut parts: Vec<Vec<Block>> = chunks.iter().map(|c| split_chunk(c, size)).collect();
    let mut ends_seen = 0usize;
    for ev in events.iter().filter(|e| e.kind != EventKind::MsgTerminated) {
        let k = clause_of(open_clause(&ev.kind, ends_seen), ev.sample, &ends);
        let Some(blocks) = parts.get_mut(k) else { continue };
        let last = blocks.len() - 1;
        let ix = ((ev.sample - starts[k]).max(0) as usize / size).min(last);
        blocks[ix].events.push(ev.clone());
        ends_seen += usize::from(matches!(ev.kind, EventKind::End(_)));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(chunks: &[Vec<i16>], events: &[Ev], size: usize) -> Vec<Block> {
        super::build(chunks, events, size).into_iter().flatten().collect()
    }

    #[test]
    fn buffer_is_60ms_at_22050() {
        assert_eq!(buffer_samples(0, 22050), 1324);
    }

    fn ev(kind: EventKind, position: i32, length: i32, sample: i32) -> Ev {
        Ev { kind, position, length, sample, audio_ms: 0 }
    }

    fn kinds(block: &Block) -> Vec<&EventKind> {
        block.events.iter().map(|e| &e.kind).collect()
    }

    // C `events 2 en 0x1010` for `One <mark name="a"/> two. Three, four.`: a
    // 304-sample buffer closes the first clause, whose End it carries.
    #[test]
    fn end_stays_in_the_last_buffer_of_its_clause_and_the_next_clause_opens_a_buffer() {
        let events = [
            ev(EventKind::Sentence(1), 1, 0, 0),
            ev(EventKind::End(1), 26, 0, 1628),
            ev(EventKind::Sentence(2), 27, 0, 1628),
        ];
        let blocks = build(&[vec![0; 1628], vec![0; 100]], &events, 1324);
        let found: Vec<usize> = blocks.iter().map(|b| b.events.len()).collect();
        assert_eq!(found, vec![1, 1, 1]);
        assert_eq!(kinds(&blocks[1]), vec![&EventKind::End(1)]);
        assert_eq!(kinds(&blocks[2]), vec![&EventKind::Sentence(2)]);
    }

    #[test]
    fn an_event_at_an_exact_buffer_boundary_opens_the_next_buffer() {
        let events = [ev(EventKind::Word(1), 1, 2, 1324)];
        let blocks = build(&[vec![0; 3000]], &events, 1324);
        assert_eq!(blocks[0].events.len(), 0);
        assert_eq!(blocks[1].events.len(), 1);
    }

    // C delivers a zero-sample call for the empty clause after an audio icon,
    // carrying that clause's End.
    #[test]
    fn an_empty_clause_keeps_its_own_end() {
        let events = [
            ev(EventKind::Play("a.wav".into()), 22, 0, 2000),
            ev(EventKind::End(0), 21, 0, 2000),
            ev(EventKind::End(0), 34, 0, 2000),
            ev(EventKind::Word(2), 37, 3, 2000),
        ];
        let blocks = build(&[vec![0; 2000], vec![], vec![0; 500]], &events, 1324);
        assert_eq!(blocks.iter().map(|b| b.events.len()).collect::<Vec<_>>(), vec![0, 2, 1, 1]);
        assert_eq!(kinds(&blocks[2]), vec![&EventKind::End(0)]);
        assert_eq!(kinds(&blocks[3]), vec![&EventKind::Word(2)]);
    }

    #[test]
    fn a_voice_change_reports_its_sample_rate_after_the_end_of_the_previous_clause() {
        let events = [
            ev(EventKind::End(1), 5, 0, 1000),
            ev(EventKind::SampleRate(16000), 0, 0, 1000),
            ev(EventKind::Word(2), 7, 3, 1000),
        ];
        let blocks = build(&[vec![0; 1000], vec![0; 500]], &events, 1324);
        assert_eq!(kinds(&blocks[0]), vec![&EventKind::End(1), &EventKind::SampleRate(16000)]);
        assert_eq!(kinds(&blocks[1]), vec![&EventKind::Word(2)]);
    }

    #[test]
    fn msg_terminated_is_not_part_of_a_block() {
        let blocks = build(&[vec![0; 10]], &[ev(EventKind::MsgTerminated, 0, 0, 0)], 1324);
        assert!(blocks[0].events.is_empty());
    }

    #[test]
    fn the_engine_order_survives_inside_a_block() {
        let events = [
            ev(EventKind::SampleRate(22050), 0, 0, 0),
            ev(EventKind::Sentence(1), 1, 0, 0),
            ev(EventKind::Word(1), 1, 3, 0),
            ev(EventKind::Phoneme("h".into()), 1, 0, 264),
        ];
        let blocks = build(&[vec![0; 3000]], &events, 1324);
        assert_eq!(blocks[0].events.len(), 4);
    }
}
