// C's `IsAlpha` takes every codepoint in 0x901..0xdf7 whose low 7 bits are
// below 0x64 as a letter, whatever script it belongs to, and `IsEmoji` ends
// at U+1FBFF. Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q -x -v <lang>`.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_letter_of_another_indic_script_stays_in_the_word() {
    assert_eq!(cli("hi", "\u{c95}\u{cdd}"), "(kn)k'a(hi)");
    assert_eq!(cli("hi", "\u{d81}\u{d9a}"), "(si)k'a(hi)");
    assert_eq!(
        cli("ta", "\u{c28}\u{c5d}"),
        "_:(en)t'Elugu_:(te)n'a _:(en)t'Elugu(ta)kURij,i:d.U__|s'i:_|'aIndU_|d'i:_:"
    );
}

#[test]
fn vedic_accent_marks_are_word_letters_in_every_indic_voice() {
    let rows = [
        ("hi", "\u{915}\u{953}", "k'@ Vks.@r_|n'O:_|p'a~c_|t'i:n_:"),
        ("hi", "\u{915}\u{954}", "k'@ Vks.@r_|n'O:_|p'a~c_|c'a:r_:"),
        ("kn", "\u{c95}\u{951}", "k'a _:(en)h'Indi(kn)Ud'a:t"),
        ("si", "\u{d9a}\u{954}", "k'@ _:(en)h'Indi(si)l'et@_|n'av@j@_|p'ah@_|h'at@r@_:"),
    ];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}

#[test]
fn emoji_range_reaches_the_last_symbols_for_legacy_computing_block() {
    assert_eq!(
        cli("en", "a\u{1fbf0}b"),
        "a# sImb@L_|w'0n_|'Ef_|b'i:_|'Ef_|z'i@roU_: b'i:"
    );
}
