//! `CalcPitches` from `intonation.c`, over the clause phoneme list.

use super::phlist::{Ph, SFLAG_SYLLABLE};
use super::voice::LangSynth;
use crate::phoneme::PHON_PAUSE_CLAUSE;
use crate::synthesize::intonation::{
    calc_pitches_table, calc_pitches_tune, count_pitch_vowels, tone_for_clause, Syllable, Tune,
    SYL_EMPHASIS, SYL_END_CLAUSE, SYL_RISE,
};

const PITCH_FALL: usize = 0;
const PITCH_RISE: usize = 2;

struct Ctx<'a> {
    tr: &'a LangSynth,
    tunes: &'a [Tune],
    no_tonic: bool,
}

impl Ctx<'_> {
    fn calc(&self, syls: &mut [Syllable], start: usize, end: usize, tone: usize) {
        let n = syls.len();
        let layout = count_pitch_vowels(syls, start, end, n, self.no_tonic);
        self.apply(syls, start, end, tone, &layout);
    }

    fn calc_clause_end(&self, syls: &mut [Syllable], start: usize, end: usize, tone: usize) {
        let layout = count_pitch_vowels(syls, start, end, end, self.no_tonic);
        self.apply(syls, start, end, tone, &layout);
    }

    fn apply(&self, syls: &mut [Syllable], start: usize, end: usize, tone: usize, layout: &crate::synthesize::intonation::Layout) {
        if self.tr.intonation_group == 0 {
            if let Some(t) = self.tunes.get(tone) {
                calc_pitches_tune(syls, start, end, t, layout, self.no_tonic);
            }
        } else {
            calc_pitches_table(syls, start, end, tone, layout, self.no_tonic);
        }
    }

    fn group_tone(&self, clause_type: usize) -> (usize, usize) {
        let g = self.tr.intonation_group;
        if g == 0 {
            (self.tr.tunes[clause_type.min(5)], self.tr.tunes[1])
        } else {
            (tone_for_clause(g, clause_type), tone_for_clause(g, 1))
        }
    }
}

fn collect(list: &[Ph]) -> (Vec<Syllable>, usize) {
    let mut syls: Vec<Syllable> = Vec::new();
    let mut n_primary = 0;
    for p in &list[..list.len() - 1] {
        if p.synthflags & SFLAG_SYLLABLE != 0 {
            syls.push(Syllable::new(p.stresslevel));
            if p.stresslevel >= 4 {
                n_primary += 1;
            }
        } else if p.code() == PHON_PAUSE_CLAUSE {
            if let Some(s) = syls.last_mut() {
                s.flags |= SYL_END_CLAUSE;
            }
        }
    }
    (syls, n_primary)
}

fn emphasis_step(syls: &mut [Syllable], st_ix: usize, st_start: usize) {
    let lo = st_start.max(st_ix.saturating_sub(3));
    for ix in (lo..st_ix).rev() {
        if syls[ix].stress == 6 {
            break;
        }
        if syls[ix].stress == 4 {
            syls[ix].stress = 3;
            break;
        }
    }
    for ix in st_ix + 1..syls.len() {
        if syls[ix].stress == 4 {
            break;
        }
        if syls[ix].stress == 6 {
            syls[st_ix].flags = SYL_EMPHASIS;
            syls[st_ix].stress = 5;
            break;
        }
    }
}

/// Assign `env`, `pitch1` and `pitch2` to each syllable of the clause.
pub fn calc_pitches(list: &mut [Ph], tr: &LangSynth, tunes: &[Tune], clause_type: usize) {
    let (mut syls, n_primary) = collect(list);
    let n_st = syls.len();
    if n_st == 0 {
        return;
    }
    let ctx = Ctx { tr, tunes, no_tonic: clause_type == 4 };
    let (mut group_tone, group_tone_comma) = ctx.group_tone(clause_type);
    let mut st_start = 0;
    let mut count_primary = 0;
    let mut st_ix = 0;
    while st_ix < n_st {
        if syls[st_ix].stress >= 4 {
            count_primary += 1;
        }
        if syls[st_ix].stress == 6 {
            emphasis_step(&mut syls, st_ix, st_start);
        }
        if syls[st_ix].stress == 6 {
            syls[st_ix].flags = SYL_EMPHASIS;
            let mut count = if n_primary - count_primary > 1 { 1 } else { 0 };
            let mut ix = st_ix + 1;
            while ix < n_st {
                if syls[ix].stress > 4 {
                    break;
                }
                if syls[ix].stress == 4 {
                    count += 1;
                    if count > 1 {
                        break;
                    }
                }
                ix += 1;
            }
            ctx.calc(&mut syls, st_start, ix, group_tone);
            if ix < n_st || clause_type == 0 {
                group_tone = if clause_type == 1 || clause_type == 2 { tr.tunes[1] } else { tr.tunes[0] };
            }
            st_start = ix;
        }
        if st_start < st_ix && syls[st_ix].flags & SYL_END_CLAUSE != 0 {
            let end = st_ix + 1;
            ctx.calc_clause_end(&mut syls, st_start, end, group_tone_comma);
            st_start = end;
        }
        st_ix += 1;
    }
    if st_start < st_ix {
        ctx.calc(&mut syls, st_start, st_ix, group_tone);
    }
    unpack(list, &syls);
}

fn unpack(list: &mut [Ph], syls: &[Syllable]) {
    let mut st_ix = 0;
    let zero = Syllable::new(0);
    for p in list.iter_mut() {
        let syl = syls.get(st_ix).unwrap_or(&zero);
        p.stresslevel = syl.stress;
        if p.synthflags & SFLAG_SYLLABLE == 0 {
            continue;
        }
        p.pitch1 = syl.pitch1;
        p.pitch2 = syl.pitch2;
        p.env = if syl.flags & SYL_RISE != 0 {
            PITCH_RISE
        } else if p.stresslevel > 5 {
            syl.env as usize
        } else {
            PITCH_FALL
        };
        if p.pitch1 > p.pitch2 {
            std::mem::swap(&mut p.pitch1, &mut p.pitch2);
        }
        if syl.flags & SYL_EMPHASIS != 0 {
            p.stresslevel |= 8;
        }
        st_ix += 1;
    }
}
