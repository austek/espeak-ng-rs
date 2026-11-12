#![no_main]

use espeak_ng::Translator;
use libfuzzer_sys::fuzz_target;

const LANGS: &[&str] = &[
    "en", "de", "fr", "es", "it", "ru", "ar", "cmn", "yue", "ja", "hi", "ko", "vi", "tr", "om", "ku",
];

// The first byte picks the language and the SSML/punctuation options; the rest
// is the text. Any input must come back as Ok or Err, never a panic.
fuzz_target!(|data: &[u8]| {
    let Some((&head, rest)) = data.split_first() else { return };
    let lang = LANGS[head as usize % LANGS.len()];
    let Ok(mut translator) = Translator::new(lang, None) else { return };
    translator.options.punct = match head >> 6 {
        0 => None,
        1 => Some(Vec::new()),
        _ => Some(vec!['.', ',', '!']),
    };
    let text = String::from_utf8_lossy(rest);
    let _ = translator.text_to_ipa(&text);
    let _ = translator.text_to_ipa_ssml(&text);
});
