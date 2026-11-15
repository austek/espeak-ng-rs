// tests/dotted_abbrev_buffer_limit.rs
//
// C `CheckDottedAbbrev` copies the initials into an 80-byte buffer and gives up when the run
// needs more than 80 bytes, so 40 two-byte letters still join and 41 do not.  A run that does
// not fit leaves its first letter to the word lookup instead of spelling it.  Expected values
// are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn cyrillic(count: usize) -> String {
    let letters: Vec<String> = (0..count)
        .map(|i| char::from_u32(0x430 + (i % 32) as u32).unwrap_or('a').to_string())
        .collect();
    format!("{} done", letters.join(" . "))
}

fn check(lang: &str, text: &str, ipa: &str, x: &str) {
    assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text:?}");
    assert_eq!(run(lang, "-x", text), x, "{lang} -x {text:?}");
}

#[test]
fn forty_cyrillic_letters_join() {
    check(
        "ru",
        &cyrillic(40),
        "ˌabˌɛvˈɛɡˌɛdˌɛjˈeʒˌɛzˌɛˈiˌikrˌɑtkɔjɪkˌɑˈɛɭʲˌɛmˌɛnˈopˌɛˌɛrˈɛstˌɛˌuˈɛfxˌɑtsˌɛtʃʲˈeʃˌɑɕˌɑtvʲˈɵrdyjznˌɑkˌymʲˈɑxkʲɪjznˌɑkˌɛˈjuˌjaˌabˈɛvˌɛɡˌɛdˈɛjˌiʒˌɛzˈɛ (en)dˈʌn(ru)",
        ",ab,Ev'Eg,Ed,Ej'eZ,Ez,E'i,ikR,AtkOjIk,A'El;,Em,En_'op,E,Er'Est,E,u'Efx,Ats,EtS;'eS,AS;,Atv;'8@-rdyjzn,Ak,ym;'Axk;I#jzn,Ak,E'ju,ja,ab'Ev,Eg,Ed'Ej,iZ,Ez'E (en)d'Vn(ru)",
    );
}

#[test]
fn forty_one_cyrillic_letters_do_not_join() {
    check(
        "ru",
        &cyrillic(41),
        "a bˌɛvˌɛɡˈɛdˌɛjˌiʒˈɛzˌɛˌiˌikrˈɑtkɔjɪkˌɑˌɛɭʲˈɛmˌɛnˌʌpˈɛˌɛrˌɛstˈɛˌuˌɛfxˈɑtsˌɛtʃʲˌiʃˈɑɕˌɑtvʲˌʌrdyjznˈɑkˌymʲˌaxkʲɪjznˈɑkˌɛˌjuˈjaˌabˌɛvˈɛɡˌɛdˌɛjˈeʒˌɛzˌɛˈi (en)dˈʌn(ru)",
        "a# b,Ev,Eg'Ed,Ej,iZ'Ez,E,i,ikR'AtkOjIk,A,El;'Em,En_,Vp'E,Er,Est'E,u,Efx'Ats,EtS;,iS'AS;,Atv;,V@-rdyjzn'Ak,ym;,axk;Ijzn'Ak,E,ju'ja,ab,Ev'Eg,Ed,Ej'eZ,Ez,E'i (en)d'Vn(ru)",
    );
}
