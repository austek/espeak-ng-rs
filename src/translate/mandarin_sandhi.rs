//! Mandarin tone sandhi over the finished phoneme list: a port of the Mandarin
//! branch of `CalcPitches_Tone` (`intonation.c`).  C runs it over every syllable
//! of the clause, so an English word inside Chinese text takes a tone too.

use std::collections::HashSet;
use std::iter::Peekable;
use std::rc::Rc;

use super::letters::{AltLangs, LangCtx};
use super::{is_tone_phoneme, phoneme_mnemonic, CodeMarker, PhonemeCode};
use crate::phoneme::{PhonemeData, PHON_LENGTHEN};

const PHON_DEFAULT_TONE: u8 = 17;

struct Slot {
    vowel: usize,
    tone: usize,
    name: String,
    pause: bool,
    new_word: bool,
}

struct Walk<'a> {
    voice: &'a PhonemeData,
    alts: &'a AltLangs,
    voice_name: &'a str,
    table: Option<Rc<LangCtx>>,
    pause: bool,
    new_word: bool,
}

impl Walk<'_> {
    fn phdata(&self) -> &PhonemeData {
        self.table.as_ref().map_or(self.voice, |t| &t.phdata)
    }

    fn is_vowel(&self, code: u8) -> bool {
        matches!(self.phdata().get(code), Some(ph) if ph.typ == 2)
    }

    fn switch(&mut self, target: &str) {
        self.table = if target == self.voice_name { None } else { self.alts.get(target) };
    }
}

fn tone_code(code: u8) -> PhonemeCode {
    PhonemeCode { code, voice_table: true, ..Default::default() }
}

fn collect_slots<I: Iterator<Item = PhonemeCode>>(
    codes: &mut Peekable<I>,
    walk: &mut Walk,
) -> (Vec<PhonemeCode>, Vec<Slot>) {
    let mut out: Vec<PhonemeCode> = Vec::new();
    let mut slots: Vec<Slot> = Vec::new();
    while let Some(c) = codes.next() {
        if let Some(CodeMarker::LangSwitch(target)) = &c.marker {
            walk.switch(target);
        }
        let vowel = c.marker.is_none() && !c.is_boundary && walk.is_vowel(c.code);
        if c.is_boundary {
            walk.new_word = true;
            walk.pause |= c.clause_char.is_some();
        }
        out.push(c);
        if vowel {
            let vowel_ix = out.len() - 1;
            while codes.peek().is_some_and(|n| n.code == PHON_LENGTHEN && n.marker.is_none()) {
                out.extend(codes.next());
            }
            let given = codes.next_if(|n| n.marker.is_none() && is_tone_phoneme(n.code, walk.phdata()));
            let name = given.as_ref().map(|t| phoneme_mnemonic(t.code, walk.phdata()));
            out.push(given.unwrap_or_else(|| tone_code(PHON_DEFAULT_TONE)));
            slots.push(Slot {
                vowel: vowel_ix,
                tone: out.len() - 1,
                name: name.unwrap_or_default(),
                pause: std::mem::take(&mut walk.pause),
                new_word: std::mem::take(&mut walk.new_word),
            });
        }
    }
    (out, slots)
}

fn neutral_level(prevw: &str) -> Option<&'static str> {
    match prevw {
        "55" => Some("22"),
        "35" => Some("33"),
        "214" => Some("44"),
        _ => None,
    }
}

fn apply_sandhi(out: &mut [PhonemeCode], slots: &[Slot], voice: &PhonemeData) -> HashSet<usize> {
    let code_of = |m: &str| voice.lookup_phoneme(m);
    let set = |out: &mut [PhonemeCode], at: usize, mnemonic: &str| {
        let code = code_of(mnemonic);
        if code != 0 {
            out[at] = tone_code(code);
        }
    };
    let (mut prev_at, mut prev, mut prevw) = (None::<usize>, String::new(), String::new());
    let mut promoted = false;
    let mut unstressed = HashSet::new();
    for s in slots {
        if s.pause {
            prevw.clear();
        }
        if s.new_word {
            prev.clear();
        }
        let tone = if s.name.is_empty() {
            let name = if s.pause || promoted { "55" } else { "11" };
            promoted = name == "55";
            set(out, s.tone, name);
            name.to_string()
        } else {
            promoted = false;
            s.name.clone()
        };
        if let Some(p) = prev_at {
            if prevw == "214" {
                set(out, p, if tone == "214" { "35" } else { "21" });
            }
            if prev == "51" && tone == "51" {
                set(out, p, "53");
            }
        }
        if tone == "11" {
            unstressed.insert(s.vowel);
            if let Some(level) = neutral_level(&prevw) {
                set(out, s.tone, level);
            }
        }
        (prev_at, prev, prevw) = (Some(s.tone), tone.clone(), tone);
    }
    unstressed
}

fn drop_stress(out: Vec<PhonemeCode>, vowels: &HashSet<usize>) -> Vec<PhonemeCode> {
    let stress_before: HashSet<usize> = vowels
        .iter()
        .filter(|&&v| v > 0 && matches!(out[v - 1].code, 3..=7) && out[v - 1].marker.is_none())
        .map(|v| v - 1)
        .collect();
    out.into_iter()
        .enumerate()
        .filter(|(i, _)| !stress_before.contains(i))
        .map(|(_, c)| c)
        .collect()
}

/// Give every vowel of the clause a tone and apply Mandarin sandhi across them.
pub(super) fn apply(
    codes: Vec<PhonemeCode>,
    voice: &PhonemeData,
    voice_name: &str,
    alts: &AltLangs,
) -> Vec<PhonemeCode> {
    let mut walk = Walk { voice, alts, voice_name, table: None, pause: true, new_word: true };
    let (mut out, slots) = collect_slots(&mut codes.into_iter().peekable(), &mut walk);
    let unstressed = apply_sandhi(&mut out, &slots, voice);
    drop_stress(out, &unstressed)
}
