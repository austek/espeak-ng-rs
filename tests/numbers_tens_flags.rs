// Tens/units order and the "and" follow the language's `numbers` flags (C's NUM_SWAP_TENS
// and NUM_AND_UNITS, from the voice file as well as the built-in table), not the language
// name.  Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>`.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "--ipa", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn tens_order_follows_the_numbers_flags() {
    for (lang, text, want) in [
        ("qdb", "34", "fˌuːʔuːnjəseːɽˈiːteːŋ"),
        ("an", "34", "tɾˌentaikwˈatɾo"),
        ("an", "21", "bˌintiˈun"),
        ("an", "45,67", "kwaɾˌantaiθˈinko kon sˈeɪs sjˈɛt"),
        ("smj", "34", "nˌelljakˈolomolok"),
        ("ka", "34", "ˈotsʰdatʰˈotʰxmeti"),
        ("ka", "57", "ˈormotsdatʃvˈidmeti"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
