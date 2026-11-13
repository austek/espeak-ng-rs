// Cherokee capitals lower-case into U+AB70.., which C's alphabet table gives to
// Korean. The Korean translator cannot read them either, so the word is spelled
// letter by letter and, with no letter names, speaks nothing. Expected values
// come from the C espeak-ng master binary (`--ipa` and `-x`).

use espeak_ng::Translator;

fn silent(lang: &str, text: &str) {
    let Some(t) = Translator::new(lang, None).ok() else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    assert_eq!(t.text_to_ipa(text).unwrap().trim(), "", "{lang} {text}");
}

#[test]
fn spelled_letters_stay_in_the_original_voice() {
    let Some(t) = Translator::new("kk", None).ok() else {
        eprintln!("[SKIP] no kk data");
        return;
    };
    assert_eq!(t.text_to_ipa("ᎣᏏᏲ").unwrap().trim(), "lˈetəbˈɪrˈuʃˈefˈeːj");
    assert_eq!(t.text_to_ipa("ᏏᏴᏫ").unwrap().trim(), "lˈetəbˈɪrˈuʃˈefsˈiː");
}

#[test]
fn cherokee_word_is_silent_in_cherokee_voice() {
    silent("chr", "ᎠᏚ");
    silent("chr", "ᎠᏚᎢᏍᏘᎠᎵᏚᎶ");
}

#[test]
fn cherokee_word_is_silent_in_other_voices() {
    silent("ko", "ꭰꭱ");
    silent("en", "ᎠᏚ");
    silent("de", "ꭰꭱ ꭰꭱ");
}
