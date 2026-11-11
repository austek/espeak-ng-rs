// A digit run of 5+ digits that starts with 0 is read digit by digit, one word
// per group of three. C appends the thousands separator to every group but the
// last, and the language's rules speak a glued dot ("punto"). Expected
// mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join(format!("{lang}_dict")).exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no {lang} data");
            return;
        };
        assert_eq!(got, *want, "{lang} {text}");
    }
}

#[test]
fn dot_languages_speak_the_group_separator() {
    check(&[
        ("es", "000123", "T'e**oT'e**o_!T'e**op,unto 'unoD'os_!t**'es"),
        ("pt", "00012", "z'E*uz'E*u_!p'oNtU z'E*u'um_!d'oIs#"),
        ("nl", "00012", "n'8ln,8l_!p8nt n'8l,e:n_!tv#,e:"),
        ("pl", "00012", "z'ERO:z'ERO:_!kR'Opka# z'ERO:j'EdEn_!dv'a"),
        ("sv", "00012", "n'Oln'Ol_!p'8Nt n'Ol'Et_!tv'o:"),
        ("uk", "00012", "n'ul^n'ul^_!kR'apka n'ul^od'en_!dB'a"),
        ("no", "000123", "n'u-ln'u-l_!n'u-lpu-Nkt,u-:m 'e:nt'u:_!tR'e:"),
        (
            "eo",
            "0001234567",
            "nulp'unkto n,ulnul_!'unup,unkto d'ut@-R'i_!kv'aRpunkto kv'ins'es_!s'ep",
        ),
    ]);
}

#[test]
fn languages_without_a_dot_word_keep_the_groups_bare() {
    check(&[("de", "000123", "n'Uln'Ul_!n'Ul _'aInstsv'aI_!dr'aI")]);
}

#[test]
fn es_groups_run_from_the_right() {
    check(&[(
        "es",
        "0001234567",
        "T'e**op,unto T'e**oT'e**o_!'unop,unto D'ost**'es_!kw'at**op,unto T'inkos'eIs_!sj'ete",
    )]);
}
