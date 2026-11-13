// The time-colon rules of de, fr and it (`_L05_) : (_L06D_`): the colon is a
// connector only after a whole-word hour 0-9 or 00-23 and before a minute
// 00-59. Expected strings are from C espeak-ng master f0fb11a8.

use espeak_ng::Translator;

const CASES: &[(&str, &str, &str)] = &[
    ("de", "12:61", "tsvˈœlf ˈaɪn ʊntzˈɛçtsɪç"),
    ("de", "12:60", "tsvˈœlf zˈɛçtsɪç"),
    ("de", "20:60", "tsvˈantsɪç zˈɛçtsɪç"),
    ("de", "005:30", "nˈʊlnˈʊl fˈʏnf dɾˈaɪsɪç"),
    ("de", "0005:30", "nˈʊlnˈʊlnˈʊlfˈʏnf dɾˈaɪsɪç"),
    ("de", "012:30", "nˈʊl tsvˈœlf dɾˈaɪsɪç"),
    ("de", "000:30", "nˈʊlnˈʊl nˈʊl dɾˈaɪsɪç"),
    ("de", "12 :30", "tsvˈœlf uːɾ dɾˈaɪsɪç"),
    ("de", "3 :45", "dɾˈaɪ uːɾ fˈʏnf ʊntfˈɪɾtsɪç"),
    ("de", "12:::30", "tsvˈœlf uːɾ dɾˈaɪsɪç"),
    ("de", "12:30.5", "tsvˈœlf dɾˈaɪsɪç pˌʊŋkt fˈʏnf"),
    ("de", "18:15:30", "ˈaxtseːn uːɾ fˈʏnftseːn uːɾ dɾˈaɪsɪç"),
    ("de", "12:61:30", "tsvˈœlf ˈaɪn ʊntzˈɛçtsɪç dɾˈaɪsɪç"),
    ("fr", "12:61", "dˈuz swasɑ̃teˈœ̃"),
    ("fr", "005:30", "zeʁozeʁo sˈɛ̃k tʁˈɑ̃t"),
    ("fr", "12 :30", "dˈuz ˈœʁ tʁˈɑ̃t"),
    ("fr", "18:15:30", "dizyˈit ˈœʁ kˈɛ̃z ˈœʁ tʁˈɑ̃t"),
    ("it", "12:61", "dˈoditʃɪ sessˈantˈuno"),
    ("it", "005:30", "dzˈɛɾodzˈɛɾo tʃˈinkwe trˈeːnta"),
    ("it", "12 :30", "dˈoditʃɪ e trˈeːnta"),
    ("it", "18:15:30", "ditʃˈɔtːo e kwˈimditʃɪ e trˈeːnta"),
    ("en", "005:30", "zˈiəɹəʊzˈiəɹəʊ fˈaɪv θˈɜːti"),
];

#[test]
fn time_colon_follows_the_c_rule_context() {
    for &(lang, text, want) in CASES {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(t.text_to_ipa(text).unwrap_or_default().trim_end(), want, "{lang} {text}");
    }
}
