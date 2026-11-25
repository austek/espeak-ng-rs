#![no_main]

use std::sync::OnceLock;

use espeak_ng::translate::ssml_driver::SsmlSettings;
use espeak_ng::Translator;
use libfuzzer_sys::fuzz_target;

const LANGS: &[&str] = &["en", "de", "fr", "es", "ru", "ar", "cmn", "ja", "hi"];

static TRANSLATORS: [OnceLock<Option<Translator>>; LANGS.len()] = [const { OnceLock::new() }; LANGS.len()];

// Loading a language's dictionaries costs far more than parsing one input,
// so each translator is built once and reused.
fn translator(index: usize) -> Option<&'static Translator> {
    TRANSLATORS[index].get_or_init(|| Translator::new(LANGS[index], None).ok()).as_ref()
}

// The SSML parser and clause reader must turn any markup into a document or an
// error, and a document must render to IPA lines. The settings come from the input so rate and break scaling are fuzzed.
fuzz_target!(|data: &[u8]| {
    let [lang, rate, brk, rest @ ..] = data else { return };
    let Some(translator) = translator(*lang as usize % LANGS.len()) else { return };
    let settings = SsmlSettings {
        rate_wpm: 80 + i32::from(*rate),
        break_mul: i32::from(*brk) * 2,
        base_variant: String::new(),
    };
    let text = String::from_utf8_lossy(rest);
    if let Ok(document) = translator.translate_ssml(&text, settings) {
        let _ = document.ipa_lines(false);
    }
});
