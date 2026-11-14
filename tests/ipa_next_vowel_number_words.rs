// `nextVowel` stops at the first phoneme that starts a word, which includes
// the words a number is spoken as. Values from upstream espeak-ng master
// (f0fb11a8), `espeak-ng -q --ipa -v bg`.

fn ipa(text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "--ipa", "-v", "bg", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn an_unstressed_a_before_the_next_number_word_is_reduced() {
    let rows = [
        ("0,05", "nˈuɫɐ tsʲˈaɫoi nˈuɫɐ pˈet"),
        ("0,5", "nˈuɫɐ tsʲˈaɫoi pˈet"),
        ("1999", "çilˈadɐ dˈevetstˈotin dˌeveddesˈetidˈevet"),
        ("1234", "çilˈadɐ dvˈestɐ trˈidesˌetitʃˈetirˌi"),
        ("7,008", "sˈedem tsʲˈaɫoi nˈuɫɐ nˈuɫɐ ˈosem"),
        ("0,05 лева", "nˈuɫɐ tsʲˈaɫoi nˈuɫɐ pˈet lˈevɐ"),
    ];
    for (text, want) in rows {
        assert_eq!(ipa(text), want, "{text:?}");
    }
}
