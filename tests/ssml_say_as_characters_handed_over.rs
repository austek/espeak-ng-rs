// tests/ssml_say_as_characters_handed_over.rs
//
// C `TranslateLetter` speaks a character the voice has no name for in the voice of its script
// (English) between a switch to it and a switch back, inside the spelled word, so the characters
// around it are kept (translateword.c:1036).  Expected values are what C espeak-ng master
// (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 9] = [
    (
        "hi",
        "<speak><say-as interpret-as=\"characters\">123456789</say-as></speak>",
        "ˌeːkdˌoː(en)kˈɒmə(hi) ʃˌuːnjətˌiːncˌaːɾ(en)kˈɒmə(hi) ʃˌuːnjəpˌa\u{303}ccʰˌəh(en)kˈɒmə(hi) sˌaːtˌaːʈʰnˈɔː",
        ",e:k_|d,o:_|_:(en)k'0m@(hi)_! S,u:nj@-_|t,i:n_|c,a:r_!_:(en)k'0m@(hi)_! S,u:nj@-_|p,a~c_|c#,@H_!_:(en)k'0m@(hi)_! s,a:t_|,a:t.#_|n'O:_!",
    ),
    (
        "hi",
        "<speak><say-as interpret-as=\"characters\">12,345,678</say-as></speak>",
        "ˌeːkdˌoː(en)kˈɒmə(hi)tˌiːncˌaːɾpˈa\u{303}c(en)kˌɒmə(hi)cʰˌəhsˌaːtˈaːʈʰ",
        ",e:k_|d,o:_|_:(en)k'0m@(hi)_!t,i:n_|c,a:r_|p'a~c_!_:(en)k,0m@(hi)_|c#,@H_|s,a:t_!'a:t.#_!",
    ),
    (
        "hi",
        "<speak><say-as interpret-as=\"characters\">12345-6789</say-as></speak>",
        "ˌeːkdˌoː(en)kˈɒmə(hi) tˌiːncˌaːɾpˈa\u{303}c(en)hˌaɪfən(hi)cʰˌəhsˌaːtˌaːʈʰnˈɔː",
        ",e:k_|d,o:_|_:(en)k'0m@(hi)_! t,i:n_|c,a:r_|p'a~c_!_:(en)h,aIf@n(hi)_!c#,@H_|s,a:t_|,a:t.#_!n'O:_!",
    ),
    (
        "hi",
        "<speak><say-as interpret-as=\"characters\">abc12345</say-as></speak>",
        "(en)ˌeɪ(hi)(en)bˌiː(hi)(en)sˈiː(hi)ˌeːkdˌoːtˈiːncˌaːɾpˈa\u{303}c",
        "_:(en),eI(hi)_|_:(en)b,i:(hi)_|_:(en)s'i:(hi)_!,e:k_|d,o:_|t'i:n_!c,a:r_|p'a~c_!",
    ),
    (
        "hi",
        "<speak><say-as interpret-as=\"characters\">12,3</say-as></speak>",
        "ˌeːkdˌoː(en)kˌɒmə(hi)tˈiːn",
        ",e:k_|d,o:_|_:(en)k,0m@(hi)_!t'i:n_!",
    ),
    (
        "hi",
        "<speak><say-as interpret-as=\"characters\">ab,c</say-as></speak>",
        "(en)ˌeɪ(hi)(en)bˌiː(hi)(en)kˌɒmə(hi)(en)sˈiː(hi)",
        "_:(en),eI(hi)_|_:(en)b,i:(hi)_|_:(en)k,0m@(hi)_!_:(en)s'i:(hi)_!",
    ),
    (
        "zh",
        "<speak><say-as interpret-as=\"characters\">12345</say-as></speak>",
        "jˌi5ˌər5(en)kɒ1mə1(cmn) sˌa5nsˌi̪5wˈu2",
        "j,i55_|,@r51_|_:(en)k011m@11(cmn)_!_| s,a55n_|s,i[51_|w'u214_!_|",
    ),
    (
        "ja",
        "<speak><say-as interpret-as=\"characters\">12345</say-as></speak>",
        "ˌitsinˌiː(en)kˈɒmə(ja) sˌänsˌiɡˈo̞",
        ",it_si_!_|n,i:_!_|_:(en)k'0m@(ja)_! s,an_!_|s,i_!_|g'o_!_!",
    ),
    (
        "ar",
        "<speak><say-as interpret-as=\"characters\">12345</say-as></speak>",
        "wˌaːħidʔiθnˌaːn(en)kˈɒmə(ar) θalˌaːθaʔˌarbaʕʕˌaxˈamsa",
        "w,a:Hid_|?iTn,a:n_|_:(en)k'0m@(ar)_! Tal,a:Ta_|?,aRbaA-,a_|x'amsa_!",
    ),
];

#[test]
fn spelled_character_named_by_another_voice_keeps_its_neighbours() {
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
