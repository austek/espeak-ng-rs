// Thousands and decimal separators follow the language's `NUM_DECIMAL_COMMA` /
// `NUM_THOUS_SPACE` options, a typed four-digit group is spoken as two words, and
// the pieces of a decimal fraction run together without a word break.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

fn check(cases: &[(&str, &str, &str)]) {
    if ipa("en", "1").is_none() {
        return;
    }
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(*want), "{lang} {text}");
    }
}

#[test]
fn comma_groups_thousands_unless_the_language_decimal_is_a_comma() {
    check(&[
        ("ku", "1,234", "hɛzˈar udˈʊsˈɛdu sˈiutʃˈar"),
        ("ku", "1,000", "hɛzˈar"),
        ("ku", "12,345", "danzdˈɛ hɛzˈar usˈesˈɛdu tʃˈɪlupˈendʒ"),
        ("fa", "1,005", "jˌek hezˈɑro ˈo pandʒ"),
        ("fa", "1,234", "jˌek hezˈɑro dˌevistˌo siotʃˈɑhɑr"),
        ("cs", "1.234", "jˈeden tˌetʃka dvjˈescˈe tr̝̊ˈitsettʃtˈir̝i"),
    ]);
}

#[test]
fn typed_four_digit_group_is_two_words() {
    check(&[
        ("ar", "1,234", "ʔˈalf wˌamiʔatˈaːnwa ʔˈarbaʕʕˌawaθˌalaːθˈuːn"),
        ("en", "1,234", "wˈɒn θˈaʊzənd tˈuːhˈʌndɹɪdən θˈɜːti fˈɔː"),
    ]);
}

#[test]
fn fraction_pieces_run_together() {
    check(&[
        ("es", "0,05", "θˈeɾo kˌomaθˈeɾo θˈinko"),
        ("es", "1,005", "ˈuno kˌomaθˈeɾoθˈeɾo θˈinko"),
        ("fr", "0,05", "zeʁo viʁɡylzeʁo sˈɛ̃k"),
        ("fr", "1,2345", "œ̃ viʁɡyldø mil tʁwasɑ̃ kaʁɑ̃tsˈɛ̃k"),
        ("it", "1,000", "ˈuno vˈirɡola dzˈɛɾomillˈezimɪ"),
        ("pl", "0,05", "zˈɛrɔː pʃɛtɕˈinɛkzˈɛrɔː pʲˈɛɲtɕ"),
        ("cs", "0,05", "nˈula tʃˌaːrkanˈula pjˈet"),
    ]);
}
