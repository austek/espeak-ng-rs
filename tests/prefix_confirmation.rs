// tests/prefix_confirmation.rs
//
// A word whose rules match a standard prefix keeps it only if the prefix is
// still recognised once the word's own suffix is removed (C `confirm_prefix`
// with `FLAG_NO_PREFIX`).  German `umgehen` loses the `unge-` style prefix
// match when `-en` is cut, and the prefix found on `umgeh` (`um-`) is the one
// removed.  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn prefix_confirmation_matches_c() {
    let cases = [
        ("de", "umgehen", "ʊmɡˈeːˌən"),
        ("de", "unseres", "ˈʊnzərəs"),
        ("de", "unserer", "ˈʊnzərɜ"),
        ("de", "ungehen", "ˈʊnɡˌeːən"),
        ("de", "umgehe", "ˈʊmɡˌeːə"),
        ("de", "unsere", "ˈʊnzrə"),
        ("de", "ungeheuer", "ˈʊnɡəhˌɔøɜ"),
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
