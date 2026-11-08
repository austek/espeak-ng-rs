// Dutch skips the leading "s" when it counts the consonants before the first
// vowel (LOPT_UNPRONOUNCABLE = 's'), so "schr..." and "schn..." are words and
// are not spelled out.  Expected strings are from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn schr_and_schn_words_are_not_spelled() {
    let Some(t) = Translator::new("nl", None).ok() else {
        eprintln!("[SKIP] no nl data");
        return;
    };
    let cases = [
        ("schrander", "sxrˈɑndər"),
        ("schnabbel", "ʃnˈɑbəl"),
        ("schrikkel", "sxrˈɪkəl"),
        ("schreeuwer", "sxrˈeʊər"),
        ("schrompel", "sxrˈɔmpəl"),
        ("schroeier", "sxrˈujər"),
        ("schrijver", "sxrˈɛɪvər"),
        ("schnell", "ʃnˈɛl"),
        ("school", "sxˈoːl"),
        ("stra", "strˈaː"),
        ("spr", "ˌɛspˌeːˈɛr"),
    ];
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(got, want, "{word}");
    }
}
