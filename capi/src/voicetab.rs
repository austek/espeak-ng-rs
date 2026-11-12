//! Owned `espeak_VOICE` tables handed out by `espeak_ListVoices` and
//! `espeak_GetCurrentVoice`.

use std::ffi::{c_void, CString};
use std::ptr;

use espeak_ng::voices::VoiceEntry;

use crate::types::EspeakVoice;

/// Owns the strings and structs the raw pointers in `ptrs` refer to.
pub struct VoiceTable {
    _strings: Vec<CString>,
    _languages: Vec<Vec<u8>>,
    voices: Vec<EspeakVoice>,
    ptrs: Vec<*const EspeakVoice>,
}

// SAFETY: the raw pointers point into heap allocations owned by this value
// and are only dereferenced by C callers that hold the library's contract
// (valid until the next listing call).
unsafe impl Send for VoiceTable {}

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

fn language_list(v: &VoiceEntry) -> Vec<u8> {
    let mut out = Vec::new();
    for (code, prio) in &v.languages {
        out.push(*prio);
        out.extend(code.bytes().filter(|&b| b != 0));
        out.push(0);
    }
    out.push(0);
    out
}

fn gender_code(g: char) -> u8 {
    match g {
        'M' => 1,
        'F' => 2,
        _ => 0,
    }
}

impl VoiceTable {
    pub fn from_entries(entries: &[VoiceEntry]) -> Self {
        let mut strings = Vec::new();
        let mut languages = Vec::new();
        let mut voices = Vec::new();
        for v in entries {
            let name = cstring(&v.name);
            let ident = cstring(&v.file);
            let langs = language_list(v);
            voices.push(EspeakVoice {
                name: name.as_ptr(),
                languages: langs.as_ptr().cast(),
                identifier: ident.as_ptr(),
                gender: gender_code(v.gender),
                age: v.age,
                variant: 0,
                xx1: 0,
                score: 0,
                spare: ptr::null_mut::<c_void>(),
            });
            strings.push(name);
            strings.push(ident);
            languages.push(langs);
        }
        let mut ptrs: Vec<*const EspeakVoice> = voices.iter().map(|v| v as *const _).collect();
        ptrs.push(ptr::null());
        VoiceTable { _strings: strings, _languages: languages, voices, ptrs }
    }

    pub fn list_ptr(&self) -> *const *const EspeakVoice {
        self.ptrs.as_ptr()
    }

    pub fn first(&self) -> *mut EspeakVoice {
        self.voices.first().map_or(ptr::null_mut(), |v| v as *const _ as *mut _)
    }
}
