//! Hands rendered blocks to the user's synth callback.  The callback calls
//! are the only `unsafe` here.

use std::ffi::{c_int, c_void, CString};
use std::ptr;
use std::sync::atomic::Ordering;

use crate::blocks::{Block, Ev, EvKind};
use crate::state::CANCEL;
use crate::types::*;

pub struct Target {
    pub callback: SynthCallback,
    pub uid: u32,
    pub user_data: *mut c_void,
    pub rate: u32,
    pub retrieval: bool,
}

fn event(t: &Target, ty: c_int, ev: &Ev, id: EventId) -> EspeakEvent {
    EspeakEvent {
        type_: ty,
        unique_identifier: t.uid,
        text_position: ev.position,
        length: ev.length,
        audio_position: (f64::from(ev.sample) * 1000.0 / f64::from(t.rate)) as i32,
        sample: ev.sample,
        user_data: t.user_data,
        id,
    }
}

/// C events for one block; `names` keeps mark names alive for the call.
fn c_events(t: &Target, evs: &[Ev], names: &mut Vec<CString>) -> Vec<EspeakEvent> {
    let mut out: Vec<EspeakEvent> = evs
        .iter()
        .map(|ev| match &ev.kind {
            EvKind::SampleRate(r) => event(t, EVENT_SAMPLERATE, ev, EventId { number: *r }),
            EvKind::Sentence(n) => event(t, EVENT_SENTENCE, ev, EventId { number: *n }),
            EvKind::Word(n) => event(t, EVENT_WORD, ev, EventId { number: *n }),
            EvKind::End => event(t, EVENT_END, ev, EventId { number: 0 }),
            EvKind::Mark(name) => {
                names.push(CString::new(name.replace('\0', "")).unwrap_or_default());
                let p = names.last().map_or(ptr::null(), |c| c.as_ptr());
                event(t, EVENT_MARK, ev, EventId { name: p })
            }
        })
        .collect();
    let end = Ev { kind: EvKind::End, position: 0, length: 0, sample: 0 };
    out.push(event(t, EVENT_LIST_TERMINATED, &end, EventId { number: 0 }));
    out
}

fn call(t: &Target, samples: Option<&mut [i16]>, events: &mut [EspeakEvent]) -> bool {
    let (p, n) = match samples {
        Some(s) => (s.as_mut_ptr(), s.len() as i32),
        None => (ptr::null_mut(), 0),
    };
    // SAFETY: the callback was registered by the C caller, who guarantees it
    // accepts a sample buffer of `n` entries (or NULL) and a terminated event
    // list that stays valid for the duration of the call.
    unsafe { (t.callback)(p, n, events.as_mut_ptr()) != 0 }
}

fn deliver_block(t: &Target, block: &Block) -> bool {
    let mut names = Vec::new();
    let mut events = c_events(t, &block.events, &mut names);
    let mut samples = block.samples.clone();
    call(t, Some(&mut samples), &mut events)
}

fn terminated(t: &Target) -> bool {
    let msg = Ev { kind: EvKind::End, position: 0, length: 0, sample: 0 };
    let mut list = vec![
        event(t, EVENT_MSG_TERMINATED, &msg, EventId { number: 0 }),
        event(t, EVENT_LIST_TERMINATED, &msg, EventId { number: 0 }),
    ];
    call(t, None, &mut list)
}

/// Deliver every block, then the end-of-data call.  Returns true when the
/// callback or `espeak_Cancel` stopped the synthesis early.
pub fn deliver(t: &Target, blocks: &[Block]) -> bool {
    for block in blocks {
        if CANCEL.load(Ordering::SeqCst) || deliver_block(t, block) {
            return true;
        }
    }
    let mut end = c_events(t, &[], &mut Vec::new());
    let stopped = call(t, None, &mut end);
    if t.retrieval {
        terminated(t);
    }
    stopped
}
