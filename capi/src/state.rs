//! Process-global library state, as in C.  The lock is held only to read or
//! change plain data; user callbacks run with it released.

use std::ffi::{c_int, CString};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::phonemes::Clause;
use crate::settings::Settings;
use crate::types::{PhonemeCallback, SynthCallback, UriCallback, ENOUTPUT_MODE_SYNCHRONOUS};
use crate::voicetab::VoiceTable;

/// Remaining clauses of the text a `TextToPhonemes` loop is walking.
pub struct PhonemeRun {
    pub base: usize,
    pub raw: Vec<u8>,
    pub mode: i32,
    pub clauses: Vec<Clause>,
}

pub struct State {
    pub settings: Settings,
    pub output_mode: c_int,
    pub buflength_ms: i32,
    pub next_uid: u32,
    pub text_out: CString,
    pub info_path: CString,
    pub listing: Option<VoiceTable>,
    pub current: Option<VoiceTable>,
    pub run: Option<PhonemeRun>,
    pub phoneme_trace: i32,
}

#[derive(Clone, Copy, Default)]
pub struct Callbacks {
    pub synth: Option<SynthCallback>,
    pub uri: Option<UriCallback>,
    pub phoneme: Option<PhonemeCallback>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static CALLBACKS: Mutex<Callbacks> = Mutex::new(Callbacks { synth: None, uri: None, phoneme: None });
static PENDING_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
pub static CANCEL: AtomicBool = AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl State {
    pub fn new(settings: Settings) -> Self {
        let info_path = CString::new(settings.data_dir.to_string_lossy().replace('\0', "")).unwrap_or_default();
        State {
            info_path,
            settings,
            output_mode: ENOUTPUT_MODE_SYNCHRONOUS,
            buflength_ms: 0,
            next_uid: 1,
            text_out: CString::default(),
            listing: None,
            current: None,
            run: None,
            phoneme_trace: 0,
        }
    }

    pub fn synchronous(&self) -> bool {
        self.output_mode & ENOUTPUT_MODE_SYNCHRONOUS != 0
    }
}

/// Run `f` on the initialized state; `None` when the library is not initialized.
pub fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    lock(&STATE).as_mut().map(f)
}

pub fn install(state: State) {
    *lock(&STATE) = Some(state);
}

pub fn clear() -> bool {
    lock(&STATE).take().is_some()
}

pub fn callbacks() -> Callbacks {
    *lock(&CALLBACKS)
}

pub fn update_callbacks(f: impl FnOnce(&mut Callbacks)) {
    f(&mut lock(&CALLBACKS));
}

pub fn set_pending_path(path: Option<PathBuf>) {
    *lock(&PENDING_PATH) = path;
}

pub fn pending_path() -> Option<PathBuf> {
    lock(&PENDING_PATH).clone()
}
