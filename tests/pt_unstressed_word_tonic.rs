// A clause with no stressed syllable gets its tonic on the last syllable at the
// intonation stage, *after* the phoneme programs ran.  The vowel keeps the
// reduced quality those programs chose for an unstressed syllable: `ser` is
// `sˈɨɹ` (not `sˈeɹ`), `sou` `sˈuw`, `estava` `ʃtˈɐvɐ`.
//
// Values from a locally built upstream `espeak-ng` (`f0fb11a8`).
// Skips when the pt voice data is absent.

use espeak_ng::Translator;

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Ok(t) = Translator::new(lang, None) else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    for (text, want) in cases {
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{lang} {text:?}");
    }
}

#[test]
fn unstressed_words_keep_reduced_vowels_under_the_tonic() {
    check(
        "pt",
        &[
            ("ser", "sˈɨɹ"),
            ("ter", "tˈɨɹ"),
            ("sou", "sˈuw"),
            ("estou", "ɨʃtˈuw"),
            ("por", "pˈuɹ"),
            ("somos", "sˈumʊʃ"),
            ("fomos", "fˈumʊʃ"),
            ("estava", "ʃtˈɐvɐ"),
            ("estávamos", "ʃtˈɐvɐmʊʃ"),
            ("aos", "ˈɐʊʃ"),
            ("às", "ˈɐɐʃ"),
            ("estar", "ɨʃtˈɐɹ"),
            ("estará", "ʃtɐɾˈɐ"),
            ("terá", "tɨɾˈɐ"),
            ("terás", "tɨɾˈɐʃ"),
            ("fôreis", "fˈuɾeɪʃ"),
            ("prà", "pɹˈɐ"),
            ("pràs", "pɹˈɐʃ"),
            ("prò", "pɹˈu"),
            ("pròs", "pɹˈuʃ"),
            ("estavam", "ʃtˈɐvɐ\u{303}ʊ\u{303}"),
        ],
    );
}

#[test]
fn unstressed_words_in_phrases() {
    check(
        "pt",
        &[
            ("eu sou", "eʊ sˈuw"),
            ("vou ser", "vuw sˈɨɹ"),
            ("nós somos", "nɔʃ sˈumʊʃ"),
            ("vamos estar", "vˌɐ\u{303}mʊz ɨʃtˈɐɹ"),
        ],
    );
}
