//! Every read of a caller-supplied pointer lives here.
//!
//! Each function takes a possibly-null pointer and returns `None` for null.
//! The caller of the exported C function guarantees that a non-null pointer
//! refers to a NUL-terminated buffer of the element type the C header names;
//! that is the same contract the C library relies on.

use std::ffi::{c_char, c_void, CStr};

use espeak_ng::{DecodeMode, Encoding, TextDecoder};

use crate::types::{WChar, CHARS_16BIT, CHARS_8BIT, CHARS_UTF8, CHARS_WCHAR};

/// # Safety
/// `p` is null or points to a NUL-terminated C string.
pub unsafe fn c_string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    Some(CStr::from_ptr(p).to_string_lossy().into_owned())
}

/// # Safety
/// `p` is null or points to a NUL-terminated array of `T`.
unsafe fn terminated<T: Copy + PartialEq + Default>(p: *const T) -> Option<Vec<T>> {
    if p.is_null() {
        return None;
    }
    let zero = T::default();
    let mut out = Vec::new();
    let mut cur = p;
    while *cur != zero {
        out.push(*cur);
        cur = cur.add(1);
    }
    Some(out)
}

fn decode_bytes(bytes: &[u8], encoding: Encoding, mode: DecodeMode) -> String {
    TextDecoder::new(bytes, encoding, mode)
        .map(|mut d| d.decode_to_string())
        .unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
}

/// Read the text argument of `espeak_Synth` and friends, decoding it as the
/// `espeakCHARS_*` part of `flags` says.
///
/// # Safety
/// `p` is null or points to a NUL-terminated buffer of the flagged width.
pub unsafe fn text_arg(p: *const c_void, flags: u32) -> Option<String> {
    match flags & 0xf {
        CHARS_WCHAR => terminated(p.cast::<WChar>()).map(|w| wide_to_string(&w)),
        CHARS_16BIT => terminated(p.cast::<u16>()).map(|u| String::from_utf16_lossy(&u)),
        CHARS_8BIT => terminated(p.cast::<u8>())
            .map(|b| decode_bytes(&b, Encoding::Iso8859_1, DecodeMode::Strict)),
        CHARS_UTF8 => terminated(p.cast::<u8>()).map(|b| decode_bytes(&b, Encoding::Utf8, DecodeMode::Strict)),
        _ => terminated(p.cast::<u8>()).map(|b| decode_bytes(&b, Encoding::Iso8859_1, DecodeMode::Auto)),
    }
}

fn wide_to_string(units: &[WChar]) -> String {
    #[cfg(windows)]
    {
        String::from_utf16_lossy(units)
    }
    #[cfg(not(windows))]
    {
        units.iter().filter_map(|&u| char::from_u32(u as u32)).collect()
    }
}

/// # Safety
/// `p` is null or points to a NUL-terminated `wchar_t` array.
pub unsafe fn wide_list(p: *const WChar) -> Option<Vec<char>> {
    terminated(p).map(|w| wide_to_string(&w).chars().collect())
}

/// The bytes of the NUL-terminated buffer at `p`, without the terminator.
///
/// # Safety
/// `p` is null or points to a NUL-terminated byte buffer.
pub unsafe fn raw_bytes(p: *const c_void) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    CStr::from_ptr(p.cast()).to_bytes().to_vec()
}

/// The selection criteria of an `espeak_VOICE` passed by the caller.
pub struct VoiceCriteria {
    pub name: Option<String>,
    pub language: Option<String>,
    pub variant: usize,
}

/// # Safety
/// `spec` is null or points to a readable `espeak_VOICE` whose string fields
/// are null or NUL-terminated.
pub unsafe fn voice_criteria(spec: *const crate::types::EspeakVoice) -> Option<VoiceCriteria> {
    let s = spec.as_ref()?;
    Some(VoiceCriteria {
        name: c_string(s.name),
        language: c_string(s.languages),
        variant: usize::from(s.variant),
    })
}

/// # Safety
/// `p` is null or points to a writable `T`.
pub unsafe fn write_out<T>(p: *mut T, value: T) {
    if !p.is_null() {
        p.write(value);
    }
}

/// Read the pointer a `const void **` points at.
///
/// # Safety
/// `pp` is null or points to a readable `*const c_void`.
pub unsafe fn read_ptr(pp: *const *const c_void) -> Option<*const c_void> {
    if pp.is_null() {
        None
    } else {
        Some(pp.read())
    }
}

/// # Safety
/// `pp` is null or points to a writable `*const c_void`.
pub unsafe fn write_ptr(pp: *mut *const c_void, value: *const c_void) {
    write_out(pp, value);
}
