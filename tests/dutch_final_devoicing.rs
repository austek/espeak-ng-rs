// Word-final devoicing (LOPT_REGRESSIVE_VOICING 0x100) also devoices the last
// word of a clause that has no closing punctuation, and it leaves the phonemes
// of a word read in another language alone.  Expected strings are from C
// espeak-ng master.

use espeak_ng::Translator;

#[test]
fn clause_final_consonants_devoice_without_punctuation() {
    let Some(t) = Translator::new("nl", None).ok() else {
        eprintln!("[SKIP] no nl data");
        return;
    };
    let cases = [
        ("hebt", "hˈɛpt"),
        ("lobt", "lˈɔpt"),
        ("abc", "ˈɑpk"),
        ("heb", "hˈɛp"),
        ("klaagt", "klˈaːxt"),
        ("dubbel", "dˈɵbəl"),
        ("handen", "hˈɑndən"),
        ("wegen", "ʋˈeːɣən"),
    ];
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(got, want, "{word}");
    }
}

#[test]
fn borrowed_english_words_keep_their_voiced_ending() {
    let Some(t) = Translator::new("nl", None).ok() else {
        eprintln!("[SKIP] no nl data");
        return;
    };
    let cases = [
        ("items", "(en)ˈaɪtəmz(nl)"),
        ("airlines", "(en)ˈeəlaɪnz(nl)"),
        ("website", "(en)wˈɛbsaɪt(nl)"),
        ("websites", "(en)wˈɛbsaɪts(nl)"),
        ("homepage", "(en)hˈəʊmpeɪdʒ(nl)"),
        ("enterprise", "(en)ˈɛntəpɹˌaɪz(nl)"),
        ("android", "(en)ˈandɹɔɪd(nl)"),
        ("lobt homepage", "lˈɔpt (en)hˈəʊmpeɪdʒ(nl)"),
    ];
    for (text, want) in cases {
        let got = t.text_to_ipa(text).unwrap_or_default();
        assert_eq!(got, want, "{text}");
    }
}
