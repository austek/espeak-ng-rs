//! `ScoreVoice`, `SetVoiceScores`, `SelectVoiceByName` and `SelectVoice` from `voices.c`.

use super::VoiceEntry;

pub const GENDER_UNKNOWN: i32 = 0;
pub const GENDER_MALE: i32 = 1;
pub const GENDER_FEMALE: i32 = 2;

const DEFAULT_VOICE: &str = "en";
const N_VOICE_VARIANTS: usize = 12;
const AGE_OLD: i32 = 60;
const VARIANTS_EITHER: [i32; 9] = [1, 2, 12, 3, 13, 4, 14, 5, 11];
const VARIANTS_MALE: [i32; 6] = [1, 2, 3, 4, 5, 6];
const VARIANTS_FEMALE: [i32; 4] = [11, 12, 13, 14];

/// The `espeak_VOICE` fields `SelectVoice` reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct VoiceRequest<'a> {
    pub name: Option<&'a str>,
    pub identifier: Option<&'a str>,
    pub language: Option<&'a str>,
    pub age: i32,
    pub gender: i32,
    pub variant: i32,
}

/// What `SelectVoice` returns: the voice file name with any `+variant`, and `*found`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub id: String,
    pub found: bool,
}

fn entry_gender(v: &VoiceEntry) -> i32 {
    if v.gender == 'F' { GENDER_FEMALE } else { GENDER_MALE }
}

fn entry_name(v: &VoiceEntry) -> &str {
    if v.name.is_empty() { &v.file } else { &v.name }
}

/// `espeak_ListVoices`' order: primary language, priority, name.
pub fn sorted_voices(list: &[VoiceEntry]) -> Vec<&VoiceEntry> {
    let mut out: Vec<&VoiceEntry> = list.iter().filter(|v| !v.languages.is_empty()).collect();
    out.sort_by(|a, b| {
        (a.languages[0].0.as_bytes(), a.languages[0].1, entry_name(a).as_bytes())
            .cmp(&(b.languages[0].0.as_bytes(), b.languages[0].1, entry_name(b).as_bytes()))
    });
    out
}

/// C `SelectVoiceByName`: the voice name, else its file path, else the last component of its path.
pub fn voice_by_name<'a>(list: &[&'a VoiceEntry], name: &str) -> Option<&'a VoiceEntry> {
    let tail = format!("/{name}").to_ascii_lowercase();
    let mut by_file = None;
    let mut by_tail = None;
    for v in list {
        if entry_name(v).eq_ignore_ascii_case(name) {
            return Some(v);
        }
        let id = v.file.to_ascii_lowercase();
        if id == name.to_ascii_lowercase() {
            by_file = Some(*v);
        } else if id.ends_with(&tail) {
            by_tail = Some(*v);
        }
    }
    by_file.or(by_tail)
}

/// The comparison of one voice language (`p`) with the requested one, as `ScoreVoice` makes it.
fn language_score(spec: &[u8], spec_n_parts: i32, code: &str, priority: i32) -> i32 {
    let (mut n_parts, mut matching, mut matching_parts) = (1, true, 0);
    let code = code.as_bytes();
    for ix in 0..=code.len() {
        let c1 = match spec.get(ix) {
            Some(&b) if b != b'-' => b,
            _ => 0,
        };
        let raw = code.get(ix).copied().unwrap_or(0);
        let c2 = if raw == b'-' { 0 } else { raw };
        matching &= c1 == c2;
        if raw == b'-' {
            n_parts += 1;
            matching_parts += i32::from(matching);
        }
    }
    matching_parts += i32::from(matching);
    if matching_parts == 0 {
        return 0;
    }
    let mut x = 5;
    x -= (spec_n_parts - matching_parts).max(0);
    x -= (n_parts - matching_parts).max(0);
    x * 100 - priority * 2
}

fn age_score(spec_age: i32, v: &VoiceEntry) -> i32 {
    let mut score = 0;
    if spec_age <= 12 && entry_gender(v) == GENDER_FEMALE && v.age > 12 {
        score += 5;
    }
    if v.age != 0 {
        let required = if spec_age == 0 { 30 } else { spec_age };
        let mut ratio = required * 100 / i32::from(v.age);
        if ratio < 100 {
            ratio = 10000 / ratio.max(1);
        }
        score += (5 - (ratio - 100) / 10).min(0);
        if spec_age > 0 {
            score += 10;
        }
    }
    score
}

fn score_voice(req: &VoiceRequest, language: &[u8], n_parts: i32, v: &VoiceEntry) -> i32 {
    let mut score = if n_parts == 0 { 100 } else { 0 };
    if n_parts != 0 {
        for (code, priority) in &v.languages {
            score = score.max(language_score(language, n_parts, code, i32::from(*priority)));
        }
    }
    if score == 0 {
        return 0;
    }
    if let Some(name) = req.name {
        if name == entry_name(v) {
            score += 500;
        } else if name == v.file {
            score += 400;
        }
    }
    let gender = entry_gender(v);
    if matches!(req.gender, GENDER_MALE | GENDER_FEMALE) {
        score += if req.gender == gender { 50 } else { -50 };
    }
    (score + age_score(req.age, v)).max(1)
}

fn language_parts(language: &str) -> (Vec<u8>, i32) {
    let bytes = language.to_ascii_lowercase().into_bytes();
    let parts = if bytes.is_empty() { 0 } else { 1 + bytes.iter().filter(|&&b| b == b'-').count() as i32 };
    (bytes, parts)
}

/// `SetVoiceScores`: the voices that match the request, best first.
fn scored<'a>(list: &[&'a VoiceEntry], req: &VoiceRequest, language: &str) -> Vec<&'a VoiceEntry> {
    let candidates = list.iter().filter(|v| !v.file.starts_with("mb/"));
    if language.starts_with("all") {
        return candidates.copied().collect();
    }
    let (bytes, parts) = language_parts(language);
    let mut hits: Vec<(i32, &VoiceEntry)> = candidates
        .filter_map(|v| Some((score_voice(req, &bytes, parts, v), *v)).filter(|(s, _)| *s > 0))
        .collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| entry_name(a.1).as_bytes().cmp(entry_name(b.1).as_bytes())));
    hits.into_iter().map(|(_, v)| v).collect()
}

/// `ExtractVoiceVariantName` for a request with no variant number: the `+variant` suffix as a
/// `!v/` file name, a number mapped to `m<n>` or `f<n-10>`.
fn split_variant(name: &str) -> (&str, String) {
    let Some((base, suffix)) = name.split_once('+') else { return (name, String::new()) };
    let digits: String = suffix.chars().take_while(char::is_ascii_digit).collect();
    if suffix.starts_with(|c: char| c.is_ascii_digit()) {
        return (base, variant_file(digits.parse().unwrap_or(0)));
    }
    (base, suffix.to_string())
}

/// `m<n>` below 10, `f<n-10>` from 10; empty for 0.
pub fn variant_file(variant: i32) -> String {
    match variant {
        v if v <= 0 => String::new(),
        v if v < 10 => format!("m{v}"),
        v => format!("f{}", v - 10),
    }
}

fn variant_list(gender: i32) -> &'static [i32] {
    match gender {
        GENDER_MALE => &VARIANTS_MALE,
        GENDER_FEMALE => &VARIANTS_FEMALE,
        _ => &VARIANTS_EITHER,
    }
}

/// One entry of `voices2`: a voice and the variant number it carries.
type Candidate<'a> = (&'a VoiceEntry, i32);

/// The top voices with their variants, then the remaining variants of the last voice.
fn with_variants<'a>(voices: &[&'a VoiceEntry], gender: i32, aged: bool) -> Vec<Candidate<'a>> {
    let all = variant_list(gender);
    let start = usize::from(!aged);
    let mut p = start;
    let mut out: Vec<Candidate> = Vec::new();
    let mut n_variants = 0;
    for v in voices {
        let skip = (gender != GENDER_UNKNOWN && entry_gender(v) != gender) || (out.is_empty() && aged && i32::from(v.age) < AGE_OLD);
        if !skip {
            out.push((v, 0));
        }
        let mut j = 0;
        while j < v.variants.unwrap_or(4) && n_variants < N_VOICE_VARIANTS {
            match all.get(p) {
                None => p = start,
                Some(&n) => {
                    out.push((v, n));
                    n_variants += 1;
                    p += 1;
                    j += 1;
                }
            }
        }
    }
    if let Some(last) = voices.last() {
        while let Some(&n) = all.get(p) {
            if n_variants >= N_VOICE_VARIANTS {
                break;
            }
            out.push((last, n));
            n_variants += 1;
            p += 1;
        }
    }
    out
}

/// C `SelectVoice`: the best voice for the request, `None` when no candidate remains.
pub fn select_voice(list: &[VoiceEntry], req: &VoiceRequest) -> Option<Selection> {
    let sorted = sorted_voices(list);
    let mut language = req.language.unwrap_or("").to_string();
    let mut spec_name = req.name;
    if language.is_empty() {
        let name = req.name.or(req.identifier).unwrap_or(DEFAULT_VOICE);
        spec_name = Some(name);
        let (base, variant_name) = split_variant(name);
        if let Some(vp) = voice_by_name(&sorted, base) {
            language = vp.languages.first().map(|l| l.0.clone()).unwrap_or_default();
            if req.gender == GENDER_UNKNOWN && req.age == 0 && req.variant == 0 {
                let id = if variant_name.is_empty() { vp.file.clone() } else { format!("{}+{variant_name}", vp.file) };
                return Some(Selection { id, found: true });
            }
        }
    }
    let spec = VoiceRequest { name: spec_name, ..*req };
    let mut voices = scored(&sorted, &spec, &language);
    let found = !voices.is_empty();
    if !found {
        voices = voice_by_name(&sorted, DEFAULT_VOICE).into_iter().collect();
    }
    let gender = match (req.gender, req.age) {
        (GENDER_FEMALE, _) | (_, 1..=12) => GENDER_FEMALE,
        (GENDER_MALE, _) => GENDER_MALE,
        _ => GENDER_UNKNOWN,
    };
    let candidates = with_variants(&voices, gender, req.age >= AGE_OLD);
    let (vp, variant) = *candidates.get(usize::try_from(req.variant).ok()? % candidates.len().max(1))?;
    let id = match variant_file(variant) {
        v if v.is_empty() => vp.file.clone(),
        v => format!("{}+{v}", vp.file),
    };
    Some(Selection { id, found })
}
