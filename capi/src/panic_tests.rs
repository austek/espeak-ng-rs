//! A panic inside an exported function must not unwind into the C caller, and the library must
//! work again afterwards.  `state::fault::visit` (compiled in tests only) panics at the `n`th of
//! the fault points an `espeak_Synth` call passes: the entry to `with_state` (state lock held,
//! so the lock is poisoned), after the process globals were restored, after the rendering, and
//! before each audio block is delivered to the callback, so later points panic with part of the
//! audio already delivered.

use std::ffi::{c_int, c_uint, c_void, CString};
use std::sync::Mutex;

use crate::api::*;
use crate::state::fault;
use crate::types::*;

/// The library state is process-global, so the tests take turns.
static TURN: Mutex<()> = Mutex::new(());
static SAMPLES: Mutex<Vec<i16>> = Mutex::new(Vec::new());

const TEXT: &[u8] = b"Hello world. How are you today? Fine, thanks.\0";

unsafe extern "C" fn collect(wav: *mut i16, count: c_int, _events: *mut EspeakEvent) -> c_int {
    if !wav.is_null() {
        let chunk = std::slice::from_raw_parts(wav, usize::try_from(count).unwrap_or(0));
        SAMPLES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend_from_slice(chunk);
    }
    0
}

fn data_parent() -> Option<CString> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    root.join("espeak-ng-data/en_dict").exists().then(|| CString::new(root.to_string_lossy().as_bytes()).unwrap())
}

fn initialize(parent: &CString) -> c_int {
    unsafe {
        let rate = espeak_Initialize(AUDIO_OUTPUT_SYNCHRONOUS, 0, parent.as_ptr(), 0);
        espeak_SetSynthCallback(Some(collect));
        espeak_SetVoiceByName(c"en".as_ptr());
        rate
    }
}

/// The status of one `espeak_Synth` of `TEXT` and the samples it delivered.
fn synth() -> (c_int, Vec<i16>) {
    SAMPLES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear();
    let mut uid: c_uint = 0;
    let status = unsafe {
        espeak_Synth(TEXT.as_ptr().cast::<c_void>(), TEXT.len(), 0, 1, 0, CHARS_UTF8, &mut uid, std::ptr::null_mut())
    };
    let samples = SAMPLES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    (status, samples)
}

/// Wave state and pauses carry across synth calls, so a later call differs by a few samples;
/// a poisoned or lost state would give nothing or a different utterance.
fn same_length(got: &[i16], baseline: &[i16]) -> bool {
    got.len().abs_diff(baseline.len()) * 100 <= baseline.len()
}

#[test]
fn a_panic_at_any_fault_point_of_synth_leaves_the_library_usable() {
    let Some(parent) = data_parent() else {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    };
    let _turn = TURN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(initialize(&parent) > 0);
    let (status, baseline) = synth();
    assert_eq!(status, EE_OK);
    assert!(!baseline.is_empty());

    espeak_Terminate();
    assert!(initialize(&parent) > 0);
    fault::reset_visits();
    let _ = synth();
    let accesses = fault::visits();
    assert!(accesses > 0, "synth never reached the state");

    let mut delivered_before_panic = Vec::new();
    for n in 1..=accesses {
        espeak_Terminate();
        assert!(initialize(&parent) > 0);
        fault::arm(n);
        let (status, partial) = synth();
        fault::disarm();
        delivered_before_panic.push(partial.len());
        assert_eq!(status, EE_INTERNAL_ERROR, "panic at fault point {n} of {accesses} became an error code");
        let (again_status, again) = synth();
        assert_eq!(again_status, EE_OK, "synth after a panic at access {n}");
        assert!(same_length(&again, &baseline), "{} samples after a panic at access {n}, baseline {}", again.len(), baseline.len());
    }
    assert!(accesses > 4, "the sweep reaches the delivery loop: {accesses} fault points");
    assert!(
        delivered_before_panic.iter().any(|&n| n > 0 && n < baseline.len()),
        "some panic came mid-delivery: {delivered_before_panic:?}"
    );
    espeak_Terminate();
}

#[test]
fn a_panic_during_initialize_and_setup_calls_returns_the_error_default() {
    let Some(parent) = data_parent() else {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    };
    let _turn = TURN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(initialize(&parent) > 0);
    let (_, baseline) = synth();

    fault::arm(1);
    let status = unsafe { espeak_SetVoiceByName(c"de".as_ptr()) };
    fault::disarm();
    assert_eq!(status, EE_INTERNAL_ERROR);

    fault::arm(1);
    let status = espeak_SetParameter(1, 150, 0);
    fault::disarm();
    assert_eq!(status, EE_INTERNAL_ERROR);

    assert_eq!(unsafe { espeak_SetVoiceByName(c"en".as_ptr()) }, EE_OK);
    assert_eq!(espeak_SetParameter(1, 175, 0), EE_OK);
    let (status, again) = synth();
    assert_eq!(status, EE_OK);
    assert!(same_length(&again, &baseline), "{} samples after the panics, baseline {}", again.len(), baseline.len());
    espeak_Terminate();
}
