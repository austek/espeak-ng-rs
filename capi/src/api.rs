//! The exported C symbols.  Each body runs inside `guard`, so a panic becomes
//! an error code instead of unwinding into C.

use std::ffi::{c_char, c_int, c_long, c_uint, c_void, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;

use crate::ops::{self, Job};
use crate::delivery::{deliver, Target};
use crate::phonemes;
use crate::ptr::*;
use crate::settings::Settings;
use crate::state::{self, with_state, PhonemeRun, State, CANCEL};
use crate::types::*;
use crate::voicetab::VoiceTable;

fn guard<T>(default: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(default)
}

fn status_to_ee(status: c_int) -> c_int {
    match status {
        ENS_OK => EE_OK,
        ENS_VOICE_NOT_FOUND => EE_NOT_FOUND,
        _ => EE_INTERNAL_ERROR,
    }
}

fn bool_status(ok: bool, failure: c_int) -> c_int {
    if ok {
        ENS_OK
    } else {
        failure
    }
}

fn error_status(e: &espeak_ng::Error) -> c_int {
    match e {
        espeak_ng::Error::VoiceNotFound(_) => ENS_VOICE_NOT_FOUND,
        _ => ENS_NOT_SUPPORTED,
    }
}

fn start(path: Option<PathBuf>, mode: c_int, buflength: c_int) -> c_int {
    let Some(dir) = ops::resolve_data_dir(path) else {
        return ENS_NOT_INITIALIZED;
    };
    let mut state = State::new(Settings::new(dir));
    state.output_mode = mode;
    state.buflength_ms = buflength;
    state::install(state);
    ENS_OK
}

// ---------------------------------------------------------------- init

/// # Safety
/// `path` is null or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn espeak_Initialize(output: c_int, buflength: c_int, path: *const c_char, _options: c_int) -> c_int {
    guard(EE_INTERNAL_ERROR, || {
        let mode = match output {
            AUDIO_OUTPUT_RETRIEVAL => 0,
            AUDIO_OUTPUT_SYNCHRONOUS => ENOUTPUT_MODE_SYNCHRONOUS,
            _ => return EE_INTERNAL_ERROR,
        };
        let path = c_string(path).map(PathBuf::from);
        match start(path, mode, buflength) {
            ENS_OK => ops::SAMPLE_RATE,
            _ => EE_INTERNAL_ERROR,
        }
    })
}

/// # Safety
/// `path` is null or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_InitializePath(path: *const c_char) {
    guard((), || state::set_pending_path(c_string(path).map(PathBuf::from)));
}

/// # Safety
/// `context` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_Initialize(context: *mut *mut c_void) -> c_int {
    guard(ENS_NOT_INITIALIZED, || {
        write_out(context, ptr::null_mut());
        start(state::pending_path(), ENOUTPUT_MODE_SYNCHRONOUS, 0)
    })
}

/// # Safety
/// `_device` is unused.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_InitializeOutput(mode: c_int, buflength: c_int, _device: *const c_char) -> c_int {
    guard(ENS_NOT_INITIALIZED, || {
        if mode & ENOUTPUT_MODE_SPEAK_AUDIO != 0 {
            return ENS_AUDIO_ERROR;
        }
        let set = with_state(|s| {
            s.output_mode = mode;
            s.buflength_ms = buflength;
        });
        bool_status(set.is_some(), ENS_NOT_INITIALIZED)
    })
}

#[no_mangle]
pub extern "C" fn espeak_ng_GetSampleRate() -> c_int {
    ops::SAMPLE_RATE
}

#[no_mangle]
pub extern "C" fn espeak_Terminate() -> c_int {
    guard(EE_INTERNAL_ERROR, || {
        state::clear();
        EE_OK
    })
}

#[no_mangle]
pub extern "C" fn espeak_ng_Terminate() -> c_int {
    guard(ENS_NOT_INITIALIZED, || {
        state::clear();
        ENS_OK
    })
}

// ----------------------------------------------------------- callbacks

#[no_mangle]
pub extern "C" fn espeak_SetSynthCallback(cb: Option<SynthCallback>) {
    guard((), || state::update_callbacks(|c| c.synth = cb));
}

#[no_mangle]
pub extern "C" fn espeak_SetUriCallback(cb: Option<UriCallback>) {
    guard((), || state::update_callbacks(|c| c.uri = cb));
}

#[no_mangle]
pub extern "C" fn espeak_SetPhonemeCallback(cb: Option<PhonemeCallback>) {
    guard((), || state::update_callbacks(|c| c.phoneme = cb));
}

// ----------------------------------------------------------- synthesis

struct Request {
    text: String,
    flags: c_uint,
    position: c_uint,
    position_type: c_int,
}

fn synthesize(req: Request, uid_out: *mut c_uint, user_data: *mut c_void) -> c_int {
    let snapshot = with_state(|s| {
        let retrieval = !s.synchronous();
        let uid = if retrieval { s.next_uid } else { 0 };
        s.next_uid += u32::from(retrieval);
        (s.settings.clone(), s.buflength_ms, uid, retrieval)
    });
    let Some((settings, buflength_ms, uid, retrieval)) = snapshot else {
        return ENS_NOT_INITIALIZED;
    };
    // SAFETY: `uid_out` is null or the caller's writable unsigned int.
    unsafe { write_out(uid_out, uid) };
    CANCEL.store(false, Ordering::SeqCst);
    let job = Job {
        settings,
        text: req.text,
        flags: req.flags,
        position: req.position,
        position_type: req.position_type,
        buflength_ms,
    };
    let rendered = match ops::render(&job) {
        Ok(r) => r,
        Err(e) => return error_status(&e),
    };
    if let Some(callback) = state::callbacks().synth {
        let target = Target { callback, uid, user_data, rate: rendered.rate, retrieval };
        deliver(&target, &rendered.blocks);
    }
    ENS_OK
}

unsafe fn synth_text(
    text: *const c_void,
    position: c_uint,
    position_type: c_int,
    flags: c_uint,
    uid_out: *mut c_uint,
    user_data: *mut c_void,
) -> c_int {
    guard(ENS_NOT_SUPPORTED, || match text_arg(text, flags) {
        Some(text) => synthesize(Request { text, flags, position, position_type }, uid_out, user_data),
        None => ENS_UNKNOWN_TEXT_ENCODING,
    })
}

/// # Safety
/// `text` is NUL-terminated in the width `flags` names; `unique_identifier`
/// is null or writable.
#[no_mangle]
pub unsafe extern "C" fn espeak_Synth(
    text: *const c_void,
    _size: usize,
    position: c_uint,
    position_type: c_int,
    _end_position: c_uint,
    flags: c_uint,
    unique_identifier: *mut c_uint,
    user_data: *mut c_void,
) -> c_int {
    status_to_ee(synth_text(text, position, position_type, flags, unique_identifier, user_data))
}

/// # Safety
/// As `espeak_Synth`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_Synthesize(
    text: *const c_void,
    _size: usize,
    position: c_uint,
    position_type: c_int,
    _end_position: c_uint,
    flags: c_uint,
    unique_identifier: *mut c_uint,
    user_data: *mut c_void,
) -> c_int {
    synth_text(text, position, position_type, flags, unique_identifier, user_data)
}

unsafe fn synth_mark(
    text: *const c_void,
    index_mark: *const c_char,
    flags: c_uint,
    uid_out: *mut c_uint,
    user_data: *mut c_void,
) -> c_int {
    guard(ENS_NOT_SUPPORTED, || {
        let Some(full) = text_arg(text, flags) else {
            return ENS_UNKNOWN_TEXT_ENCODING;
        };
        let tail = c_string(index_mark)
            .and_then(|m| full.find(&format!("name=\"{m}\"")).and_then(|i| full[i..].find('>').map(|j| i + j + 1)))
            .map_or(full.clone(), |at| full[at..].to_string());
        let req = Request { text: tail, flags: flags | SSML, position: 0, position_type: POS_CHARACTER_VALUE };
        synthesize(req, uid_out, user_data)
    })
}

/// # Safety
/// As `espeak_Synth`; `index_mark` is null or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn espeak_Synth_Mark(
    text: *const c_void,
    _size: usize,
    index_mark: *const c_char,
    _end_position: c_uint,
    flags: c_uint,
    unique_identifier: *mut c_uint,
    user_data: *mut c_void,
) -> c_int {
    status_to_ee(synth_mark(text, index_mark, flags, unique_identifier, user_data))
}

/// # Safety
/// As `espeak_Synth_Mark`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_SynthesizeMark(
    text: *const c_void,
    _size: usize,
    index_mark: *const c_char,
    _end_position: c_uint,
    flags: c_uint,
    unique_identifier: *mut c_uint,
    user_data: *mut c_void,
) -> c_int {
    synth_mark(text, index_mark, flags, unique_identifier, user_data)
}

fn speak_character(code: u32) -> c_int {
    let req = Request { text: ops::character_markup(code), flags: SSML, position: 0, position_type: POS_CHARACTER_VALUE };
    synthesize(req, ptr::null_mut(), ptr::null_mut())
}

unsafe fn speak_key(key: *const c_char) -> c_int {
    guard(ENS_NOT_SUPPORTED, || match c_string(key) {
        None => ENS_UNKNOWN_TEXT_ENCODING,
        Some(k) => {
            let mut chars = k.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => speak_character(c as u32),
                _ => synthesize(
                    Request { text: k, flags: 0, position: 0, position_type: POS_CHARACTER_VALUE },
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
            }
        }
    })
}

/// # Safety
/// `key_name` is null or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn espeak_Key(key_name: *const c_char) -> c_int {
    status_to_ee(speak_key(key_name))
}

/// # Safety
/// `key_name` is null or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_SpeakKeyName(key_name: *const c_char) -> c_int {
    speak_key(key_name)
}

#[no_mangle]
pub extern "C" fn espeak_Char(character: WChar) -> c_int {
    status_to_ee(guard(ENS_NOT_SUPPORTED, || speak_character(character as u32)))
}

#[no_mangle]
pub extern "C" fn espeak_ng_SpeakCharacter(character: WChar) -> c_int {
    guard(ENS_NOT_SUPPORTED, || speak_character(character as u32))
}

#[no_mangle]
pub extern "C" fn espeak_Cancel() -> c_int {
    guard(EE_INTERNAL_ERROR, || {
        CANCEL.store(true, Ordering::SeqCst);
        EE_OK
    })
}

#[no_mangle]
pub extern "C" fn espeak_ng_Cancel() -> c_int {
    guard(ENS_NOT_INITIALIZED, || {
        CANCEL.store(true, Ordering::SeqCst);
        ENS_OK
    })
}

#[no_mangle]
pub extern "C" fn espeak_IsPlaying() -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn espeak_Synchronize() -> c_int {
    EE_OK
}

#[no_mangle]
pub extern "C" fn espeak_ng_Synchronize() -> c_int {
    ENS_OK
}

// ---------------------------------------------------------- parameters

fn param_index(parameter: c_int) -> Option<usize> {
    usize::try_from(parameter).ok().filter(|&i| i < N_SPEECH_PARAM)
}

fn set_parameter(parameter: c_int, value: c_int, relative: c_int) -> c_int {
    let Some(ix) = param_index(parameter) else {
        return 22;
    };
    match with_state(|s| s.settings.set_param(ix, value, relative != 0)) {
        Some(true) => ENS_OK,
        Some(false) => 22,
        None => ENS_NOT_INITIALIZED,
    }
}

#[no_mangle]
pub extern "C" fn espeak_SetParameter(parameter: c_int, value: c_int, relative: c_int) -> c_int {
    status_to_ee(guard(22, || set_parameter(parameter, value, relative)))
}

#[no_mangle]
pub extern "C" fn espeak_ng_SetParameter(parameter: c_int, value: c_int, relative: c_int) -> c_int {
    guard(22, || set_parameter(parameter, value, relative))
}

#[no_mangle]
pub extern "C" fn espeak_GetParameter(parameter: c_int, current: c_int) -> c_int {
    guard(0, || {
        let Some(ix) = param_index(parameter) else {
            return 0;
        };
        let live = (current != 0).then(|| with_state(|s| s.settings.params[ix])).flatten();
        live.unwrap_or(PARAM_DEFAULTS[ix])
    })
}

unsafe fn set_punctuation(list: *const WChar) -> c_int {
    guard(ENS_NOT_INITIALIZED, || {
        let chars = wide_list(list).unwrap_or_default();
        bool_status(with_state(|s| s.settings.punct_list = chars).is_some(), ENS_NOT_INITIALIZED)
    })
}

/// # Safety
/// `punctlist` is null or a NUL-terminated `wchar_t` array.
#[no_mangle]
pub unsafe extern "C" fn espeak_SetPunctuationList(punctlist: *const WChar) -> c_int {
    status_to_ee(set_punctuation(punctlist))
}

/// # Safety
/// As `espeak_SetPunctuationList`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_SetPunctuationList(punctlist: *const WChar) -> c_int {
    set_punctuation(punctlist)
}

/// # Safety
/// `_stream` is not used.
#[no_mangle]
pub unsafe extern "C" fn espeak_SetPhonemeTrace(phonememode: c_int, _stream: *mut CFile) {
    guard((), || {
        with_state(|s| s.phoneme_trace = phonememode);
    });
}

#[no_mangle]
pub extern "C" fn espeak_ng_SetPhonemeEvents(_enable: c_int, _ipa: c_int) -> c_int {
    ENS_OK
}

// -------------------------------------------------------------- voices

fn voice_status(ok: Option<bool>) -> c_int {
    match ok {
        Some(true) => ENS_OK,
        Some(false) => ENS_VOICE_NOT_FOUND,
        None => ENS_NOT_INITIALIZED,
    }
}

unsafe fn set_voice_name(name: *const c_char) -> c_int {
    guard(ENS_NOT_INITIALIZED, || match c_string(name) {
        Some(n) => voice_status(with_state(|s| ops::select_voice(&mut s.settings, &n))),
        None => ENS_VOICE_NOT_FOUND,
    })
}

unsafe fn set_voice_file(name: *const c_char) -> c_int {
    guard(ENS_NOT_INITIALIZED, || match c_string(name) {
        Some(n) => voice_status(with_state(|s| ops::select_voice_file(&mut s.settings, &n))),
        None => ENS_VOICE_NOT_FOUND,
    })
}

unsafe fn set_voice_props(spec: *mut EspeakVoice) -> c_int {
    guard(ENS_NOT_INITIALIZED, || match voice_criteria(spec) {
        Some(c) => voice_status(with_state(|s| {
            ops::select_by_properties(&mut s.settings, c.name.as_deref(), c.language.as_deref(), c.variant)
        })),
        None => ENS_VOICE_NOT_FOUND,
    })
}

/// # Safety
/// `name` is null or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn espeak_SetVoiceByName(name: *const c_char) -> c_int {
    status_to_ee(set_voice_name(name))
}

/// # Safety
/// As `espeak_SetVoiceByName`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_SetVoiceByName(name: *const c_char) -> c_int {
    set_voice_name(name)
}

/// # Safety
/// `filename` is null or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn espeak_SetVoiceByFile(filename: *const c_char) -> c_int {
    status_to_ee(set_voice_file(filename))
}

/// # Safety
/// As `espeak_SetVoiceByFile`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_SetVoiceByFile(filename: *const c_char) -> c_int {
    set_voice_file(filename)
}

/// # Safety
/// `voice_spec` is null or a readable `espeak_VOICE`.
#[no_mangle]
pub unsafe extern "C" fn espeak_SetVoiceByProperties(voice_spec: *mut EspeakVoice) -> c_int {
    status_to_ee(set_voice_props(voice_spec))
}

/// # Safety
/// As `espeak_SetVoiceByProperties`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_SetVoiceByProperties(voice_spec: *mut EspeakVoice) -> c_int {
    set_voice_props(voice_spec)
}

/// # Safety
/// `voice_spec` is null or a readable `espeak_VOICE`.  The returned table is
/// valid until the next call to this function or `espeak_Terminate`.
#[no_mangle]
pub unsafe extern "C" fn espeak_ListVoices(voice_spec: *mut EspeakVoice) -> *const *const EspeakVoice {
    guard(ptr::null(), || {
        let language = voice_criteria(voice_spec).and_then(|c| c.language);
        with_state(|s| {
            let entries = ops::list(&s.settings, language.as_deref());
            let table = VoiceTable::from_entries(&entries);
            let p = table.list_ptr();
            s.listing = Some(table);
            p
        })
        .unwrap_or(ptr::null())
    })
}

/// The returned voice is valid until the next call or `espeak_Terminate`.
#[no_mangle]
pub extern "C" fn espeak_GetCurrentVoice() -> *mut EspeakVoice {
    guard(ptr::null_mut(), || {
        with_state(|s| {
            let entry = espeak_ng::voices::find_catalog_voice(&s.settings.voice, &s.settings.data_dir);
            let table = VoiceTable::from_entries(&entry.into_iter().collect::<Vec<_>>());
            let p = table.first();
            s.current = Some(table);
            p
        })
        .unwrap_or(ptr::null_mut())
    })
}

// ------------------------------------------------------------ phonemes

fn same_run(run: &PhonemeRun, base: usize, raw: &[u8], mode: c_int) -> bool {
    base >= run.base
        && run.mode == mode
        && run.raw.get(base - run.base..).is_some_and(|rest| rest == raw)
}

fn clause_for(state: &mut State, base: usize, raw: Vec<u8>, text: &dyn Fn() -> String, mode: c_int) -> Option<(phonemes::Clause, usize)> {
    let reuse = state.run.as_ref().is_some_and(|r| same_run(r, base, &raw, mode));
    if !reuse {
        let clauses = phonemes::clauses(&state.settings, &text(), mode).ok()?;
        state.run = Some(PhonemeRun { base, raw, mode, clauses });
    }
    let run = state.run.as_ref()?;
    let offset = base - run.base;
    let clause = run.clauses.iter().find(|c| c.end_byte > offset)?.clone();
    let next = if clause.end_byte >= run.raw.len() { 0 } else { run.base + clause.end_byte };
    Some((clause, next))
}

unsafe fn text_to_phonemes(
    textptr: *const *const c_void,
    textmode: c_int,
    phonememode: c_int,
    terminator: *mut c_int,
) -> *const c_char {
    guard(ptr::null(), || {
        let Some(p) = read_ptr(textptr).filter(|p| !p.is_null()) else {
            return ptr::null();
        };
        let raw = raw_bytes(p);
        let decode = || text_arg(p, textmode as c_uint).unwrap_or_default();
        let found = with_state(|s| {
            let (clause, next) = clause_for(s, p as usize, raw, &decode, phonememode)?;
            s.text_out = CString::new(clause.phonemes.replace('\0', "")).unwrap_or_default();
            Some((s.text_out.as_ptr(), next, clause.terminator))
        })
        .flatten();
        let Some((out, next, term)) = found else {
            write_ptr(textptr.cast_mut(), ptr::null());
            return ptr::null();
        };
        write_ptr(textptr.cast_mut(), next as *const c_void);
        write_out(terminator, term);
        out
    })
}

/// # Safety
/// `textptr` points to a pointer to a NUL-terminated text; the result is valid
/// until the next call.
#[no_mangle]
pub unsafe extern "C" fn espeak_TextToPhonemes(textptr: *const *const c_void, textmode: c_int, phonememode: c_int) -> *const c_char {
    text_to_phonemes(textptr, textmode, phonememode, ptr::null_mut())
}

/// # Safety
/// As `espeak_TextToPhonemes`; `terminator` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn espeak_TextToPhonemesWithTerminator(
    textptr: *const *const c_void,
    textmode: c_int,
    phonememode: c_int,
    terminator: *mut c_int,
) -> *const c_char {
    text_to_phonemes(textptr, textmode, phonememode, terminator)
}

// --------------------------------------------------------- dictionaries

unsafe fn compile(dsource: *const c_char, dict_name: *const c_char) -> c_int {
    guard(ENS_NOT_INITIALIZED, || {
        let (source, name) = (c_string(dsource), c_string(dict_name));
        let done = with_state(|s| ops::compile_dict(&s.settings, source.as_deref(), name.as_deref()));
        match done {
            Some(Ok(())) => ENS_OK,
            Some(Err(_)) => 0x100001FF,
            None => ENS_NOT_INITIALIZED,
        }
    })
}

/// # Safety
/// `path` is null or NUL-terminated; `_log` is not used.
#[no_mangle]
pub unsafe extern "C" fn espeak_CompileDictionary(path: *const c_char, _log: *mut CFile, _flags: c_int) {
    guard((), || {
        compile(path, ptr::null());
    });
}

/// # Safety
/// Strings are null or NUL-terminated; `_log` and `context` are not used.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_CompileDictionary(
    dsource: *const c_char,
    dict_name: *const c_char,
    _log: *mut CFile,
    _flags: c_int,
    _context: *mut *mut c_void,
) -> c_int {
    compile(dsource, dict_name)
}

macro_rules! unsupported {
    ($($name:ident($($arg:ident: $ty:ty),*);)*) => {$(
        /// # Safety
        /// Arguments are not read.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            $(let _ = $arg;)*
            ENS_NOT_SUPPORTED
        }
    )*};
}

unsupported! {
    espeak_ng_CompileMbrolaVoice(path: *const c_char, log: *mut CFile, context: *mut *mut c_void);
    espeak_ng_CompilePhonemeData(rate: c_long, log: *mut CFile, context: *mut *mut c_void);
    espeak_ng_CompileIntonation(log: *mut CFile, context: *mut *mut c_void);
    espeak_ng_CompileIntonationPath(src: *const c_char, dst: *const c_char, log: *mut CFile, context: *mut *mut c_void);
    espeak_ng_CompilePhonemeDataPath(rate: c_long, src: *const c_char, dst: *const c_char, log: *mut CFile, context: *mut *mut c_void);
    espeak_ng_SetOutputHooks(hooks: *mut c_void);
    espeak_ng_SetConstF0(f0: c_int);
    espeak_ng_SetRandSeed(seed: c_long);
}

// ---------------------------------------------------------------- info

fn version_cstring() -> &'static CString {
    static V: OnceLock<CString> = OnceLock::new();
    V.get_or_init(|| CString::new(ops::VERSION).unwrap_or_default())
}

/// # Safety
/// `path_data` is null or writable.  The returned strings are valid until
/// `espeak_Terminate`.
#[no_mangle]
pub unsafe extern "C" fn espeak_Info(path_data: *mut *const c_char) -> *const c_char {
    guard(ptr::null(), || {
        let path = with_state(|s| s.info_path.as_ptr()).unwrap_or(ptr::null());
        write_out(path_data, path);
        version_cstring().as_ptr()
    })
}

/// # Safety
/// `context` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_ClearErrorContext(context: *mut *mut c_void) {
    write_out(context, ptr::null_mut());
}

fn status_message(status: c_int) -> String {
    let fixed = match status {
        0x100001FF => "Compile error",
        0x100002FF => "Wrong version of espeak-ng-data",
        0x100003FF => "The FIFO buffer is full",
        ENS_NOT_INITIALIZED => "The espeak-ng library has not been initialized",
        ENS_AUDIO_ERROR => "Cannot initialize the audio device",
        ENS_VOICE_NOT_FOUND => "The specified espeak-ng voice does not exist",
        0x100007FF => "Could not load the mbrola.dll file",
        0x100008FF => "Could not load the specified mbrola voice file",
        0x100009FF => "The event buffer is full",
        ENS_NOT_SUPPORTED => "The requested functionality has not been built into espeak-ng",
        0x10000BFF => "The phoneme file is not in a supported format",
        0x10000CFF => "The spectral file does not contain any frame data",
        0x10000DFF => "The phoneme manifest file does not contain any phonemes",
        0x10000FFF => "The phoneme feature is not recognised",
        ENS_UNKNOWN_TEXT_ENCODING => "The text encoding is not supported",
        _ => "",
    };
    match (fixed, status) {
        ("", 0..=255) => {
            let text = std::io::Error::from_raw_os_error(status).to_string();
            text.split(" (os error").next().unwrap_or_default().to_string()
        }
        ("", _) => format!("Unspecified error 0x{status:x}"),
        _ => fixed.to_string(),
    }
}

extern "C" {
    fn fputs(s: *const c_char, stream: *mut CFile) -> c_int;
}

/// # Safety
/// `out` is null or an open C `FILE`; `_context` is not read.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_PrintStatusCodeMessage(status: c_int, out: *mut CFile, _context: *mut c_void) {
    guard((), || {
        if out.is_null() {
            return;
        }
        let line = CString::new(format!("Error: {}.\n", status_message(status))).unwrap_or_default();
        // SAFETY: `out` is a live `FILE*` supplied by the caller and `line`
        // is NUL-terminated.
        unsafe { fputs(line.as_ptr(), out) };
    });
}

/// # Safety
/// `buffer` is null or writable for `length` bytes.
#[no_mangle]
pub unsafe extern "C" fn espeak_ng_GetStatusCodeMessage(status: c_int, buffer: *mut c_char, length: usize) {
    guard((), || {
        if buffer.is_null() || length == 0 {
            return;
        }
        let text = status_message(status);
        let msg = text.as_bytes();
        let n = msg.len().min(length - 1);
        // SAFETY: `buffer` holds `length` bytes and `n + 1 <= length`.
        unsafe {
            ptr::copy_nonoverlapping(msg.as_ptr().cast::<c_char>(), buffer, n);
            buffer.add(n).write(0);
        }
    });
}
