// tests/embedded_sign_before_unknown_letter.rs
//
// A `\x01` that starts no command is blanked by C `ReadClause`, and `EmbeddedCommand` leaves the
// sign, digits and letter as text.  The blank counts: a hyphen after two blanks is no minus sign
// in `en_rules` (`__) - (_D` needs two word breaks before it, `_) - (_D` is a pause), so
// `go\x01-5Qb` reads like `go  -5Qb`.  Expected values are what C espeak-ng master (f0fb11a8)
// prints for `-x`.

use std::process::Command;

fn phonemes(voice: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", voice, text])
        .output()
        .expect("run")
        .stdout;
    String::from_utf8_lossy(&out).trim().to_string()
}

#[test]
fn a_minus_before_an_unknown_command_letter_is_a_pause() {
    assert_eq!(phonemes("en", "go\u{01}-5Qb"), "g,oU _ f'aIv kj,u:b'i:");
    assert_eq!(phonemes("en", "go \u{01}-5Qb"), "g,oU _ f'aIv kj,u:b'i:");
    assert_eq!(phonemes("en", "go\u{01} -5Qb"), "g,oU _ f'aIv kj,u:b'i:");
}

#[test]
fn a_plus_before_an_unknown_command_letter_is_read() {
    assert_eq!(phonemes("en", "go\u{01}+5Qb"), "g,oU pl'Vs f'aIv kj,u:b'i:");
}

#[test]
fn an_unsigned_number_before_an_unknown_command_letter_is_read() {
    assert_eq!(phonemes("en", "go\u{01}5Qb"), "g,oU f'aIv kj,u:b'i:");
}

#[test]
fn a_minus_after_two_blanks_is_a_pause() {
    assert_eq!(phonemes("en", "go  -5"), "g,oU _ f'aIv");
    assert_eq!(phonemes("en", "go   -5Qb"), "g,oU _ f'aIv kj,u:b'i:");
    assert_eq!(phonemes("de", "wort  -5"), "v'Ort _ f'ynf");
    assert_eq!(phonemes("fi", "wort  -5"), "v'ort _ v'i:si");
}

#[test]
fn a_minus_after_one_blank_or_at_a_clause_start_is_still_a_sign() {
    assert_eq!(phonemes("en", "go -5"), "g,oU m'aIn@s f'aIv");
    assert_eq!(phonemes("en", "  -5"), "m'aIn@s f'aIv");
    assert_eq!(phonemes("en", "go.  -5"), phonemes("en", "go. -5"));
}
