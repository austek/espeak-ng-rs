// A word that has a standard prefix removed keeps the stress flag of its list
// entry (`invasie $2`), and the stem is stressed before the prefix is put back.
// Expected strings are from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn prefixed_words_follow_their_list_stress() {
    let Some(t) = Translator::new("nl", None).ok() else {
        eprintln!("[SKIP] no nl data");
        return;
    };
    let cases = [
        ("invasie", "ɪnvˈaːzˌi"),
        ("gefrituurd", "ɣəfritˈyrt"),
        ("wanschapen", "ʋɑnsxˈaːpˌən"),
        ("werktuigelijk", "ʋɛrktˈœyɣˌələk"),
        ("ingeval", "ˌɪnɣəvˈɑl"),
        ("boeket", "bukˈɛt"),
        ("wanluidend", "ʋɑnlˈœydˌənt"),
        ("ingevolge", "ˌɪnɣəvˈɔlɣˌə"),
        ("kredietwaardig", "krəditʋˈaːrdˌəx"),
        ("ingewikkeld", "ˌɪnɣəʋˈɪkˌəlt"),
        ("bijziend", "bɛɪzˈint"),
        ("ingrijpend", "ɪnɣrˈɛɪpˌənt"),
        ("inductie", "ɪndˈɵksˌi"),
        ("sportief", "spɔrtˈif"),
        ("sportieve", "spɔrtˈivˌə"),
        ("tegennatuur", "tˌeːɣənnaːtˈyr"),
        ("wantrouwend", "ʋɑntrˈʌʊˌənt"),
        ("onze", "ɔnzˈə"),
        ("wester", "ʋɛstˈər"),
        ("vergeten", "vərɣˈeːtən"),
        ("ontvangen", "ɔntvˈɑŋən"),
        ("onderzoek", "ˈɔndərzˌuk"),
        ("overal", "ˌoːvərˈɑl"),
        ("uitgang", "ˈœytɣˌɑŋ"),
        ("beantwoorden", "bəˈɑntʋɔːrdən"),
        ("boekenkast", "bˈukˌɛŋkɑst"),
        ("ingewikkelder", "ˈɪnɣəʋˌɪkɛldər"),
    ];
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(got, want, "{word}");
    }
}
