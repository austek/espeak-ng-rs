//! `CalcLengths` from `setlengths.c`.

use super::phlist::{Ph, PHLIST_END_OF_CLAUSE, SFLAG_LENGTHEN, SFLAG_SEQCONTINUE, SFLAG_SYLLABLE};
use super::speed::Speed;
use super::voice::LangSynth;
use crate::phoneme::{
    PHON_PAUSE_CLAUSE, PHON_PAUSE_VSHORT, PH_BRKAFTER, PH_FRICATIVE, PH_LENGTHENSTOP, PH_LIQUID,
    PH_NASAL, PH_NONSYLLABIC, PH_NOPAUSE, PH_PAUSE, PH_SIBILANT, PH_STOP, PH_VFRICATIVE,
    PH_VOICELESS, PH_VOWEL, PH_VSTOP,
};
use crate::synthesize::envelopes::ENVELOPE_DATA;

const PITCH_FALL: usize = 0;
const PITCH_RISE: usize = 2;
const S_EO_CLAUSE1: u32 = 0x40000;
const S_NO_EOC_LENGTHEN: u32 = 0x200000;

struct State {
    more_syllables: i32,
    pre_sonorant: bool,
    pre_voiced: bool,
    last_pitch: i32,
}

fn at(list: &[Ph], i: usize) -> &Ph {
    &list[i.min(list.len() - 1)]
}

/// Set `length`, `prepause`, `amp` and the consonants' pitch for a clause.
/// `speed_at` gives the speed in force at each phoneme (embedded `S`).
pub fn calc_lengths(list: &mut [Ph], tr: &LangSynth, speed_at: &dyn Fn(usize) -> Speed) {
    let mut st = State { more_syllables: 0, pre_sonorant: false, pre_voiced: false, last_pitch: 0 };
    for ix in 1..list.len() {
        let sp = speed_at(ix);
        let typ = if list[ix].synthflags & SFLAG_SYLLABLE != 0 { PH_VOWEL } else { list[ix].typ };
        match typ {
            PH_PAUSE => st.last_pitch = 0,
            PH_STOP => stop(list, ix, tr, &mut st),
            PH_VFRICATIVE | PH_FRICATIVE => fricative(list, ix, tr, typ, &mut st),
            PH_VSTOP => vstop(list, ix, tr, &mut st),
            PH_LIQUID | PH_NASAL => sonorant(list, ix, tr, &sp, &mut st),
            PH_VOWEL => vowel(list, ix, tr, &sp, &mut st),
            _ => {}
        }
    }
}

fn stop(list: &mut [Ph], ix: usize, tr: &LangSynth, st: &mut State) {
    st.last_pitch = 0;
    let stress = (list[ix].stresslevel & 7) as i32;
    let prev_type = list[ix - 1].typ;
    let p = &mut list[ix];
    p.prepause = if prev_type == PH_FRICATIVE {
        25
    } else if st.more_syllables > 0 || stress < 4 {
        48
    } else {
        60
    };
    if prev_type == PH_STOP {
        p.prepause = 60;
    }
    if tr.word_gap & 0x10 != 0 && p.newword != 0 {
        p.prepause = 60;
    }
    if p.ph.phflags & PH_LENGTHENSTOP != 0 {
        p.prepause += 30;
    }
    if p.synthflags & SFLAG_LENGTHEN != 0 {
        p.prepause += tr.long_stop;
    }
}

fn fricative(list: &mut [Ph], ix: usize, tr: &LangSynth, typ: u8, st: &mut State) {
    let prev = list[ix - 1].clone();
    let next = at(list, ix + 1).clone();
    let p = &mut list[ix];
    if p.newword != 0 && !(prev.typ == PH_VOWEL && p.ph.phflags & PH_NOPAUSE != 0) {
        p.prepause = 15;
    }
    if next.typ == PH_PAUSE && prev.typ == PH_NASAL && p.ph.phflags & PH_VOICELESS == 0 {
        p.prepause = 25;
    }
    if prev.ph.phflags & PH_BRKAFTER != 0 {
        p.prepause = 30;
    }
    if tr.word_gap & 0x10 != 0 && p.newword != 0 {
        p.prepause = 30;
    }
    p.length = if p.ph.phflags & PH_SIBILANT != 0 && next.typ == PH_STOP && next.newword == 0 {
        if prev.typ == PH_VOWEL { 200 } else { 150 }
    } else {
        256
    };
    if typ == PH_VFRICATIVE {
        if next.typ == PH_VOWEL {
            st.pre_voiced = true;
        }
        if prev.typ == PH_VOWEL || prev.typ == PH_LIQUID {
            p.length = (255 + prev.length) / 2;
        }
    }
}

fn vstop(list: &mut [Ph], ix: usize, tr: &LangSynth, st: &mut State) {
    let prev = list[ix - 1].clone();
    let next = at(list, ix + 1).clone();
    let p = &mut list[ix];
    if prev.typ == PH_VFRICATIVE || prev.typ == PH_FRICATIVE || prev.ph.phflags & PH_SIBILANT != 0 || prev.typ == PH_LIQUID {
        p.prepause = 30;
    }
    if next.typ == PH_VOWEL || next.typ == PH_LIQUID {
        if next.typ == PH_VOWEL || next.newword == 0 {
            st.pre_voiced = true;
        }
        p.prepause = 40;
        if prev.typ == PH_VOWEL {
            p.prepause = 0;
        } else if prev.typ == PH_PAUSE {
            p.prepause = if prev.length < p.prepause { p.prepause - prev.length } else { 0 };
        } else if p.newword == 0 {
            if prev.typ == PH_LIQUID {
                p.prepause = 20;
            }
            if prev.typ == PH_NASAL {
                p.prepause = 12;
            }
            if prev.typ == PH_STOP && prev.ph.phflags & PH_VOICELESS == 0 {
                p.prepause = 0;
            }
        }
    }
    if tr.word_gap & 0x10 != 0 && p.newword != 0 && p.prepause < 20 {
        p.prepause = 20;
    }
}

fn sonorant(list: &mut [Ph], ix: usize, tr: &LangSynth, sp: &Speed, st: &mut State) {
    let prev = list[ix - 1].clone();
    let next = at(list, ix + 1).clone();
    let later_pitch2 = list[ix..].iter().find(|q| q.typ == PH_VOWEL).map(|q| q.pitch2);
    let p = &mut list[ix];
    p.amp = tr.stress_amps[0];
    p.length = 256;
    if p.newword != 0 {
        if prev.typ == PH_LIQUID {
            p.prepause = 25;
        }
        if prev.typ == PH_VOWEL && p.ph.phflags & PH_NOPAUSE == 0 {
            p.prepause = 12;
        }
    }
    if next.typ == PH_VOWEL {
        st.pre_sonorant = true;
        return;
    }
    p.pitch2 = st.last_pitch;
    if prev.typ == PH_VOWEL || prev.typ == PH_LIQUID {
        p.length = prev.length;
        if p.typ == PH_LIQUID {
            p.length = sp.len_speeds[0];
        }
        if next.typ == PH_VSTOP {
            p.length = p.length * 160 / 100;
        }
        if next.typ == PH_VFRICATIVE {
            p.length = p.length * 120 / 100;
        }
    } else if let Some(p2) = later_pitch2 {
        p.pitch2 = p2;
    }
    p.pitch1 = if p.pitch2 < 16 { 0 } else { p.pitch2 - 16 };
    p.env = PITCH_FALL;
    st.pre_voiced = false;
}

fn word_end_info(list: &[Ph], ix: usize) -> (i32, bool) {
    let mut more = 0;
    let mut eoc = false;
    let mut j = ix + 1;
    while j < list.len() && list[j].newword == 0 {
        if list[j].typ == PH_VOWEL && list[j].ph.phflags & PH_NONSYLLABIC == 0 {
            more += 1;
        }
        if list[j].code() == PHON_PAUSE_CLAUSE {
            eoc = true;
        }
        j += 1;
    }
    let p2 = at(list, j);
    if p2.code() == PHON_PAUSE_CLAUSE || (p2.newword & PHLIST_END_OF_CLAUSE != 0 && more == 0) {
        eoc = true;
    }
    (more, eoc)
}

fn vowel_length_mod(list: &[Ph], ix: usize, tr: &LangSynth, sp: &Speed, more: i32) -> i32 {
    let (mut n1, mut n2, mut n3) = (ix + 1, ix + 2, ix + 3);
    if at(list, n1).code() == PHON_PAUSE_VSHORT && at(list, n2).typ == PH_PAUSE {
        (n1, n2, n3) = (n2, n3, ix + 4);
    }
    let (next, next2, next3) = (at(list, n1), at(list, n2), at(list, n3));
    let mut next2type = next2.ph.length_mod as usize;
    let next_lm = next.ph.length_mod as usize;
    let mut length_mod;
    if more == 0 {
        if (next.newword != 0 || next2.newword != 0) && next2type != 1 {
            next2type = 0;
        }
        let len = tr.length_mods0[(next2type * 10 + next_lm).min(99)] as i32;
        length_mod = if next.newword != 0 && tr.word_gap & 0x20 != 0 {
            (len + tr.length_mods0[(next_lm * 10 + 1).min(99)] as i32) / 2
        } else {
            len
        };
    } else {
        length_mod = tr.length_mods[(next2type * 10 + next_lm).min(99)] as i32;
        if next.typ == PH_NASAL && (next2.typ == PH_STOP || next2.typ == PH_VSTOP) && next3.ph.phflags & PH_VOICELESS != 0 {
            length_mod -= 15;
        }
    }
    let k = match more {
        0 => sp.len_speeds[0],
        1 => sp.len_speeds[1],
        _ => sp.len_speeds[2],
    };
    (length_mod * k / 128).max(8)
}

fn vowel(list: &mut [Ph], ix: usize, tr: &LangSynth, sp: &Speed, st: &mut State) {
    let mut stress = (list[ix].stresslevel & 7) as usize;
    let emphasized = list[ix].stresslevel & 8 != 0;
    if stress <= 1 {
        stress ^= 1;
    }
    let mut amp = if st.pre_sonorant { tr.stress_amps[stress] - 1 } else { tr.stress_amps[stress] };
    if emphasized {
        amp = 25;
    }
    if ix + 3 >= list.len() && amp > tr.maxamp_eoc {
        amp = tr.maxamp_eoc;
    }
    list[ix].amp = amp;
    let (more, eoc) = word_end_info(list, ix);
    st.more_syllables = more;
    let mut length_mod = vowel_length_mod(list, ix, tr, sp, more);
    if stress >= 7 {
        length_mod += tr.lengthen_tonic;
        if emphasized {
            length_mod += tr.lengthen_tonic / 2;
        }
    } else if emphasized {
        length_mod += tr.lengthen_tonic;
    }
    let len = if tr.stress_lengths[stress] == 0 { tr.stress_lengths[6] } else { tr.stress_lengths[stress] };
    length_mod *= len;
    if eoc && tr.stress_flags & S_NO_EOC_LENGTHEN == 0 {
        let len = if tr.stress_flags & S_EO_CLAUSE1 != 0 { 200 } else { list[ix].ph.std_length as i32 * 2 };
        length_mod = length_mod * (256 + (280 - len) / 3) / 256;
    }
    length_mod = length_mod.min(tr.max_lengthmod * sp.len_speeds[0]) / 128;
    let mut min_drop = 0;
    if list[ix].typ != PH_VOWEL {
        length_mod = 256;
        min_drop = 16;
    }
    list[ix].length = length_mod;
    pre_vocalic(list, ix, st, length_mod);
    post_vocalic(list, ix);
    let p = &mut list[ix];
    if min_drop > 0 && (p.pitch2 - p.pitch1) < min_drop {
        p.pitch1 = (p.pitch2 - min_drop).max(0);
    }
    st.last_pitch = p.pitch1 + ((p.pitch2 - p.pitch1) * ENVELOPE_DATA[p.env.min(19)][127] as i32) / 256;
    st.pre_sonorant = false;
    st.pre_voiced = false;
}

fn pre_vocalic(list: &mut [Ph], ix: usize, st: &mut State, length_mod: i32) {
    let env2 = list[ix].env + 1;
    let p = &list[ix];
    let pitch_start = p.pitch1 + ((p.pitch2 - p.pitch1) * ENVELOPE_DATA[env2.min(19)][0] as i32) / 256;
    if !(st.pre_sonorant || st.pre_voiced) {
        return;
    }
    if pitch_start == 255 {
        st.last_pitch = pitch_start;
    }
    if pitch_start - st.last_pitch > 16 {
        st.last_pitch = pitch_start - 16;
    }
    let amp = list[ix].amp;
    let rising = st.last_pitch < pitch_start;
    if rising {
        list[ix].env = env2;
    }
    let prev = &mut list[ix - 1];
    prev.pitch1 = st.last_pitch;
    prev.pitch2 = pitch_start;
    prev.env = if rising { PITCH_RISE } else { PITCH_FALL };
    prev.length = length_mod;
    prev.amp = amp;
    if prev.typ != PH_LIQUID && prev.amp > 18 {
        prev.amp = 18;
    }
}

fn post_vocalic(list: &mut [Ph], ix: usize) {
    if ix + 1 >= list.len() {
        return;
    }
    let next2_type = at(list, ix + 2).typ;
    let next = &mut list[ix + 1];
    next.synthflags &= !SFLAG_SEQCONTINUE;
    if next.typ == PH_NASAL && next2_type != PH_VOWEL {
        next.synthflags |= SFLAG_SEQCONTINUE;
    }
    if next.typ == PH_LIQUID {
        next.synthflags |= SFLAG_SEQCONTINUE;
        if next2_type == PH_VOWEL {
            next.synthflags &= !SFLAG_SEQCONTINUE;
        }
        if next2_type != PH_VOWEL && next.ph.mnemonic == (b'/' as u32) * 256 + b'r' as u32 {
            next.synthflags &= !SFLAG_SEQCONTINUE;
        }
    }
}
