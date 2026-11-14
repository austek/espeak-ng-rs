//! The clause phoneme list (`PHONEME_LIST`) built from translated codes, with
//! the stress handling `MakePhonemeList` does after translation.

use crate::phoneme::{
    PhonemeData, PhonemeTab, PHON_DEFAULTTONE, PHON_END_WORD, PHON_LENGTHEN, PHON_PAUSE, PHON_PAUSE_SHORT, PHON_SWITCH, PHON_SYLLABIC, PH_LIQUID,
    PH_NASAL, PH_PAUSE, PH_PREVOICE, PH_STRESS, PH_VFRICATIVE, PH_VOWEL, PH_VSTOP,
};
use crate::translate::{CodeMarker, EmbeddedCmd, PhonemeCode};

pub const SFLAG_SEQCONTINUE: u16 = 0x01;
pub const SFLAG_EMBEDDED: u16 = 0x02;
pub const SFLAG_SYLLABLE: u16 = 0x04;
pub const SFLAG_LENGTHEN: u16 = 0x08;
pub const SFLAG_NEXT_PAUSE: u16 = 0x2000;

pub const PHLIST_START_OF_WORD: u8 = 1;
pub const PHLIST_END_OF_CLAUSE: u8 = 2;
pub const PHLIST_START_OF_SENTENCE: u8 = 4;
pub const PHLIST_START_OF_CLAUSE: u8 = 8;

/// One entry of C's `phoneme_list`.
#[derive(Clone, Debug)]
pub struct Ph {
    pub ph: PhonemeTab,
    pub typ: u8,
    pub stresslevel: u8,
    pub wordstress: u8,
    pub synthflags: u16,
    pub newword: u8,
    pub length: i32,
    pub env: usize,
    pub prepause: i32,
    pub amp: i32,
    pub pitch1: i32,
    pub pitch2: i32,
    pub embedded: Vec<EmbeddedCmd>,
    /// A pause `MakePhonemeList` inserts between words, after it has looked
    /// ahead from the phonemes before it.
    pub inserted: bool,
    /// The tone phoneme of a tone-language syllable (C's `tone_ph`).
    pub tone: Option<PhonemeTab>,
    pub tone_env: ToneEnv,
}

/// What a tone phoneme's program selects: phondata offsets of its pitch and
/// amplitude envelopes (0 = none) and the first byte of the pitch envelope.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToneEnv {
    pub pitch: u32,
    pub amp: u32,
    pub first: i32,
}

impl Ph {
    fn new(ph: PhonemeTab, stress: u8, newword: u8) -> Self {
        Ph {
            typ: ph.typ,
            ph,
            stresslevel: stress,
            wordstress: 0,
            synthflags: 0,
            newword,
            length: 0,
            env: 0,
            prepause: 0,
            amp: 20,
            pitch1: 255,
            pitch2: 255,
            embedded: Vec::new(),
            inserted: false,
            tone: None,
            tone_env: ToneEnv::default(),
        }
    }

    pub fn code(&self) -> u8 {
        self.ph.code
    }
}

/// A translated clause: its phonemes and the punctuation that ended it.
pub struct Clause {
    pub list: Vec<Ph>,
    pub clause_char: Option<char>,
    pub embedded_after: Vec<EmbeddedCmd>,
}

fn tab(phdata: &PhonemeData, code: u8) -> PhonemeTab {
    phdata.get(code).cloned().unwrap_or_else(|| phdata.get(PHON_PAUSE).cloned().unwrap_or_default())
}

/// One clause of the code stream, as `TranslateClause` hands it over.
pub struct ClauseCodes {
    pub codes: Vec<PhonemeCode>,
    pub clause_char: Option<char>,
    /// An SSML `<break>` that ended the clause: its length in ms, and whether
    /// it is wall-clock exact (`time`) rather than in clause units (`strength`).
    pub brk: Option<(i32, bool)>,
}

fn take_break(cur: &mut Vec<PhonemeCode>) -> Option<(i32, bool)> {
    let pos = cur.iter().rposition(|c| {
        matches!(&c.marker, Some(CodeMarker::Embedded(e)) if e.letter == crate::translate::ssml::BREAK_LETTER)
    })?;
    if cur[pos + 1..].iter().any(|c| c.code != 0 && !c.is_boundary && c.code != PHON_END_WORD) {
        return None;
    }
    match cur.remove(pos).marker {
        Some(CodeMarker::Embedded(e)) => Some((e.value.max(0), e.relative == 1)),
        _ => None,
    }
}

/// Split the code stream into clauses.
pub fn split_clauses(codes: &[PhonemeCode]) -> Vec<ClauseCodes> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for c in codes {
        if c.code == 0 && c.is_boundary && c.marker.is_none() {
            let brk = take_break(&mut cur);
            out.push(ClauseCodes { codes: std::mem::take(&mut cur), clause_char: c.clause_char, brk });
        } else {
            cur.push(c.clone());
        }
    }
    if cur.iter().any(|c| c.code != 0 || c.marker.is_some()) {
        out.push(ClauseCodes { codes: cur, clause_char: None, brk: None });
    }
    out
}

/// The phoneme list for one clause: pause, phonemes, the clause pause of
/// `post_pause` ms and a closing pause, as `MakePhonemeList` leaves it.
pub fn build_list(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    switched: &std::collections::HashMap<String, PhonemeData>,
    post_pause: i32,
    start_sentence: bool,
    stress_flags: u32,
) -> Clause {
    let mut list = vec![Ph::new(tab(phdata, PHON_PAUSE_SHORT), 0, 0)];
    let mut stress: Option<u8> = None;
    let mut newword = PHLIST_START_OF_WORD | PHLIST_START_OF_CLAUSE | if start_sentence { PHLIST_START_OF_SENTENCE } else { 0 };
    let mut pending: Vec<EmbeddedCmd> = Vec::new();
    let mut table = phdata;
    let mut promoted: Option<u8> = None;
    let mut prev_vowel: Option<usize> = None;
    let mut next_tone: Option<PhonemeTab> = None;
    let mut inserted: Option<i32> = None;
    let mut gaps: Vec<(usize, i32)> = Vec::new();
    let mut insert = Insert::None;
    let mut bare: Vec<usize> = Vec::new();
    for (idx, c) in codes.iter().enumerate() {
        match &c.marker {
            Some(CodeMarker::InsertedBefore) => {
                insert = Insert::Marked;
                continue;
            }
            Some(CodeMarker::Embedded(cmd)) => {
                pending.push(*cmd);
                continue;
            }
            Some(CodeMarker::PromotedTonic(original)) => {
                promoted = Some(*original);
                continue;
            }
            Some(CodeMarker::InsertedPause(units)) => {
                inserted = Some(*units);
                continue;
            }
            Some(CodeMarker::LangSwitch(lang)) => {
                let opening = switched.contains_key(lang);
                table = switched.get(lang).unwrap_or(phdata);
                list.push(Ph::new(tab(phdata, PHON_SWITCH), 0, if opening { newword } else { 0 }));
                if opening {
                    newword = 0;
                }
                continue;
            }
            Some(_) => continue,
            None => {}
        }
        if c.code == PHON_END_WORD || (c.is_boundary && c.code == 0) {
            newword |= PHLIST_START_OF_WORD;
            prev_vowel = None;
            next_tone = None;
            continue;
        }
        let Some(t) = table.get(c.code) else { continue };
        if c.code == PHON_DEFAULTTONE && t.typ != PH_STRESS {
            continue; // the voice's default tone written into a switched table
        }
        if t.typ == PH_STRESS && t.program != 0 {
            // The translator writes the default tone for toneless syllables;
            // C leaves those to CalcPitches_Tone.
            let tone = (c.code != PHON_DEFAULTTONE).then(|| t.clone());
            match (prev_vowel, tone) {
                (Some(i), tone) => list[i].tone = tone,
                (None, tone) => next_tone = tone,
            }
            continue;
        }
        if t.typ == PH_STRESS {
            stress = match promoted.take() {
                Some(0) => None,
                Some(code) => table.get(code).map(|o| o.std_length),
                None => Some(t.std_length),
            };
            continue;
        }
        if c.code == PHON_LENGTHEN {
            if let Some(last) = list.last_mut() {
                last.synthflags |= SFLAG_LENGTHEN;
            }
            continue;
        }
        if c.code == PHON_SYLLABIC {
            if let Some(last) = list.last_mut() {
                last.synthflags |= SFLAG_SYLLABLE;
                last.stresslevel = stress.unwrap_or(1);
                prev_vowel = Some(list.len() - 1);
            }
            continue;
        }
        let before = std::mem::replace(&mut insert, Insert::None);
        let s = entry_stress(before, t.typ, &mut stress, || follows_vowel(&codes[idx + 1..], table));
        let mut p = Ph::new(t.clone(), s, newword);
        newword = 0;
        if before == Insert::Neighbour {
            bare.push(list.len());
        }
        if let Some(units) = inserted.take() {
            p.inserted = true;
            if units > 0 && t.typ == PH_PAUSE {
                gaps.push((list.len(), units * 14));
            }
        }
        if before == Insert::Neighbour {
            insert = Insert::None;
        } else if t.typ == PH_VOWEL {
            p.synthflags |= SFLAG_SYLLABLE;
            p.tone = next_tone.take();
            prev_vowel = Some(list.len());
        }
        if before == Insert::Marked {
            insert = Insert::Neighbour;
        }
        if !pending.is_empty() {
            p.synthflags |= SFLAG_EMBEDDED;
            p.embedded = std::mem::take(&mut pending);
        }
        list.push(p);
    }
    finish_list(&mut list, phdata, post_pause);
    gaps.into_iter().for_each(|(i, length)| list[i].length = length);
    set_word_stress(&mut list);
    bare.iter().for_each(|&i| list[i].wordstress = 0);
    diminish_unstressed(&mut list, stress_flags);
    Clause { list, clause_char: None, embedded_after: pending }
}

/// Where the phoneme being added stands to a phoneme a program inserted
/// before another: the inserted one, or that neighbour added again.
#[derive(Clone, Copy, PartialEq)]
enum Insert {
    None,
    Marked,
    Neighbour,
}

/// The inserted phoneme takes the neighbour's pending stress, and is
/// unstressed when it is a vowel before a consonant; the neighbour is added
/// again with no stress.
fn entry_stress(insert: Insert, typ: u8, pending: &mut Option<u8>, next_is_vowel: impl Fn() -> bool) -> u8 {
    match insert {
        Insert::None if typ == PH_VOWEL => pending.take().unwrap_or(1),
        Insert::Marked if next_is_vowel() => {
            let s = pending.take().unwrap_or(1);
            if typ == PH_VOWEL { s } else { 0 }
        }
        _ => 0,
    }
}

fn follows_vowel(rest: &[PhonemeCode], table: &PhonemeData) -> bool {
    rest.iter()
        .filter(|c| c.marker.is_none() && !c.is_boundary)
        .find_map(|c| table.get(c.code).filter(|t| t.typ != PH_STRESS))
        .is_some_and(|t| t.typ == PH_VOWEL)
}

fn finish_list(list: &mut Vec<Ph>, phdata: &PhonemeData, post_pause: i32) {
    for p in list.iter_mut().skip(1) {
        p.length = p.ph.std_length as i32 * 2;
        let t = p.typ;
        if t == PH_VOWEL || t == PH_LIQUID || t == PH_NASAL || t == PH_VSTOP || t == PH_VFRICATIVE || p.ph.phflags & PH_PREVOICE != 0 {
            p.length = 128;
        }
    }
    list[0].length = list[0].ph.std_length as i32 * 2;
    let mut end = Ph::new(tab(phdata, PHON_PAUSE), 0, PHLIST_END_OF_CLAUSE);
    end.length = post_pause;
    list.push(end);
    let mut last = Ph::new(tab(phdata, PHON_PAUSE_SHORT), 0, 0);
    last.typ = PH_PAUSE;
    list.push(last);
}

fn set_word_stress(list: &mut [Ph]) {
    let mut start = 1;
    while start < list.len() {
        let mut end = start + 1;
        while end < list.len() && list[end].newword == 0 {
            end += 1;
        }
        let top = list[start..end].iter().map(|p| p.stresslevel).max().unwrap_or(0);
        list[start..end].iter_mut().for_each(|p| p.wordstress = top);
        start = end;
    }
}

/// The consecutive-unstressed-vowel pass of `MakePhonemeList`.  Its test for
/// "a stressed word" reads C's `word_stress`, which still holds the clause's
/// last word's stress from the pass that set `wordstress`.
fn diminish_unstressed(list: &mut [Ph], stress_flags: u32) {
    const S_NO_DIM: u32 = 0x02;
    let word_stress = list.len().checked_sub(3).map_or(0, |i| list[i].wordstress);
    let mut unstress_count = 0;
    for i in 1..list.len() {
        if list[i].typ != PH_VOWEL {
            continue;
        }
        if list[i].stresslevel > 1 {
            unstress_count = 0;
            continue;
        }
        unstress_count += 1;
        if stress_flags & 0x08 != 0 {
            diminish_pair(list, i);
        } else if unstress_count > 1 && unstress_count & 1 == 0 {
            let next_word = list[i + 1..].iter().find(|n| !n.inserted).is_some_and(|n| n.newword != 0);
            if stress_flags & S_NO_DIM != 0 || (word_stress > 3 && next_word) {
                unstress_count = 1;
            } else {
                list[i].stresslevel = 0;
            }
        }
    }
}

fn diminish_pair(list: &mut [Ph], i: usize) {
    for j in i + 1..list.len() {
        if list[j].inserted {
            continue;
        }
        if list[j].typ == PH_PAUSE {
            return;
        }
        if list[j].typ == PH_VOWEL {
            if list[j].stresslevel <= 1 {
                if list[i].wordstress < 4 {
                    list[i].stresslevel = 0;
                }
                if list[j].wordstress < 4 {
                    list[j].stresslevel = 0;
                }
            }
            return;
        }
    }
}
