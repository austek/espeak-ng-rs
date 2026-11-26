// tests/symbol_run_long_words.rs
//
// Long words next to a symbol run (`//`, `##`, `~~`, `://`), against C (f0fb11a8).
//
// `ROWS` is a parity sweep, not a boundary test: with ASCII letters the C output stops
// depending on the letter count well below 160 bytes (the phoneme buffer fills first), so
// in en, es, fr and ru the rows for 159, 160 and 161 letters share one hash (de keeps
// changing every two letters), and rebuilding C with N_WORD_BYTES of 480 changes none of
// them.
//
// C's N_WORD_BYTES (160) limit shows only with two-byte letters in the languages where an
// accented Latin letter has no rule of its own (tr, th, he, ...).  A word of 80 "é" (160
// bytes) is read whole and a word of 81 or more is cut to those 80; with N_WORD_BYTES of
// 100 C already cuts at 50.  `BOUNDARY_ROWS` pin the rows up to 160 bytes, which Rust
// reads like C.  `CUT_ROWS` pin the cut above 160 bytes, which Rust does not make: it
// reads the whole word (ignored until the difference is fixed).  Rust's own `N_WORD_BYTES`
// only gates the dictionary lookup, so changing it moves none of these rows.
//
// Each row is (language, text before the word, filler, filler count, text after the word,
// byte length of the output including its newline, FNV-1a 64 of that output): the
// `-q --ipa` output of the C build.

use espeak_ng::Translator;

type Row = (&'static str, &'static str, &'static str, usize, &'static str, usize, u64);

const ROWS: &[Row] = &[
    ("en", "x //", "a", 159, "// y", 336, 0xc979709c1514365e),
    ("en", "x //", "a", 160, "// y", 336, 0xc979709c1514365e),
    ("en", "x //", "a", 161, "// y", 336, 0xc979709c1514365e),
    ("en", "x //", "é", 80, "// y", 360, 0x40ddcc87119e9d11),
    ("en", "x //", "é", 81, "// y", 363, 0x4a10f2bf311730e3),
    ("en", "x //", "7", 161, "// y", 280, 0x9731ba9640595008),
    ("de", "x //", "a", 159, "// y", 423, 0xb0807a1f1098bf21),
    ("de", "x //", "a", 160, "// y", 423, 0xb0807a1f1098bf21),
    ("de", "x //", "a", 161, "// y", 429, 0xed49ed2dceb800f7),
    ("de", "x //", "é", 80, "// y", 343, 0x2f12004000138197),
    ("de", "x //", "é", 81, "// y", 348, 0xb21f49c43dd074be),
    ("de", "x //", "7", 161, "// y", 289, 0xcbb2606dd8c6c476),
    ("es", "x //", "a", 159, "// y", 237, 0x5a1e4aaddd3567e5),
    ("es", "x //", "a", 160, "// y", 237, 0x5a1e4aaddd3567e5),
    ("es", "x //", "a", 161, "// y", 237, 0x5a1e4aaddd3567e5),
    ("es", "x //", "é", 80, "// y", 281, 0x32e3b80871df0734),
    ("es", "x //", "é", 81, "// y", 284, 0x0f2e221d3c1ad9e0),
    ("es", "x //", "7", 161, "// y", 250, 0x263d2567c9f238b0),
    ("fr", "x //", "a", 159, "// y", 118, 0x2e9943da2a24f290),
    ("fr", "x //", "a", 160, "// y", 118, 0x2e9943da2a24f290),
    ("fr", "x //", "a", 161, "// y", 118, 0x2e9943da2a24f290),
    ("fr", "x //", "é", 80, "// y", 100, 0xe63f55a7f123b9e6),
    ("fr", "x //", "é", 81, "// y", 101, 0x9db704765d9feec3),
    ("fr", "x //", "7", 161, "// y", 194, 0xfe635a26ce305acf),
    ("ru", "x //", "a", 159, "// y", 428, 0xf5d3c9a7e25dadf2),
    ("ru", "x //", "a", 160, "// y", 428, 0xf5d3c9a7e25dadf2),
    ("ru", "x //", "a", 161, "// y", 428, 0xf5d3c9a7e25dadf2),
    ("ru", "x //", "é", 80, "// y", 452, 0x0ac8d90571838f71),
    ("ru", "x //", "é", 81, "// y", 455, 0x22973aad915b6939),
    ("ru", "x //", "7", 161, "// y", 413, 0x7dc537e2d11926e4),
    ("en", "a :// ", "a", 200, "", 323, 0x0178b5d2dcf6ae25),
    ("en", "x ##", "a", 200, "## y", 332, 0x2bdad52f05e43df6),
    ("en", "", "a", 200, "//", 309, 0x538d32aca923850e),
    ("en", "//", "a", 200, "//", 322, 0x4964c1347a2f7c79),
    ("en", "x ~~ ", "a", 200, " ~~ y", 350, 0xf25567059e75242e),
    ("fr", "a :// ", "a", 200, "", 103, 0x149c5d699a30e317),
    ("fr", "x ##", "a", 200, "## y", 118, 0x2e9943da2a24f290),
    ("fr", "", "a", 200, "//", 101, 0x82ddf794f6e15018),
    ("fr", "//", "a", 200, "//", 101, 0x82ddf794f6e15018),
    ("fr", "x ~~ ", "a", 200, " ~~ y", 144, 0xb201eaad9d50176e),
];

/// Up to 160 bytes of "é": C's output depends on the letter count here, and Rust agrees.
const BOUNDARY_ROWS: &[Row] = &[
    ("tr", "", "é", 51, "", 55, 0x729be2043aa19952),
    ("tr", "", "é", 60, "", 64, 0x33491a1afd5f3e9d),
    ("tr", "", "é", 79, "", 83, 0x342ef5fa3e7026be),
    ("tr", "", "é", 80, "", 84, 0x4d6b3389b3fe5831),
    ("tr", "x //", "é", 51, "// y", 67, 0x76d208307ff26715),
    ("tr", "x //", "é", 60, "// y", 76, 0xe7781e02e9179a2e),
    ("tr", "x //", "é", 79, "// y", 95, 0x03eb69147b185be1),
    ("tr", "x //", "é", 80, "// y", 96, 0x63595552659bb9d2),
    ("tr", "a :// ", "é", 51, "", 59, 0x7ebc8abea3dd7b3c),
    ("tr", "a :// ", "é", 60, "", 68, 0x79f59d5903ddf133),
    ("tr", "a :// ", "é", 79, "", 87, 0x9c11c72320765a58),
    ("tr", "a :// ", "é", 80, "", 88, 0x2230230bccdd7937),
    ("th", "", "é", 51, "", 211, 0x64bf3977062dbe64),
    ("th", "", "é", 79, "", 323, 0x1e993300eab0bd18),
    ("th", "", "é", 80, "", 328, 0x6272f5e8492db71f),
    ("th", "x //", "é", 51, "// y", 251, 0xa2789680a9be1cc5),
    ("th", "x //", "é", 79, "// y", 363, 0x5bafa226e3683b79),
    ("th", "x //", "é", 80, "// y", 368, 0x410d796dbc19c3fe),
    ("th", "a :// ", "é", 79, "", 353, 0x6d2be9ee27f556f0),
    ("th", "a :// ", "é", 80, "", 358, 0x2ba9615412cd5187),
    ("he", "", "é", 51, "", 211, 0x74cba5d00a66f213),
    ("he", "", "é", 79, "", 323, 0x29129718ecfbb237),
    ("he", "", "é", 80, "", 328, 0xd299f8882c983b80),
    ("he", "x //", "é", 51, "// y", 251, 0x4e2010975431988e),
    ("he", "x //", "é", 79, "// y", 363, 0x6d04350307d43162),
    ("he", "x //", "é", 80, "// y", 368, 0xdc54c5c70f5d953d),
    ("he", "a :// ", "é", 79, "", 353, 0xc1425f94cb77acaf),
    ("he", "a :// ", "é", 80, "", 358, 0xe617f477d0b7a468),
];

/// More than 160 bytes of "é": C cuts the word at its limit (tr: the same output as for 80).
const CUT_ROWS: &[Row] = &[
    ("tr", "", "é", 81, "", 84, 0x4d6b3389b3fe5831),
    ("tr", "", "é", 100, "", 84, 0x4d6b3389b3fe5831),
    ("tr", "x //", "é", 81, "// y", 96, 0x63595552659bb9d2),
    ("tr", "x //", "é", 100, "// y", 96, 0x63595552659bb9d2),
    ("tr", "a :// ", "é", 81, "", 88, 0x2230230bccdd7937),
    ("tr", "a :// ", "é", 100, "", 88, 0x2230230bccdd7937),
    ("th", "", "é", 81, "", 328, 0x6272f5e8492db71f),
    ("th", "x //", "é", 81, "// y", 368, 0x410d796dbc19c3fe),
    ("th", "a :// ", "é", 81, "", 358, 0x2ba9615412cd5187),
    ("he", "", "é", 81, "", 328, 0xd299f8882c983b80),
    ("he", "x //", "é", 81, "// y", 368, 0xdc54c5c70f5d953d),
    ("he", "a :// ", "é", 81, "", 358, 0xe617f477d0b7a468),
];

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli_output(lang: &str, text: &str) -> Option<String> {
    let ipa = Translator::new(lang, None).ok()?.text_to_ipa(text).unwrap();
    Some(format!("{ipa}\n"))
}

#[test]
fn long_words_beside_symbol_runs_read_as_in_c() {
    check(ROWS);
}

#[test]
fn two_byte_letter_words_up_to_the_word_limit_read_as_in_c() {
    check(BOUNDARY_ROWS);
}

#[test]
#[ignore = "Rust does not cut a word at C's N_WORD_BYTES (160 bytes), issue #27; see the header"]
fn two_byte_letter_words_beyond_the_word_limit_are_cut_as_in_c() {
    check(CUT_ROWS);
}

fn check(rows: &[Row]) {
    for (lang, before, filler, count, after, len, hash) in rows {
        let text = format!("{before}{}{after}", filler.repeat(*count));
        let Some(got) = cli_output(lang, &text) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let what = format!("{lang} {before:?} {filler:?}*{count} {after:?}");
        assert_eq!(got.len(), *len, "{what}: output length");
        assert_eq!(fnv1a(&got), *hash, "{what}: output hash");
    }
}
