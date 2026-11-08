// C ends each digit-group word that has non-zero groups after it with a pause, so voicing does not carry from "тысяч" into "девятьсот". Expected strings come from C espeak-ng master (f0fb11a).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new(lang, None).ok()?;
    Some(t.text_to_ipa(text).unwrap())
}

macro_rules! need {
    ($lang:expr, $text:expr) => {
        match ipa($lang, $text) {
            Some(s) => s,
            None => {
                eprintln!("[SKIP] no {} data", $lang);
                return;
            }
        }
    };
}

#[test]
fn russian_thousand_stays_voiceless_before_voiced_hundreds() {
    assert_eq!(
        need!("ru", "99999"),
        "dʲivʲɪnˈostɔdʲˈevɪtʲ tˈysʲitʃʲ dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ"
    );
}

#[test]
fn russian_thirteen_nines_keep_final_f_and_voiceless_tysyach() {
    assert_eq!(
        need!("ru", "9999999999999"),
        "dʲˈevɪtʲ trʲˌɪɭɪˈonʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ mʲˌɪɭɪˈjardʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ mʲˌɪɭɪˈonʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ tˈysʲitʃʲ dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ"
    );
}

#[test]
fn russian_sixteen_nines() {
    assert_eq!(
        need!("ru", "9999999999999999"),
        "dʲˈevɪtʲ kvʌdrʲiɭʲiˈonʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ trʲˌɪɭɪˈonʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ mʲˌɪɭɪˈjardʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ mʲˌɪɭɪˈonʌf dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ tˈysʲitʃʲ dʲɪvʲitsˈot dʲivʲɪnˈostɔdʲˈevɪtʲ"
    );
}

#[test]
fn polish_thirteen_digit_sequence() {
    assert_eq!(
        need!("pl", "3074185296307"),
        "tʃˈɨ bʲiljˈɔnɨ ɕˌɛdɛmdʑˈɛɕɔnttʃtˈɛrɨ mʲiljˈardɨ stˈɔ ɔɕɛmdʑˈɛɕɔntpʲˈɛɲtɕ mʲiljˈɔnuf dvʲˈɛɕcʲɛ dʑˌɛvʲɛɲdʑˈɛɕɔntʃˈɛɕtɕ tɨɕˈɛntsɨ tʃˈɨsta ɕˈɛdɛm"
    );
}

#[test]
fn polish_thirteen_nines() {
    assert_eq!(
        need!("pl", "9999999999999"),
        "dʑˈɛvʲɛɲdʑ bʲiljˈɔnuf dʑˈɛvʲɛɲtɕsˌɛd dʑɛvʲɛɲdʑˈɛɕɔnddʑˈɛvʲɛɲtɕ mʲiljˈarduf dʑˈɛvʲɛɲtɕsˌɛd dʑɛvʲɛɲdʑˈɛɕɔnddʑˈɛvʲɛɲtɕ mʲiljˈɔnuf dʑˈɛvʲɛɲtɕsˌɛd dʑɛvʲɛɲdʑˈɛɕɔnddʑˈɛvʲɛɲtɕ tɨɕˈɛntsɨ dʑˈɛvʲɛɲtɕsˌɛd dʑɛvʲɛɲdʑˈɛɕɔnddʑˈɛvʲɛɲtɕ"
    );
}

#[test]
fn czech_five_nines() {
    assert_eq!(
        need!("cs", "99999"),
        "dˈevadˌesaːddˈevjet cˈisiːts dˈevjetsˈed dˈevadˌesaːddˈevjet"
    );
}

#[test]
fn czech_thirteen_digit_sequence() {
    assert_eq!(
        need!("cs", "3074185296307"),
        "tr̝̊i bˈiliˌoːni sˈedumdˌesaːttʃtˈir̝i mˈiliard stˈo ˈosumdˌesaːtpjˈet mˈiliˌoːnuː dvjˈescˈe dˈevadˌesaːtʃˈest cˈisiːts tr̝̊ˈistˈa sˈedum"
    );
}

#[test]
fn english_thirteen_digit_sequence_is_unchanged() {
    assert_eq!(
        need!("en", "3074185296307"),
        "θɹˈiː tɹˈɪliən sˈɛvənti fˈɔː bˈɪliən wˈɒnhˈʌndɹɪdən ˈeɪti fˈaɪv mˈɪliən tˈuːhˈʌndɹɪdən nˈaɪnti sˈɪks θˈaʊzənd θɹˈiːhˈʌndɹɪdən sˈɛvən"
    );
}

#[test]
fn german_thirteen_digit_sequence_is_unchanged() {
    assert_eq!(
        need!("de", "3074185296307"),
        "dɾˈaɪ bɪljˈoːnən fˈiːɾ ʊntzˈiːptsɪç mɪljˈaɾdən ˈaɪnhˈʊndɜt fˈʏnf ʊntˈaxtsɪç mɪljˈoːnən tsvˈaɪhˈʊndɜt zˈɛks ʊntnˈɔøntsɪç tˈaʊzənt dɾˈaɪhˈʊndɜt zˈiːbən"
    );
}
