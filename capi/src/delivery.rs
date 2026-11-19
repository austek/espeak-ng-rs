//! Hands rendered blocks to the user's synth callback.  The callback calls
//! are the only `unsafe` here.

use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;

use espeak_ng::engine::EventKind;

use crate::blocks::{Block, Ev};
use crate::state::CANCEL;
use crate::types::*;

pub struct Target {
    pub callback: SynthCallback,
    pub uid: u32,
    pub user_data: *mut c_void,
    pub retrieval: bool,
}

fn event(t: &Target, ty: c_int, ev: &Ev, id: EventId) -> EspeakEvent {
    EspeakEvent {
        type_: ty,
        unique_identifier: t.uid,
        text_position: ev.position,
        length: ev.length,
        audio_position: ev.audio_ms,
        sample: ev.sample,
        user_data: t.user_data,
        id,
    }
}

fn number(n: u32) -> EventId {
    EventId { number: n as c_int }
}

/// C copies at most 8 bytes of a phoneme name and does not terminate a full one.
fn phoneme_id(name: &str) -> EventId {
    let mut string = [0 as c_char; 8];
    name.bytes().take(8).enumerate().for_each(|(i, b)| string[i] = b as c_char);
    EventId { string }
}

fn name_id(name: &str, names: &mut Vec<CString>) -> EventId {
    names.push(CString::new(name.replace('\0', "")).unwrap_or_default());
    EventId { name: names.last().map_or(ptr::null(), |c| c.as_ptr()) }
}

/// C events for one block; `names` keeps mark and play names alive for the call.
fn c_events(t: &Target, evs: &[Ev], names: &mut Vec<CString>) -> Vec<EspeakEvent> {
    let mut out: Vec<EspeakEvent> = evs
        .iter()
        .filter_map(|ev| {
            Some(match &ev.kind {
                EventKind::SampleRate(r) => event(t, EVENT_SAMPLERATE, ev, number(*r)),
                EventKind::Sentence(n) => event(t, EVENT_SENTENCE, ev, number(*n)),
                EventKind::Word(n) => event(t, EVENT_WORD, ev, number(*n)),
                EventKind::End(n) => event(t, EVENT_END, ev, number(*n)),
                EventKind::Mark(name) => event(t, EVENT_MARK, ev, name_id(name, names)),
                EventKind::Play(uri) => event(t, EVENT_PLAY, ev, name_id(uri, names)),
                EventKind::Phoneme(name) => event(t, EVENT_PHONEME, ev, phoneme_id(name)),
                _ => return None,
            })
        })
        .collect();
    out.push(event(t, EVENT_LIST_TERMINATED, &Ev::zero(), number(0)));
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
    let zero = Ev::zero();
    let mut list = vec![
        event(t, EVENT_MSG_TERMINATED, &zero, number(0)),
        event(t, EVENT_LIST_TERMINATED, &zero, number(0)),
    ];
    call(t, None, &mut list)
}

/// What C does with a clause's phoneme string before it generates the clause.
pub struct Trace {
    pub lines: Vec<String>,
    /// Print each line to `stream` (`espeak_SetPhonemeTrace`).
    pub print: bool,
    /// The trace `FILE*` as an address; 0 is NULL, which C maps to stderr.
    pub stream: usize,
    pub callback: Option<PhonemeCallback>,
}

extern "C" {
    fn fputs(s: *const c_char, stream: *mut CFile) -> c_int;
    fn fdopen(fd: c_int, mode: *const c_char) -> *mut CFile;
}

fn stderr_stream() -> *mut CFile {
    static STDERR: OnceLock<usize> = OnceLock::new();
    // SAFETY: fd 2 and a constant mode string.
    *STDERR.get_or_init(|| unsafe { fdopen(2, c"w".as_ptr()) } as usize) as *mut CFile
}

fn emit(trace: &Trace, line: &str) {
    let Ok(text) = CString::new(line.replace('\0', "")) else { return };
    if trace.print {
        let stream = if trace.stream == 0 { stderr_stream() } else { trace.stream as *mut CFile };
        let out = CString::new(format!("{line}\n").replace('\0', "")).unwrap_or_default();
        // SAFETY: `stream` is the caller's open `FILE*` (or stderr) and `out` is NUL-terminated.
        unsafe { fputs(out.as_ptr(), stream) };
    }
    if let Some(callback) = trace.callback {
        // SAFETY: the registered callback accepts a NUL-terminated string valid for the call.
        unsafe { callback(text.as_ptr()) };
    }
}

/// The trace lines due before clause `k` of `n`; the last clause also takes any surplus.
fn lines_before(trace: &Trace, k: usize, n: usize) -> &[String] {
    let from = k.min(trace.lines.len());
    let to = if k + 1 == n { trace.lines.len() } else { (k + 1).min(trace.lines.len()) };
    &trace.lines[from..to]
}

/// The trace of a request that has no synth callback.
pub fn trace_only(trace: &Trace) {
    trace.lines.iter().for_each(|line| emit(trace, line));
}

/// Deliver every clause's blocks, then the end-of-data call.  Returns true
/// when the callback or `espeak_Cancel` stopped the synthesis early.
pub fn deliver(t: &Target, clauses: &[Vec<Block>], trace: &Trace) -> bool {
    for (k, blocks) in clauses.iter().enumerate() {
        lines_before(trace, k, clauses.len()).iter().for_each(|line| emit(trace, line));
        for block in blocks {
            if CANCEL.load(Ordering::SeqCst) || deliver_block(t, block) {
                return true;
            }
        }
    }
    let mut end = c_events(t, &[], &mut Vec::new());
    let stopped = call(t, None, &mut end);
    if t.retrieval {
        terminated(t);
    }
    stopped
}
