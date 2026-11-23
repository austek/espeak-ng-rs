// tests/phoneme_list_carryover.rs
//
// C keeps `phoneme_list` (whose closing-pause amp and pitches `MakePhonemeList` never
// writes) and the wave generator in statics, so their state carries from one
// `espeak_Synth` call to the next.  Expected samples and FNV-1a hashes come
// from the reference C library (f0fb11a8) through the probe scenario
// `multi <voice> 0 <text>...` in capi/tests/c/probe.c; `@name` switches voice.

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind, Parameter};

fn fnv(pcm: &[i16]) -> u64 {
    let bytes = pcm.iter().flat_map(|s| s.to_le_bytes());
    bytes.fold(1_469_598_103_934_665_603, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

fn engine(voice: &str) -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        return None;
    }
    let mut e = EspeakNg::builder().voice(voice).data_dir(dir).build().ok()?;
    e.set_no_final_pause(true);
    Some(e)
}

/// `(text, samples, fnv)` per call; a text starting with `@` switches voice, `+P=N` sets the
/// pitch parameter.
fn check(voice: &str, calls: &[(&str, usize, u64)]) {
    let Some(mut e) = engine(voice) else { return };
    for (text, samples, hash) in calls {
        if let Some(next) = text.strip_prefix('@') {
            e.set_voice(next);
            continue;
        }
        if let Some(pitch) = text.strip_prefix("+P=") {
            e.set_parameter(Parameter::Pitch, pitch.parse().expect("pitch"));
            continue;
        }
        let (pcm, _) = e.synth(text).expect("synth");
        assert_eq!((pcm.len(), fnv(&pcm)), (*samples, *hash), "{voice}: {text:?}");
    }
}

#[test]
fn hungarian_calls_carry_the_closing_pause() {
    check(
        "hu",
        &[
            ("ggy", 2248, 0x9241_82db_5d6d_fe43),
            ("ggy", 2265, 0x95df_707b_504a_bf3b),
            ("jó napot kívánok", 25598, 0x52df_304e_b3fd_5ff6),
            ("ggy", 2251, 0x31da_a811_7496_f714),
        ],
    );
}

#[test]
fn english_calls_carry_the_closing_pause() {
    check("en", &[("hello", 9315, 0x8c83_25df_ed14_1320), ("hello", 9334, 0xd4eb_a10a_631b_1f50)]);
}

#[test]
fn slots_survive_a_voice_change() {
    check(
        "en",
        &[
            ("hello", 9315, 0x8c83_25df_ed14_1320),
            ("@hu", 0, 0),
            ("ggy", 2263, 0x6710_2937_fdb6_0373),
            ("@en", 0, 0),
            ("hello", 9329, 0xcf16_6eae_8482_263b),
            ("hello", 9320, 0x339d_e7d4_a2b1_105c),
        ],
    );
}

#[test]
fn the_echo_buffer_carries_into_the_next_call() {
    check("en+announcer", &[("hello", 10592, 0x8a06_7f65_7e63_b3e6), ("hello", 10592, 0x6c6c_4003_8998_1cba)]);
}

#[test]
fn a_pitch_change_between_calls_leaves_the_formants_alone() {
    check(
        "en",
        &[
            ("hello", 9315, 0x8c83_25df_ed14_1320),
            ("+P=85", 0, 0),
            ("hello", 9392, 0x3b53_c27e_89c5_9c0a),
            ("hello", 9392, 0x6834_c8b3_09c6_5a10),
        ],
    );
}

/// C reports the rate when the wave generator picks up a voice, so only the first call and the
/// first one after a voice selection carry the event.
#[test]
fn the_sample_rate_event_follows_a_voice_selection_only() {
    let Some(mut e) = engine("en") else { return };
    let rates = |e: &EspeakNg| {
        let (_, _, events) = e.synth_with_events("hello").expect("synth");
        events.iter().filter(|ev| matches!(ev.kind, EventKind::SampleRate(_))).count()
    };
    assert_eq!([rates(&e), rates(&e)], [1, 0]);
    e.set_voice("en");
    assert_eq!([rates(&e), rates(&e)], [1, 0]);
}

#[test]
fn slots_survive_an_empty_call() {
    check("en", &[("hello", 9315, 0x8c83_25df_ed14_1320), ("", 154, 0x161e_ef09_8334_cb73), ("hello", 9334, 0xd4eb_a10a_631b_1f50)]);
}
