// tests/ordinal_dot_context.rs
//
// A dot after a number marks an ordinal only when a lower-case word follows
// (C `CheckDotOrdinal`).  German nouns are capitalised, so "am 3. Mai" reads
// the cardinal "drei" and only "am 3. mai" reads "dritte"; a number followed by
// another number or by punctuation stays a cardinal.  The port used to turn
// every "N." into an ordinal.  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn dot_ordinal_needs_a_lowercase_word_after_it() {
    let cases = [
        ("de", "am 3. Mai", "am dɾˈaɪ | mˈaɪ"),
        ("de", "3. Platz", "dɾˈaɪ | plˈats"),
        ("de", "am 21. Dezember 2020", "am ˈaɪn ʊnttsvˈantsɪç | deːtsˈɛmbɜ tsvˈaɪ tˈaʊzənt tsvˈantsɪç"),
        ("de", "3. 4", "dɾˈaɪ | fˈiːɾ"),
        ("de", "am 3. mai", "am dɾˈɪtə mˈaɪ"),
        ("de", "im 20. jahrhundert", "ɪm tsvˈantsɪçstə jɑːɾhˈʊndɜt"),
        ("de", "am 1. und 2. Mai", "am ˈeːɾstə ʊnt tsvˈaɪ | mˈaɪ"),
        ("nl", "3. Mei", "drˈi | mˈɛɪ"),
        ("nl", "de 3. mei", "də dˈɛrdə mˈɛɪ"),
        ("da", "den 3. Maj", "dɛn tʁˈʔe | mˈ?ɑj"),
        ("fi", "3. Toukokuuta", "kˈolme | tˈoukokˌuːta"),
        ("lv", "21. a", "dˈiʋdesmit pˈirmais ˈaː"),
        ("lv", "3. A", "trˈiːs | ˈaː"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap().replace('\n', " | ");
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
