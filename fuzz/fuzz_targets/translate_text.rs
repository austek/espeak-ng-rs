#![no_main]

use std::sync::OnceLock;

use espeak_ng::Translator;
use libfuzzer_sys::fuzz_target;

const LANGS: &[&str] = &[
    "en", "de", "fr", "es", "it", "ru", "ar", "cmn", "yue", "ja", "hi", "ko", "vi", "tr", "om", "ku",
];

const PUNCT_MODES: usize = 3;

static TRANSLATORS: [OnceLock<Option<Translator>>; LANGS.len() * PUNCT_MODES] =
    [const { OnceLock::new() }; LANGS.len() * PUNCT_MODES];

// Loading a language's dictionaries costs far more than translating one input,
// so each language and punctuation mode is built once and reused.
fn translator(lang: usize, mode: usize) -> Option<&'static Translator> {
    TRANSLATORS[lang * PUNCT_MODES + mode]
        .get_or_init(|| {
            let mut translator = Translator::new(LANGS[lang], None).ok()?;
            translator.options.punct = match mode {
                0 => None,
                1 => Some(Vec::new()),
                _ => Some(vec!['.', ',', '!']),
            };
            Some(translator)
        })
        .as_ref()
}

// The first byte picks the language and the punctuation announcement; the rest
// is the text. Any input must come back as Ok or Err, never a panic.
fuzz_target!(|data: &[u8]| {
    let Some((&head, rest)) = data.split_first() else { return };
    let mode = usize::from(head >> 6).min(PUNCT_MODES - 1);
    let Some(translator) = translator(head as usize % LANGS.len(), mode) else { return };
    let text = String::from_utf8_lossy(rest);
    let _ = translator.text_to_ipa(&text);
});
