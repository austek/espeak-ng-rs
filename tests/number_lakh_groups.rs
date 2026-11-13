// Languages with BREAK_LAKH* group digits as 10,00,000 and name each group by
// its position (lakh, crore). Expected values come from C espeak-ng f0fb11a8.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    Translator::new_default(lang).ok().map(|t| t.text_to_ipa(text).unwrap())
}

#[test]
fn hindi_lakh_and_crore() {
    let Some(i) = ipa("hi", "100000") else { return };
    assert_eq!(i, "ˈeːk lˈaːkʰ");
    assert_eq!(ipa("hi", "1000000").unwrap(), "dˈʌs lˈaːkʰ");
    assert_eq!(
        ipa("hi", "1234567").unwrap(),
        "bˈaːɾəh lˈaːkʰ cˈɔ̃tis hˈʌɟaːɾ pˈa\u{303}csˈɔː sˈʌr.səʈʰ"
    );
    assert_eq!(
        ipa("hi", "12345678").unwrap(),
        "ˈeːk kəɾˈoːr. tˈeːis lˈaːkʰ paɪntˈaːlis hˈʌɟaːɾ cʰˈʌhsˈɔː ʌthˈʌtːəɾ"
    );
}

#[test]
fn bengali_lakh_and_crore() {
    let Some(i) = ipa("bn", "1000000") else { return };
    assert_eq!(i, "dˈɔʃ lˈakʰ");
    assert_eq!(
        ipa("bn", "12345678").unwrap(),
        "ˈæk kˈoʈi tˈejʃ lˈakʰ pˈɔ̃jtallˈiʃ hˈajaɾ tʃʰˈɔʃˈo ˈaʈattˈɔɾ"
    );
}

#[test]
fn nepali_lakh() {
    let Some(i) = ipa("ne", "1000000") else { return };
    assert_eq!(i, "dˈʌs lˈaːkʰə");
    assert_eq!(ipa("ne", "100000").unwrap(), "eːk lˈaːkʰə");
}
