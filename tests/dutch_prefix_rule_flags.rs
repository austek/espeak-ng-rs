// After a prefix is removed, the stem's rules see the flags of the whole word
// (`gestage $alt $2` selects the `age (_$w_alt` rule).  Expected strings are
// from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn stem_rules_see_the_flags_of_the_whole_word() {
    let Some(t) = Translator::new("nl", None).ok() else {
        eprintln!("[SKIP] no nl data");
        return;
    };
    let cases = [
        ("gestage", "ɣəstˈaːɣˌə"),
        ("inzage", "ˈɪnzˌaːɣə"),
        ("stage", "stˈaːʒə"),
        ("garage", "ɣaːrˈaːʒə"),
        ("beter", "bˈeːtər"),
        ("herover", "hɛrˈoːvər"),
        ("gepeupel", "ɣəpøːpˈɛl"),
        ("inkader", "ˈɪnkˌaːdər"),
    ];
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(got, want, "{word}");
    }
}
