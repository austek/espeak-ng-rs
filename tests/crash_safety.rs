// tests/crash_safety.rs
//
// Regression tests locking in memory-safety guarantees for inputs that crash
// (segfault) the upstream C engine.  Rust is memory-safe, so the point is to
// assert these paths return cleanly (Ok or a typed Err) rather than panic.
// Skips when the relevant language data is absent.

use espeak_ng::Translator;

fn works(lang: &str) -> Option<Translator> {
    let t = Translator::new(lang, None).ok()?;
    // Only run if this language's data actually resolves.
    t.text_to_ipa("a").is_ok().then_some(t)
}

#[test]
fn greek_replace_group_chars_do_not_panic() {
    // Upstream espeak-ng #2467: certain Greek letters (ϖ = U+03D6) inside a
    // `.replace` group segfault the C engine.  The Rust port must not panic.
    let Some(t) = works("el") else { eprintln!("[SKIP] no el data"); return };
    for input in ["\u{03D6}", "\u{03D6}\u{03D6}\u{03D6}", "\u{03D6} τεστ", "αϖβ"] {
        // The result content is irrelevant; it must simply not panic.
        let _ = t.text_to_ipa(input);
    }
}

#[test]
fn unknown_and_empty_voice_error_cleanly() {
    // Upstream #2469 segfaults on an empty/unknown phoneme table.  Here it must
    // surface as a typed error, never a panic.
    let Some(_) = works("en") else { eprintln!("[SKIP] no en data"); return };

    let t = Translator::new("definitely-not-a-language", None).expect("builds lazily");
    assert!(t.text_to_ipa("hello").is_err(), "unknown language must error");

    let t = Translator::new("", None).expect("builds lazily");
    assert!(t.text_to_ipa("hello").is_err(), "empty language must error");
}

#[test]
fn adversarial_inputs_do_not_panic() {
    // A grab-bag of odd inputs that have historically tripped tokenizers.
    let Some(t) = works("en") else { return };
    for input in [
        "",
        "   ",
        "\u{0000}\u{FFFD}",
        "((([[[",         // unterminated brackets
        "[[",             // unterminated inline-phoneme open
        "a\u{200B}b",     // zero-width space
        "😀🇬🇧",           // emoji + flag sequence
        "\t\n\r",
        &"a,".repeat(200), // many clause boundaries
    ] {
        let _ = t.text_to_ipa(input);
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
}

const PUNCT: &[char] = &[
    '.', ',', ';', ':', '!', '?', '-', '(', ')', '[', ']', '{', '}', '<', '>', '"', '\'', '`', '@',
    '#', '$', '%', '&', '*', '_', '+', '=', '/', ' ', '\t', '\n', '\u{2026}', '\u{3002}',
    '\u{0964}', '\u{061f}', '\u{00bf}',
];

const SENTENCES: &[&str] = &[
    "The quick brown fox jumps over 12,345 lazy dogs. Dr. Smith said: \"Hello!\"",
    "Привет, мир! Это тест 3.14 и 1st.",
    "你好，世界。今天是2024年5月6日。",
    "مرحبا بالعالم ١٢٣",
    "नमस्ते दुनिया १२३",
    "<speak>Hello <break time=\"1s\"/><say-as interpret-as=\"cardinal\">123</say-as></speak>",
    "e.g. U.S.A. 5% $5 1/2 x² 10:30 a.m. 3rd 1,000.50 -5 [[h@l'oU]] \u{1}\u{2}",
];

fn random_text(rng: &mut Lcg) -> String {
    let base: Vec<char> = SENTENCES[rng.below(SENTENCES.len())].chars().collect();
    let mut out = base;
    for _ in 0..1 + rng.below(6) {
        let at = rng.below(out.len() + 1);
        match rng.below(3) {
            0 if !out.is_empty() => {
                out.remove(at.min(out.len() - 1));
            }
            1 => out.insert(at, PUNCT[rng.below(PUNCT.len())]),
            _ => out.insert(at, char::from_u32(0xa0 + rng.below(0x2f00) as u32).unwrap_or('?')),
        }
    }
    out.into_iter().collect()
}

fn all_languages() -> Vec<String> {
    let dir = std::path::Path::new("espeak-ng-data");
    let mut names: Vec<String> = espeak_ng::voices::list_voices(dir)
        .into_iter()
        .map(|v| v.language)
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn random_text_never_panics_in_any_language() {
    let languages = all_languages();
    if languages.is_empty() {
        eprintln!("[SKIP] no data");
        return;
    }
    let mut rng = Lcg(0xC0FFEE);
    for lang in &languages {
        let Ok(mut t) = Translator::new(lang, None) else { continue };
        for round in 0..3 {
            let text = random_text(&mut rng);
            t.options.punct = match round % 3 {
                0 => None,
                1 => Some(Vec::new()),
                _ => Some(vec!['.', ',', '!']),
            };
            let _ = t.text_to_ipa(&text);
            let _ = t.text_to_ipa_ssml(&text);
            let _ = t.translate_to_codes(&text);
        }
    }
}

#[test]
fn undecodable_bytes_never_panic_in_any_encoding() {
    use espeak_ng::encoding::{DecodeMode, Encoding, TextDecoder};
    let mut rng = Lcg(0xBADF00D);
    let bytes: Vec<u8> = (0..4096).map(|_| rng.next() as u8).collect();
    for name in ["UTF-8", "ISO-8859-1", "ISO-8859-2", "KOI8-R", "ISCII", "UCS-2", "ASCII"] {
        for mode in [DecodeMode::Strict, DecodeMode::Auto] {
            let Ok(mut d) = TextDecoder::new(&bytes, Encoding::from_name(name), mode) else {
                continue;
            };
            while d.next_codepoint().is_some() {}
        }
    }
}

#[test]
fn synthesis_survives_random_text() {
    let Some(_) = works("en") else { return };
    let mut rng = Lcg(0x5EED5);
    for lang in ["en", "de", "ru", "cmn", "ja", "ar", "hi"] {
        let Ok(mut engine) = espeak_ng::engine::EspeakNg::new(lang) else { continue };
        for markup in [false, true] {
            engine.set_markup(markup);
            for punct in [None, Some(Vec::new())] {
                engine.set_punctuation_list(punct);
                let _ = engine.synth(&random_text(&mut rng));
            }
        }
    }
}
