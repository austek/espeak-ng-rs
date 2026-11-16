// tests/dotted_abbrev_foreign_letters.rs
//
// A dotted run is handed to English as a whole only when the voice's rules hand its first
// letter over (translateword.c:316).  Otherwise CheckDottedAbbrev joins it and
// SpeakIndividualLetters spells it with TranslateLetter, which speaks each letter the voice
// cannot name in English, between a switch and a switch back, with the spelling stress of one
// word.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 10] = [
    ("de", "Hallo x .-Ж Welt", "hˈaloː ˌɪkskyːrˌɪlɪʃ(en)ʒˈɛː(de) vˈɛlt", "h'alo: _!,Iksky:R,IlIS_:(en)Z'E:(de) v'Elt"),
    ("ky", "Салам a .-b дүйнө", "sɑlˈɑm (en)ˌeɪbˈiː(ky) d[yjnˈoe", "sal'am _:(en),eI_:b'i:(ky) dyjn'O"),
    ("ky", "Салам a . b дүйнө", "sɑlˈɑm (en)ˌeɪbˈiː(ky) d[yjnˈoe", "sal'am _:(en),eI_:b'i:(ky) dyjn'O"),
    ("ba", "a .-b", "(en)ˌeɪbˈiː(ba)", "_:(en),eI_:b'i:(ba)"),
    ("cv", "I .-a", "(en)ˌaɪˈeɪ(cv)", "_:(en),aI_:'eI(cv)"),
    ("mi", "Q .-W .-E", "(en)kjˌuː(mi)wˈe", "_:(en)kj,u:(mi)w'e_|"),
    ("ja", "A.B.C.", "(en)ˌeɪbˌiːsˈiː(ja)", "_:(en),eI_:b,i:_:s'i:(ja)"),
    ("ja", "R.I.P.", "(en)ˌɑːˌaɪpˈiː(ja)", "_:(en),A@_:,aI_:p'i:(ja)"),
    ("ky", "A.B.C.", "(en)ˌeɪbˌiːsˈiː(ky)", "_:(en),eI_:b,i:_:s'i:(ky)"),
    ("mi", "A.B.C.", "ˌa(en)bˌiːsˈiː(mi)", ",a_:(en)b,i:_:s'i:(mi)_|"),
];

#[test]
fn dotted_run_in_a_voice_that_keeps_it_is_spelled_with_foreign_letters() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
