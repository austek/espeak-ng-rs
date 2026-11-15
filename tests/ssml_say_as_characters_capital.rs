// tests/ssml_say_as_characters_capital.rs
//
// `<say-as interpret-as="characters" format="glyphs">` announces a capital letter in the
// language that spells it (C `TranslateLetter`, translateword.c:952 and :1147): the voice that
// names the letter, after the name when `langopts.accents & 2`.  A character the voice hands
// to another language (`_^_en`) abandons the spelled word, and every character is read as a
// word of its own in that language (translate.c:1784).  `option_sayas & 0x10` keeps a letter
// with no rule from being handed over by script.  Expected values are what C espeak-ng
// master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 11] = [
    ("he", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">AB</say-as></speak>", "(en)kˌapətəlˈeɪ kˌapətəlbˈiː(he)", "(en)k,ap@-t@L'eI_! k,ap@-t@Lb'i:_!(he)"),
    ("he", "<speak><say-as interpret-as=\"characters\">abc</say-as></speak>", "(en)ˈeɪ bˈiː sˈiː(he)", "(en)'eI_! b'i:_! s'i:_!(he)"),
    ("he", "<speak><say-as interpret-as=\"tts:char\">A</say-as></speak>", "(en)kˌapətəlˈeɪ(he)", "(en)k,ap@-t@L'eI_!(he)"),
    ("zh", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">AB</say-as></speak>", "(en)kˌa5pəətələlˈeɪ5 kˌa5pəətələlbˈiː5(cmn)", "(en)k,a55p@-55t@L55'eI55_!_| k,a55p@-55t@L55b'i:55_!(cmn)_|"),
    ("fa", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">aB</say-as></speak>", "(en)ˈeɪ kˌapətəlbˈiː(fa)", "(en)'eI_! k,ap@-t@Lb'i:_!(fa)"),
    ("th", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">AB</say-as></speak>", "(en)kˌapətəlˈeɪ kˌapətəlbˈiː(th)", "(en)k,ap@-t@L'eI_! k,ap@-t@Lb'i:_!(th)"),
    ("ru", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">AB</say-as></speak>", "(en)ˌeɪ(ru)(en)bˈiː(ru)", "_:(en),eI(ru)_!_:(en)b'i:(ru)_!"),
    ("ru", "<speak><say-as interpret-as=\"characters\">ab</say-as></speak>", "(en)ˌeɪ(ru)(en)bˈiː(ru)", "_:(en),eI(ru)_|_:(en)b'i:(ru)_!"),
    ("ar", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">AB</say-as></speak>", "(en)ˌeɪ(ar)(en)bˈiː(ar)", "_:(en),eI(ar)_!_:(en)b'i:(ar)_!"),
    ("fr", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">AB</say-as></speak>", "ˌamaʒyskˌylbˌemaʒyskˈyl", ",amaZysk,yl_!b,emaZysk'yl_!"),
    ("it", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">Ab</say-as></speak>", "ˌaːmajˌʊskolabˈi", ",a:maj,Uskola_!b'i_!"),
];

#[test]
fn spelled_capital_matches_c() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
