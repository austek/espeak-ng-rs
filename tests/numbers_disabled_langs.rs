// tests/numbers_disabled_langs.rs
//
// C's `SetTranslator` sets `langopts.numbers = 0` for mi, my, piqd, py, qu, quc,
// th and uz ("disable numbers until the definition are complete in _list
// file"), so `TranslateNumber` declines and every digit is looked up as `_<d>`
// on its own, a pause after each second one.  Expected values come from the C
// espeak-ng master binary (f0fb11a8).

use espeak_ng::Translator;

fn check(lang: &str, rows: &[(&str, &str)]) {
    let Ok(t) = Translator::new_default(lang) else {
        eprintln!("[SKIP] no {lang} dict");
        return;
    };
    for (text, ipa) in rows {
        assert_eq!(t.text_to_ipa(text).unwrap(), *ipa, "{lang} {text:?}");
    }
}

#[test]
fn quechua_reads_digits_one_by_one() {
    check("qu", &[
        ("7", "kˈantʃis"),
        ("42", "tˌawaˈiskaj"),
        ("100", "hˌuktʃʔusaktʃʔˈusak"),
        ("2024", "ˌiskajtʃʔˌusakˌiskajtˈawa"),
        ("1234567", "hˈuk ˌiskajkˌimsatˈawa pˌitʃkasˌuktakˈantʃis"),
        ("3.14", "kˌimsahuktˈawa"),
        ("007", "tʃʔˌusaktʃʔˌusakkˈantʃis"),
    ]);
}

#[test]
fn maori_reads_digits_one_by_one() {
    check("mi", &[("42", "faːrˈua"), ("2024", "rˌuakˌɔreruˈafaː")]);
}

#[test]
fn burmese_thai_uzbek_read_digits_one_by_one() {
    check("my", &[("42", "lˈiːnˈiː"), ("100", "tˈiːtonjˈaːtonjˈaː")]);
    check("uz", &[("42", "tˈoɾtˈykky"), ("100", "bˈyɾnˈoːɫnˈoːɫ")]);
}

#[test]
fn pidgin_and_kiche_read_digits_one_by_one() {
    check("py", &[("45", "ksˈashfak"), ("100", "hjˈikzronzrˌon")]);
    check("quc", &[("42", "kˌajibkˈeb"), ("2024", "kebkˈebkˈajib")]);
    check("piqd", &[("42", "lˈoʃt͡ʃaʔ"), ("100", "waʔpˈar̩pˈar̩")]);
}
