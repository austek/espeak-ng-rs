// tests/embedded_command_case.rs
//
// C `EmbeddedCommand` looks the letter up with `strchr_w(commands, c)` over "PSARHTIVYMUBF"
// (translate.c:786-856): the match is case-sensitive and `z` is not a command, so a lowercase
// letter, `z` and `Z` all leave the sign, digits and letter as text.
// Expected values are what C espeak-ng master (f0fb11a8) prints for `--ipa` and `-x`.

use std::process::Command;

fn cli(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli")).args(args).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn both(text: &str) -> (String, String) {
    (cli(&["-q", "--ipa", text]), cli(&["-q", "-x", text]))
}

fn assert_c(text: &str, ipa: &str, phonemes: &str) {
    assert_eq!(both(text), (ipa.to_string(), phonemes.to_string()), "{text:?}");
}

#[test]
fn lowercase_z_is_read_as_text() {
    assert_c("go\u{01}50zb", "ɡˌəʊ fˈɪfti zˌɛdbˈiː", "g,oU f'Ifti z,Edb'i:");
}

#[test]
fn uppercase_z_is_not_a_command() {
    assert_c("go\u{01}50Zb", "ɡˌəʊ fˈɪfti zˌɛdbˈiː", "g,oU f'Ifti z,Edb'i:");
}

#[test]
fn lowercase_command_letters_are_read_as_text() {
    assert_c("go\u{01}50pb", "ɡˌəʊ fˈɪfti pˌiːbˈiː", "g,oU f'Ifti p,i:b'i:");
    assert_c("go\u{01}+5sb", "ɡˌəʊ plˈʌs fˈaɪv ˌɛsbˈiː", "g,oU pl'Vs f'aIv ,Esb'i:");
    assert_c("go\u{01}50p b", "ɡˌəʊ fˈɪfti pˈiː bˈiː", "g,oU f'Ifti p'i: b'i:");
}

#[test]
fn uppercase_command_letters_are_still_consumed() {
    assert_c("go\u{01}50Pb", "ɡˌəʊ bˈiː", "g,oU b'i:");
}
