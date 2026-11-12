//! C-compatible types and constants from `speak_lib.h` and `espeak_ng.h`.

use std::ffi::{c_char, c_int, c_uint, c_void};

#[cfg(windows)]
pub type WChar = u16;
#[cfg(not(windows))]
pub type WChar = i32;

pub const EE_OK: c_int = 0;
pub const EE_INTERNAL_ERROR: c_int = -1;
pub const EE_NOT_FOUND: c_int = 2;

pub const ENS_OK: c_int = 0;
pub const ENS_VOICE_NOT_FOUND: c_int = 0x100006FF;
pub const ENS_NOT_INITIALIZED: c_int = 0x100004FF;
pub const ENS_NOT_SUPPORTED: c_int = 0x10000AFF;
pub const ENS_AUDIO_ERROR: c_int = 0x100005FF;
pub const ENS_UNKNOWN_TEXT_ENCODING: c_int = 0x100010FF;

pub const AUDIO_OUTPUT_RETRIEVAL: c_int = 1;
pub const AUDIO_OUTPUT_SYNCHRONOUS: c_int = 2;

pub const ENOUTPUT_MODE_SYNCHRONOUS: c_int = 1;
pub const ENOUTPUT_MODE_SPEAK_AUDIO: c_int = 2;

pub const CHARS_UTF8: c_uint = 1;
pub const CHARS_8BIT: c_uint = 2;
pub const CHARS_WCHAR: c_uint = 3;
pub const CHARS_16BIT: c_uint = 4;
pub const SSML: c_uint = 0x10;
pub const ENDPAUSE: c_uint = 0x1000;

pub const POS_CHARACTER_VALUE: i32 = 1;

pub const EVENT_LIST_TERMINATED: c_int = 0;
pub const EVENT_WORD: c_int = 1;
pub const EVENT_SENTENCE: c_int = 2;
pub const EVENT_MARK: c_int = 3;
pub const EVENT_END: c_int = 5;
pub const EVENT_MSG_TERMINATED: c_int = 6;
pub const EVENT_SAMPLERATE: c_int = 8;

pub const N_SPEECH_PARAM: usize = 15;
pub const PARAM_DEFAULTS: [i32; N_SPEECH_PARAM] = [0, 175, 100, 50, 50, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0];

#[repr(C)]
#[derive(Clone, Copy)]
pub union EventId {
    pub number: c_int,
    pub name: *const c_char,
    pub string: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EspeakEvent {
    pub type_: c_int,
    pub unique_identifier: c_uint,
    pub text_position: c_int,
    pub length: c_int,
    pub audio_position: c_int,
    pub sample: c_int,
    pub user_data: *mut c_void,
    pub id: EventId,
}

#[repr(C)]
pub struct EspeakVoice {
    pub name: *const c_char,
    pub languages: *const c_char,
    pub identifier: *const c_char,
    pub gender: u8,
    pub age: u8,
    pub variant: u8,
    pub xx1: u8,
    pub score: c_int,
    pub spare: *mut c_void,
}

pub type SynthCallback = unsafe extern "C" fn(*mut i16, c_int, *mut EspeakEvent) -> c_int;
pub type UriCallback = unsafe extern "C" fn(c_int, *const c_char, *const c_char) -> c_int;
pub type PhonemeCallback = unsafe extern "C" fn(*const c_char) -> c_int;

/// Opaque `FILE` stream; only passed through, never dereferenced.
#[repr(C)]
pub struct CFile {
    _private: [u8; 0],
}
