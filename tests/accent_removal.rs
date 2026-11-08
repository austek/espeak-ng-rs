// tests/accent_removal.rs
//
// An accented letter inside a word that the language's rules do not cover is
// replaced by its unaccented base letter (C's `remove_accent` table) and the
// whole word is translated again from the start.  The port used to skip such a
// letter, dropping it from the word (`Cañon` lost its `ñ`).  Expected output is
// from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn unknown_accented_letters_fold_to_their_base() {
    let cases = [
        ("de", "Cañon", "kˈɑnoːn"),
        ("de", "Zoë", "tsˈoːə"),
        ("de", "Brontë", "bɾˈɔntə"),
        ("de", "Håkan", "hakˈɑːn"),
        ("fr", "aîné", "ɛnˈe"),
        ("fr", "Ångström", "ɑ̃ɡstʁˈom"),
        ("fr", "Škoda", "skodˈa"),
        ("es", "Zoë", "θˈoe"),
        ("es", "naïve", "nˈaɪβe"),
        ("pt", "Sørensen", "sˌuɾˈeɪŋseɪŋ"),
        ("it", "Müller", "mʊllˈɛr"),
        ("it", "Łódź", "lˈodts"),
        ("pl", "Citroën", "tɕitrˈɔɛn"),
        ("tr", "Café", "dʒafˈɛ"),
        ("nl", "Åre", "ˈɑrə"),
        ("nl", "vóór", "vˈɔːr"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap();
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
