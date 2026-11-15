// tests/ssml_say_as_characters_digit_groups.rs
//
// C `TranslateClause` cuts a word that starts with five to 32 digits into groups at
// `break_numbers`, whatever the say-as mode, writes the thousands separator after each group and
// pads a group followed by a shorter one with zeros.  In `characters` mode every character of
// each group is then spelled, so the separator is read by name.  Expected values are what C
// espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 16] = [
    ("en", "x <say-as interpret-as=\"characters\">12345</say-as> y", "ˈɛks wˌɒntˌuːkˈɒmə θɹˌiːfˌɔːfˈaɪv wˈaɪ", "'Eks_:_: w,0n_|t,u:_|k'0m@_!_:_: Tr,i:_|f,o@_|f'aIv_! w'aI"),
    ("en", "<say-as interpret-as=\"characters\">1234567</say-as>", "wˌɒnkˈɒmə tˌuːθɹˌiːfˌɔːkˈɒmə fˌaɪvsˌɪkssˈɛvən", "w,0n_|k'0m@_! t,u:_|Tr,i:_|f,o@_!k'0m@_! f,aIv_|s,Iks_|s'Ev@n_!"),
    ("en", "<say-as interpret-as=\"characters\">123456789012</say-as>", "wˌɒntˌuːθɹˌiːkˈɒmə fˌɔːfˌaɪvsˌɪkskˈɒmə sˌɛvənˌeɪtnˌaɪnkˈɒmə zˌiəɹəʊwˌɒntˈuː", "w,0n_|t,u:_|Tr,i:_!k'0m@_! f,o@_|f,aIv_|s,Iks_!k'0m@_! s,Ev@n_|,eIt_|n,aIn_!k'0m@_! z,i@roU_|w,0n_|t'u:_!"),
    ("en", "<say-as interpret-as=\"characters\">1234</say-as>", "wˌɒntˌuːθɹˌiːfˈɔː", "w,0n_|t,u:_|Tr,i:_!f'o@_!"),
    ("en", "<say-as interpret-as=\"characters\">012345</say-as>", "zˌiəɹəʊwˌɒntˌuːkˈɒmə θɹˌiːfˌɔːfˈaɪv", "z,i@roU_|w,0n_|t,u:_!k'0m@_! Tr,i:_|f,o@_|f'aIv_!"),
    ("en", "<say-as interpret-as=\"characters\">12345abc</say-as>", "wˌɒntˌuːkˈɒmə θɹˌiːfˌɔːfˈaɪvˌeɪbˌiːsˈiː", "w,0n_|t,u:_|k'0m@_! Tr,i:_|f,o@_|f'aIv_!,eI_|b,i:_|s'i:_!"),
    ("en", "<say-as interpret-as=\"characters\">ab 12345 cd</say-as>", "ˌeɪbˈiː wˌɒntˌuːkˈɒmə θɹˌiːfˌɔːfˈaɪv sˌiːdˈiː", ",eI_|b'i:_!_:_: w,0n_|t,u:_|k'0m@_!_:_: Tr,i:_|f,o@_|f'aIv_!_:_: s,i:_|d'i:_!"),
    ("de", "<say-as interpret-as=\"characters\">1234567</say-as>", "ˌaɪnspˈʊŋkt tsvˌaɪdɾˌaɪfˌiːɾpˈʊŋkt fˌʏnfzˌɛkszˈiːbən", "_,aIns_|p'UNkt_! tsv,aI_|dr,aI_|f,i:r_!p'UNkt_! f,ynf_|z,Eks_|z'i:b@n_!"),
    ("es", "<say-as interpret-as=\"characters\">12345</say-as>", "ˌunodˌospˈunto tɾˌeskwˌatɾoθˈinko", ",uno_|d,os_|p'unto_! t**,es_|kw,at**o_|T'inko_!"),
    ("fr", "<say-as interpret-as=\"characters\">1234567</say-as>", "ˌœ̃pwɛ̃ dˌøtʁwˌakˌatʁpwɛ̃ sˌɛ̃ksˌissˈɛt", ",W~_|_|pwE~_! d,Y_|trw,a_|k,atr_!_|pwE~_! s,E~k_|s,is_|s'Et_!"),
    ("it", "<say-as interpret-as=\"characters\">12345</say-as>", "ˌʊnodˌʊepˈunto trˌekwˌatːrotʃˈinkwe", ",Uno_|d,Ue_|p'unto_! t@-*,e_|kw,at:@-*o_|tS'inkwe_!"),
    ("nl", "<say-as interpret-as=\"characters\">1234567</say-as>", "ˌeːnpˈɵnt tʋˌeːdrˌivˌirpˈɵnt vˌɛɪfzˌɛszˈeːvən", "_!,e:n_|p'8nt_! tv#,e:_|dr,i_|v,ir_!p'8nt_! v,EIf_|z,Es_|z'e:v@n_!"),
    ("pt", "<say-as interpret-as=\"characters\">1234567</say-as>", "ˌumpˈoŋtʊ dˌoɪʃtɹˌeʃkwˌɐtɹupˈoŋtʊ sˌiŋkusˌeɪʃsˈɛtɨ", ",um_|p'oNtU_! d,oIs#_|tR,es#_|kw,&tru_!p'oNtU_! s,iNku_|s,eIs#_|s'Ety_!"),
    ("ru", "<say-as interpret-as=\"characters\">12345</say-as>", "ʌdʲˌindvˌɑtˈotʃʲka trʲˌitʃʲɪtˌyrʲɪpʲˈɑtʲ", "V#d;,in__|dv,A_|t'otS;ka#_! tR;,i_|tS;It,yR;I_|p;'At;_!"),
    ("ka", "<say-as interpret-as=\"characters\">12345</say-as>", "ˌertʰiˌorimdzˈime sˌamiˌotʰxixˈutʰi", ",ert#i_|,ori_|mdz'ime_! s,ami_|,ot#xi_|x'ut#i_!"),
    ("ku", "<say-as interpret-as=\"characters\">12345</say-as>", "jˌɛkdˌʊbehnˈok sˌetʃˌarpˈendʒ", "j,Ek_|d,U_|behn'ok_! s,e_|tS,ar_|p'endZ_!"),
];

#[test]
fn characters_mode_cuts_digit_runs_into_groups() {
    let mut failures = Vec::new();
    for (lang, text, ipa, x) in CASES {
        for (flag, want) in [("--ipa", ipa), ("-x", x)] {
            let got = run(lang, flag, text);
            if got != want {
                failures.push(format!("{lang} {flag} {text:?}\n  want {want:?}\n  got  {got:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
