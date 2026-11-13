//! `LoadVoice` for the synthesis side: replays a voice file, then each
//! variant file, over C's `voice_t` and `Translator`.

use super::voice::{LangSynth, WVoice};
use crate::synthesize::intonation::Tune;
use crate::synthesize::wavegen::set_tone_adjust;

/// The result of loading a voice file and its variants.
pub struct Loaded {
    pub voice: WVoice,
    pub tr: LangSynth,
}

/// One `%d` conversion: the integer at the start of `s` (after blanks) and
/// the rest, as `sscanf` reads it.
fn scan_int(s: &str) -> Option<(i32, &str)> {
    let s = s.trim_start();
    let sign_len = usize::from(s.starts_with(['+', '-']));
    let digits = s[sign_len..].bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let end = sign_len + digits;
    let value = s[..end].parse::<i64>().ok()?;
    Some((value as i32, &s[end..]))
}

/// `sscanf(s, "%d %d ...")` with up to `max` conversions.
fn ints(s: &str, max: usize) -> Vec<i32> {
    let mut out = Vec::new();
    let mut rest = s;
    while out.len() < max {
        let Some((v, r)) = scan_int(rest) else { break };
        out.push(v);
        rest = r;
    }
    out
}

/// The first `%39s` token, cut at the first `-` (`strtok(name, "-")`).
fn language_of(rest: &str) -> Option<&str> {
    let name = rest.split_whitespace().next()?;
    (name != "variant").then(|| name.split('-').find(|p| !p.is_empty()).unwrap_or(name))
}

fn set_from(dst: &mut [i32; 8], n: &[i32]) {
    dst.iter_mut().zip(n).for_each(|(d, &v)| *d = v);
}

/// `LoadLanguageOptions`, for the keys the synthesizer reads.
fn apply_langopt(tr: &mut LangSynth, key: &str, rest: &str, tunes: &[Tune]) {
    let n = ints(rest, 8);
    match key {
        "stressAdd" => tr.stress_lengths.iter_mut().zip(&n).for_each(|(s, a)| *s += a),
        "stressAmp" => set_from(&mut tr.stress_amps, &n),
        "stressLength" => set_from(&mut tr.stress_lengths, &n),
        "intonation" => {
            if let Some(&g) = n.first() {
                tr.option_tone_flags = g;
                if g & 0xff != 0 {
                    tr.intonation_group = (g & 0xff) as usize;
                }
            }
        }
        "words" => tr.word_gap = n.first().copied().unwrap_or(tr.word_gap),
        "l_sonorant_min" => tr.sonorant_min = n.first().copied().unwrap_or(tr.sonorant_min),
        "tunes" => apply_tunes(tr, rest, tunes),
        _ => {}
    }
}

fn apply_tunes(tr: &mut LangSynth, rest: &str, tunes: &[Tune]) {
    tr.intonation_group = 0;
    for (ix, name) in rest.split_whitespace().take(6).enumerate() {
        if let Some(k) = tunes.iter().position(|t| t.name == name) {
            tr.tunes[ix] = k;
        }
    }
}

fn apply_formant(v: &mut WVoice, n: &[i32]) {
    let get = |i: usize, d: i32| n.get(i).copied().unwrap_or(d);
    if n.len() >= 2 && (0..=8).contains(&n[0]) {
        v.set_formant(n[0] as usize, n[1], get(2, 100), get(3, 100), get(4, 0));
    }
}

fn apply_echo(v: &mut WVoice, n: &[i32]) {
    v.echo_amp = 0;
    if let Some(&d) = n.first() {
        v.echo_delay = d;
    }
    if let Some(&a) = n.get(1) {
        v.echo_amp = a;
    }
}

fn apply_clarity(v: &mut WVoice, value: i32) {
    if value > 4 {
        v.peak_shape = 1;
    }
    v.n_harmonic_peaks = 1 + value.min(4);
}

/// `Read8Numbers` into `dst[1..]`; `dst[0]` is the count it read.
fn read8_after_count(dst: &mut [i32; 9], rest: &str) {
    let n = ints(rest, 8);
    *dst = [0; 9];
    dst[0] = n.len() as i32;
    dst[1..].iter_mut().zip(&n).for_each(|(d, &x)| *d = x);
}

fn apply_breath(v: &mut WVoice, rest: &str) {
    read8_after_count(&mut v.breath, rest);
    v.breath.iter_mut().take(8).skip(1).step_by(2).for_each(|b| *b = -*b);
}

/// The voice-quality keywords of `LoadVoice`'s `keyword_tab`.
fn apply_quality(v: &mut WVoice, key: &str, rest: &str) {
    let n = ints(rest, 12);
    let first = n.first().copied();
    match (key, first) {
        ("pitch", _) if n.len() >= 2 => v.set_pitch(n[0], n[1]),
        ("formant", _) => apply_formant(v, &n),
        ("echo", _) => apply_echo(v, &n),
        ("flutter", Some(x)) => v.flutter = x * 32,
        ("roughness", Some(x)) => v.roughness = x,
        ("clarity", Some(x)) => apply_clarity(v, x),
        ("tone", _) => v.tone_adjust = set_tone_adjust(&tone_points(&n)),
        ("voicing", Some(x)) => v.voicing = x * 64 / 100,
        ("breath", _) => apply_breath(v, rest),
        ("breathw", _) => read8_after_count(&mut v.breathw, rest),
        ("speed", Some(x)) => v.speed_percent = x,
        ("klatt", _) => v.klatt = first.unwrap_or(1),
        ("consonants", Some(x)) => {
            v.consonant_amp = x;
            v.consonant_ampv = n.get(1).copied().unwrap_or(v.consonant_ampv);
        }
        _ => {}
    }
}

/// `ReadTonePoints`: up to ten numbers, the rest `-1`.
fn tone_points(n: &[i32]) -> [i32; 12] {
    let mut pts = [-1; 12];
    pts.iter_mut().zip(n.iter().take(10)).for_each(|(p, &x)| *p = x);
    pts
}

fn split_key(line: &str) -> (&str, &str) {
    line.split_once(char::is_whitespace).unwrap_or((line, ""))
}

/// Replays one file.  A base file's first `language` line selects the
/// translator; a variant (`tone_only`) keeps the one it is given.
fn replay(text: &str, v: &mut WVoice, tr: &mut Option<LangSynth>, tunes: &[Tune], tone_only: bool) {
    for line in text.lines().map(|l| l.split("//").next().unwrap_or("").trim()) {
        let (key, rest) = split_key(line);
        if key == "language" && !tone_only && tr.is_none() {
            *tr = language_of(rest).map(LangSynth::for_lang);
        }
        if let Some(t) = tr.as_mut() {
            apply_langopt(t, key, rest, tunes);
        }
        apply_quality(v, key, rest);
    }
}

/// Loads `files[0]` as the voice and the rest as its variants.
pub fn load(files: &[String], tunes: &[Tune]) -> Option<Loaded> {
    let (base, variants) = files.split_first()?;    let mut voice = WVoice::default();
    let mut tr = None;
    replay(base, &mut voice, &mut tr, tunes, false);
    let mut tr = Some(tr.unwrap_or_else(|| LangSynth::for_lang("en")));
    for text in variants {
        voice = WVoice::default();
        replay(text, &mut voice, &mut tr, tunes, true);
    }
    Some(Loaded { voice, tr: tr? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ints_stop_where_sscanf_stops() {
        assert_eq!(ints("12 -3 +4", 8), vec![12, -3, 4]);
        assert_eq!(ints("12abc 5", 8), vec![12]);
        assert_eq!(ints("1 2 3", 2), vec![1, 2]);
        assert!(ints("x 1", 8).is_empty());
    }

    #[test]
    fn breath_negates_odd_formants_and_counts() {
        let mut v = WVoice::default();
        apply_breath(&mut v, "10 20 30");
        assert_eq!(&v.breath[..5], &[3, -10, 20, -30, 0]);
    }
}
