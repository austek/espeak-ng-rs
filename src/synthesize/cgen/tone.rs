//! `CalcPitches_Tone` from `intonation.c`: pitch from the tone phoneme of each
//! syllable, with the sandhi rules of Vietnamese, Mandarin and Hakka.

use super::langtab::ToneLang;
use super::phlist::{Ph, ToneEnv, SFLAG_SYLLABLE};
use crate::phoneme::{PhonemeData, PhonemeTab, PH_PAUSE, PH_VOWEL, PHON_DEFAULTTONE};
use crate::synthesize::bytecode::{interpret_phoneme, Neighbours};
use crate::synthesize::envelopes::ENVELOPE_DATA;

const M_TONE_11: u32 = 0x3131;
const M_TONE_55: u32 = 0x3535;
const M_TONE_35: u32 = 0x3533;
const M_TONE_51: u32 = 0x3135;
const M_TONE_214: u32 = 0x343132;
/// Tone 5 and the levels the translator re-levels it to after another tone.
const NEUTRAL_TONES: [u32; 4] = [M_TONE_11, 0x3232, 0x3333, 0x3434];

struct Sandhi {
    pause: bool,
    promoted: bool,
    prev: u32,
    prevw: u32,
    prev_p: usize,
    final_stressed: usize,
}

fn tab(phdata: &PhonemeData, name: &str) -> Option<PhonemeTab> {
    match phdata.lookup_phoneme(name) {
        0 => None,
        code => phdata.get(code).cloned(),
    }
}

fn mnemonic(p: &Ph) -> u32 {
    p.tone.as_ref().map_or(0, |t| t.mnemonic)
}

fn final_stressed(list: &[Ph]) -> usize {
    list.iter().rposition(|p| p.typ == PH_VOWEL && p.stresslevel >= 4).unwrap_or(0)
}

fn hakka_sandhi(list: &mut [Ph], ix: usize, st: &Sandhi, phdata: &PhonemeData) {
    if st.prev == 0x31 && matches!(mnemonic(&list[ix]), 0x31 | 0x34 | 0x36) {
        list[st.prev_p].tone = tab(phdata, "2");
    }
}

fn mandarin_tone5(list: &mut [Ph], ix: usize, st: &Sandhi, phdata: &PhonemeData) {
    let level = match st.prevw {
        _ if mnemonic(&list[ix]) != M_TONE_11 => "",
        M_TONE_55 => "22",
        M_TONE_35 => "33",
        M_TONE_214 => "44",
        _ => "",
    };
    if !level.is_empty() {
        list[ix].tone = tab(phdata, level);
    }
    list[ix].stresslevel = 0;
}

fn mandarin_default_tone(list: &mut [Ph], ix: usize, st: &mut Sandhi, phdata: &PhonemeData) {
    if list[ix].tone.is_some() {
        st.promoted = false;
        return;
    }
    let promote = st.pause || st.promoted;
    list[ix].tone = tab(phdata, if promote { "55" } else { "11" });
    st.promoted = promote;
}

fn mandarin_sandhi(list: &mut [Ph], ix: usize, st: &mut Sandhi, phdata: &PhonemeData) -> u32 {
    mandarin_default_tone(list, ix, st, phdata);
    let m = mnemonic(&list[ix]);
    if ix == st.final_stressed && (m == M_TONE_55 || m == M_TONE_51) {
        list[st.final_stressed].stresslevel = 6;
    }
    if st.prevw == M_TONE_214 {
        list[st.prev_p].tone = tab(phdata, if m == M_TONE_214 { "35" } else { "21" });
    }
    if st.prev == M_TONE_51 && m == M_TONE_51 {
        list[st.prev_p].tone = tab(phdata, "53");
    }
    if NEUTRAL_TONES.contains(&m) {
        mandarin_tone5(list, ix, st, phdata);
    }
    m
}

fn sandhi_step(list: &mut [Ph], ix: usize, kind: ToneLang, st: &mut Sandhi, phdata: &PhonemeData) {
    if list[ix].typ == PH_PAUSE && list[ix].ph.std_length > 50 {
        st.pause = true;
        st.prevw = 0;
    }
    if list[ix].newword != 0 {
        st.prev = 0;
    }
    if list[ix].synthflags & SFLAG_SYLLABLE == 0 {
        return;
    }
    let tph = match kind {
        ToneLang::Hakka => {
            hakka_sandhi(list, ix, st, phdata);
            mnemonic(&list[ix])
        }
        ToneLang::Mandarin => mandarin_sandhi(list, ix, st, phdata),
        _ => mnemonic(&list[ix]),
    };
    st.prev_p = ix;
    st.prev = tph;
    st.prevw = tph;
    st.pause = false;
}

fn tone_envelope(phdata: &PhonemeData, tone: &PhonemeTab) -> ToneEnv {
    let e = interpret_phoneme(tone.program, &phdata.phonindex, &Neighbours::default(), |_| None);
    let first = match e.pitch_env {
        0 => ENVELOPE_DATA[0][0] as i32,
        off => phdata.phondata.get(off as usize).map_or(0, |&b| b as i32),
    };
    ToneEnv { pitch: e.pitch_env, amp: e.amp_env, first }
}

fn assign_pitch(list: &mut [Ph], phdata: &PhonemeData) {
    for p in list.iter_mut().filter(|p| p.synthflags & SFLAG_SYLLABLE != 0) {
        if p.tone.is_none() {
            p.tone = phdata.get(PHON_DEFAULTTONE).cloned();
        }
        let Some(t) = p.tone.clone() else { continue };
        p.pitch1 = t.start_type as i32;
        p.pitch2 = t.end_type as i32;
        p.tone_env = tone_envelope(phdata, &t);
    }
}

/// `TranslateWord2` leaves each non-pause phoneme with the stress of the last
/// vowel before it in its word, and gives the consonant right before a later
/// vowel that vowel's stress. `CalcPitches_Tone` keeps these; the intonation
/// pass would overwrite them.
fn carry_stress(list: &mut [Ph]) {
    let (mut stress, mut prev_vowel) = (0u8, None);
    for i in 0..list.len() {
        if list[i].newword != 0 {
            (stress, prev_vowel) = (0, None);
        }
        if list[i].typ == PH_PAUSE {
            continue;
        }
        if list[i].typ == PH_VOWEL {
            stress = list[i].stresslevel;
            if prev_vowel.is_some_and(|v| v + 1 != i) {
                list[i - 1].stresslevel = stress;
            }
            prev_vowel = Some(i);
        } else {
            list[i].stresslevel = stress;
        }
    }
}

/// C gives a neutral-tone syllable the word's stress and lowers it to 0 only
/// in the tone pass; the translator already leaves it unstressed, so the
/// last-stressed-syllable search would skip it.
fn restore_neutral_stress(list: &mut [Ph]) {
    for p in list.iter_mut().filter(|p| p.typ == PH_VOWEL && p.stresslevel < 4) {
        if NEUTRAL_TONES.contains(&mnemonic(p)) {
            p.stresslevel = 4;
        }
    }
}

/// Give every syllable of the clause its pitch from its tone phoneme.
pub fn calc_pitches_tone(list: &mut [Ph], kind: ToneLang, phdata: &PhonemeData) {
    if kind == ToneLang::Mandarin {
        restore_neutral_stress(list);
    }
    carry_stress(list);
    let fs = final_stressed(list);
    list[fs].stresslevel = 7;
    if kind == ToneLang::Vietnamese && list[fs].tone.is_none() {
        list[fs].tone = tab(phdata, "7");
    }
    let mut st = Sandhi { pause: true, promoted: false, prev: 0, prevw: 0, prev_p: 0, final_stressed: fs };
    for ix in 0..list.len() {
        sandhi_step(list, ix, kind, &mut st, phdata);
    }
    assign_pitch(list, phdata);
}
