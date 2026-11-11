// tests/symbol_reading.rs
//
// Reading technical symbols by name (# $ % & * + / = @ ~).
//
// eSpeak NG speaks these symbols as words by default (without `--punct`),
// while ordinary punctuation (. , ; : ! ? ' " ( ) …) is silent.  The exact
// IPA values below were confirmed byte-identical to the C oracle
// (`espeak-ng -q --ipa -v en "x <sym> y"`), except `@` which inherits a
// pre-existing final-/t/ rendering quirk in the word "at".

use espeak_ng::Translator;

fn en() -> Option<Translator> {
    Translator::new("en", None).ok()
}

#[test]
fn technical_symbols_are_spoken_by_name() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let cases = [
        ("x # y", "ˈɛks hˈaʃ wˈaɪ"),
        ("x $ y", "ˈɛks dˈɒlə wˈaɪ"),
        ("x % y", "ˈɛks pəsˈɛnt wˈaɪ"),
        ("x & y", "ˈɛks and wˈaɪ"),
        ("x * y", "ˈɛks ˈastəɹˌɪsk wˈaɪ"),
        ("x + y", "ˈɛks plˈʌs wˈaɪ"),
        ("x / y", "ˈɛks slˈaʃ wˈaɪ"),
        ("x = y", "ˈɛks ˈiːkwəlz wˈaɪ"),
        ("x ~ y", "ˈɛks tˈɪldə wˈaɪ"),
    ];
    for (input, want) in cases {
        assert_eq!(t.text_to_ipa(input).unwrap(), want, "input {input:?}");
    }
}

#[test]
fn math_and_degree_symbols_are_spoken() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    // Previously these Unicode symbols were dropped entirely.
    let times = t.text_to_ipa("2×3").unwrap();
    assert!(times.contains("tˈaɪmz"), "× → times: {times}");
    let div = t.text_to_ipa("10÷2").unwrap();
    assert!(div.contains("dɪvˈaɪdɪd") && div.contains("baɪ"), "÷ → divided by: {div}");
    let deg = t.text_to_ipa("50°").unwrap();
    assert!(deg.contains("dɪɡɹˈiːz"), "° → degrees: {deg}");
    // U+2212 MINUS SIGN → "minus" (a negative amount reads correctly).
    let minus = t.text_to_ipa("−5").unwrap();
    assert!(minus.contains("mˈaɪnəs"), "− → minus: {minus}");
    let sect = t.text_to_ipa("§5").unwrap();
    assert!(sect.contains("sˈɛkʃən"), "§ → section: {sect}");
    // Math relation/operator symbols (previously dropped).
    assert!(t.text_to_ipa("∞").unwrap().contains("ɪnfˈɪnɪti"), "∞ → infinity");
    assert!(t.text_to_ipa("√4").unwrap().contains("skwˈeə ɹˈuːt"), "√ → square root");
    assert!(t.text_to_ipa("3≠4").unwrap().contains("nˌɒt ˈiːkwəl"), "≠ → not equal to");
    // "or" links into the following vowel with the English linking r
    // (`IfNextVowelAppend(r-)` in the phoneme's own program), as upstream does.
    assert!(t.text_to_ipa("x≤5").unwrap().contains("ɔːɹ ˈiːkwəl"), "≤ → less than or equal");
    // The one-character temperature signs name nothing in espeak-ng master
    // (f0fb11a8): only the number is spoken.
    assert_eq!(t.text_to_ipa("20℃").unwrap(), "twˈɛnti");
    assert_eq!(t.text_to_ipa("72℉").unwrap(), "sˈɛvənti tˈuː");
    // The two-character forms °C / °F say "degrees" and the letter, as upstream does.
    assert_eq!(t.text_to_ipa("20°C").unwrap(), "twˈɛnti dɪɡɹˈiːz sˈiː");
    assert_eq!(t.text_to_ipa("72°F").unwrap(), "sˈɛvənti tˈuː dɪɡɹˈiːz ˈɛf");
    // A bare degree (an angle) stays just "degrees".
    let ang = t.text_to_ipa("90°").unwrap();
    assert!(ang.contains("dɪɡɹˈiːz") && !ang.contains("sˈɛlsɪəs"), "90° stays degrees: {ang}");
    // Trademark / copyright signs (previously dropped).
    // "trade mark", which is what `en_emoji` names U+2122 and what upstream
    // speaks; it was "trademark" while the reading came from a hardcoded table.
    assert_eq!(t.text_to_ipa("™").unwrap(), t.text_to_ipa("trade mark").unwrap(), "™ → trade mark");
    assert_eq!(t.text_to_ipa("©").unwrap(), t.text_to_ipa("copyright").unwrap(), "© → copyright");
    assert!(t.text_to_ipa("Windows®").unwrap().contains("ɹˈɛdʒɪstəd"), "® → registered");
    // Per-mille / per-ten-thousand (previously dropped).
    // C reads neither: English has no word entry for them (C master output).
    assert_eq!(t.text_to_ipa("5‰").unwrap(), "fˈaɪv", "‰");
    assert_eq!(t.text_to_ipa("50‱").unwrap(), "fˈɪfti", "‱");
}

#[test]
fn arithmetic_and_degree_symbols_read_in_other_languages() {
    // Previously `+ − °` fell to the dictionary character-name spell-out for
    // non-English voices, which produced garbage letters (German `+` →
    // "ˈɪːɜˌaʊɐ", `°` → "r.ˈʊʑç").  Now they read the proper word — the one the
    // language's own `*_emoji` data gives, so all but the `es ×` case below
    // (upstream drops that one) are byte-identical to upstream.
    let cases: &[(&str, &str, &str)] = &[
        ("de", "5+3", "plˈʊs"),   // plus
        ("de", "5−2", "mˈiːnʊs"), // minus
        ("de", "20°", "ɡɾˈɑːt"),  // grad
        ("fr", "5+3", "plˈys"),  // plus
        ("fr", "20°", "dəɡʁ"),    // degrés
        ("es", "5+3", "mˈas"),    // más
        ("es", "20°", "ɣɾˈaðos"), // grados
        ("it", "20°", "ɡrˈaːdɪ"), // gradi (the dictionary's own reading)
        ("nl", "20°", "ɣrˈaːdən"), // graden (long vowel: the plural restores it)
        ("de", "5×3", "mˈɑːl"),   // mal (times)
        ("fr", "5×3", "myltipliˈepˈaʁ"), // multiplié par (fr_emoji; "fois" was the hardcoded table)
        ("es", "5×3", "θˈinko tɾˈes"), // C is silent for ×
        ("de", "6÷2", "ɡətˈaɪlt"), // geteilt durch
        ("fr", "6÷2", "divizˈe"),  // divisé par
        ("ru", "50%", "prʌtsˈɛnt"), // процент (was garbage "f")
        ("bg", "50%", "protsˈent"),
    ];
    for (lang, text, want) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let ipa = t.text_to_ipa(text).unwrap();
        assert!(ipa.contains(want), "{lang} {text:?} → want {want:?}, got {ipa:?}");
    }
}

#[test]
fn non_english_currency_symbols_are_spoken() {
    // C reads the symbol's name where it is written and never pluralises it;
    // expected strings are C espeak-ng master output.
    let cases: &[(&str, &str, &str)] = &[
        ("de", "5€", "fˈʏnf ˈɔøroː"),
        ("de", "€5", "ˈɔøroː fˈʏnf"),
        ("nl", "€5", "ˈøːroː vˈɛɪf"),
        ("nl", "5€", "vˈɛɪf ˈøːroː"),
        ("fr", "5€", "sˈɛ̃k øʁˈo"),
        ("es", "5€", "θˈinko ˈeʊɾo"),
        ("it", "5€", "tʃˈinkwe sˈimbolo dˈɛl ˈɛʊro"),
        ("pt", "5€", "sˈiŋku ˈeʊɾʊ"),
        ("de", "5$", "fˈʏnf dɔlˈɑː"),
        ("fr", "5£", "sˈɛ̃k lˈivʁ"),
        ("de", "1€", "ˈaɪns ˈɔøroː"),
        ("es", "1€", "ˈuno ˈeʊɾo"),
        ("en", "$5", "dˈɒlə fˈaɪv"),
        ("en", "$12.99", "dˈɒlə twˈɛlv pɔɪnt nˈaɪn nˈaɪn"),
    ];
    for (lang, text, expected) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa(text).unwrap(), expected, "{lang} {text}");
    }
}

#[test]
fn stylized_letters_normalize_to_ascii() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    let hello = ipa("Hello");
    // Every mathematical / circled style of "Hello" reads the same as plain ASCII.
    for styled in ["𝐇𝐞𝐥𝐥𝐨", "𝐻𝑒𝑙𝑙𝑜", "𝓗𝓮𝓵𝓵𝓸", "𝕳𝖊𝖑𝖑𝖔", "𝙷𝚎𝚕𝚕𝚘", "Ⓗⓔⓛⓛⓞ"] {
        assert_eq!(ipa(styled), hello, "{styled} should read as Hello");
    }
    // Stylized digits normalise too.
    assert_eq!(ipa("𝟏𝟐𝟑"), ipa("123"), "bold digits → 123");
}

#[test]
fn no_abbreviation_is_the_word_no() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    // espeak-ng reads "No." before a number as the word "no" and ends the clause.
    assert_eq!(ipa("No. 5"), "nˈəʊ\nfˈaɪv");
    assert_eq!(ipa("issue No. 42"), "ˈɪʃuː nˈəʊ\nfˈɔːti tˈuː");
    // The word "no" is left alone when no number follows.
    assert!(ipa("No way").contains("nˈəʊ"), "No way stays 'no'");
    assert!(ipa("No. I refuse").contains("nˈəʊ"), "No. I refuse stays 'no'");
}

#[test]
fn dimension_x_reads_by() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    // A lowercase x between two numbers is a dimension: "8x10" → "eight by ten".
    for (dim, spelled) in [("8x10", "8 by 10"), ("2x4", "2 by 4"), ("5 x 7", "5 by 7")] {
        assert_eq!(ipa(dim), ipa(spelled), "{dim} → {spelled}");
    }
    // With no number after the x (algebra, multipliers) it stays the letter.
    assert!(ipa("2x=4").contains("ˈɛks"), "2x= stays letter x: {}", ipa("2x=4"));
    assert!(ipa("3x faster").contains("ˈɛks"), "3x faster stays letter x");
    // A `0` side means it's not a dimension (hex literal `0x1F`).
    assert!(ipa("0x1F").contains("ˈɛks"), "0x1F stays letter x (hex): {}", ipa("0x1F"));
}

#[test]
fn abbreviations_follow_the_dictionary() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    // Upstream master has no expansion for these: they read as listed or spelled.
    let cases: &[(&str, &str)] = &[
        ("vs", "vˌiːˈɛs"),
        ("govt", "ɡˈɒvt"),
        ("Sgt", "ˌɛsdʒˌiːtˈiː"),
        ("Rd", "ˌɑːdˈiː"),
        ("Prof", "pɹˈɒf"),
        ("approx", "ɐpɹˈɒks"),
    ];
    for (abbr, want) in cases {
        assert_eq!(&ipa(abbr), want, "{abbr}");
    }
    assert!(ipa("3rd").contains("θˈɜːd"), "3rd stays third: {}", ipa("3rd"));
    assert!(ipa("road").contains("ɹˈəʊd"), "road unchanged");
}

#[test]
fn unit_abbreviations_are_spelled_out() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    // C does not expand units: the abbreviation is spelled letter by letter
    // (C output for each case below).
    let cases: &[(&str, &str)] = &[
        ("5km", "fˈaɪv kˌeɪˈɛm"),
        ("1km", "wˈɒn kˌeɪˈɛm"),
        ("10 kg", "tˈɛn kˌeɪdʒˈiː"),
        ("500ml", "fˈaɪvhˈʌndɹɪd ˌɛmˈɛl"),
        ("20MB", "twˈɛnti ˌɛmbˈiː"),
        ("100Hz", "wˈɒnhˈʌndɹɪd ˌeɪtʃzˈɛd"),
        ("2.5km", "tˈuː pɔɪnt fˈaɪv kˌeɪˈɛm"),
    ];
    for (abbr, want) in cases {
        assert_eq!(ipa(abbr), *want, "{abbr}");
    }
    // A non-unit word after a number, or a unit with no number, is untouched.
    assert!(ipa("5 cats").contains("kˈats"), "5 cats unchanged");
    // A bare "km" has no vowel, so it is spelled out ("kay em") — upstream reads
    // it the same way.
    assert_eq!(ipa("km"), "kˌeɪˈɛm", "bare km is spelled");
}

#[test]
fn units_in_other_languages() {
    // C spells the abbreviation by letters except where the language's own
    // dictionary lists it (fr km, kg); the values are C's output.
    let cases: &[(&str, &str, &str)] = &[
        ("de", "5km", "fˈʏnf kˌɑːˈɛm"),
        ("de", "10kg", "tsˈeːn kˌɑːɡˈeː"),
        ("fr", "5km", "sˈɛ̃k kilomˈɛtʁ"),
        ("fr", "1km", "ˈœ̃ kilomˈɛtʁ"),
        ("es", "5km", "θˈinko kˌaˈɛme"),
        ("it", "5km", "tʃˈinkwe kˌapːaˈɛmme"),
    ];
    for (lang, abbr, want) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(t.text_to_ipa(abbr).unwrap(), *want, "{lang} {abbr}");
    }
}

#[test]
fn decades_read_as_plurals() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    // "1980s" used to read "…eighty ess"; now it's a spoken decade.
    let cases: &[(&str, &str)] = &[
        ("1980s", "nineteen eighties"),
        ("1990s", "nineteen nineties"),
        ("1900s", "nineteen hundreds"),
        ("1850s", "eighteen fifties"),
        ("80s", "eighties"),
        ("90s", "nineties"),
        ("20s", "twenties"),
        // The possessive-apostrophe spelling is also a decade.
        ("1990's", "nineteen nineties"),
        ("80's", "eighties"),
    ];
    for (digit, word) in cases {
        assert_eq!(ipa(digit), ipa(word), "{digit} should read as {word}");
    }
    // Not a decade: a non-round number keeps its normal reading + trailing 's'.
    assert_eq!(ipa("1985"), ipa("1985"), "sanity");
    assert!(ipa("5s").contains("ˈɛs"), "5s stays 'five ess': {}", ipa("5s"));
    // 2000s are ambiguous and deliberately left alone.
    assert!(ipa("2000s").contains("ˈɛs"), "2000s left alone: {}", ipa("2000s"));
}

#[test]
fn ordinary_punctuation_is_silent() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    // None of these add a spoken word between "x" and "y".
    for input in ["x ! y", "x ( y", "x ) y", "x < y", "x > y", "x ? y", "x ; y"] {
        let ipa = t.text_to_ipa(input).unwrap().replace('\n', " ");
        let words: Vec<&str> = ipa.split_whitespace().collect();
        assert_eq!(words.len(), 2, "punctuation {input:?} was spoken: {ipa:?}");
    }
}

#[test]
fn punct_announces_punctuation_by_name() {
    let Some(mut t) = en() else { eprintln!("[SKIP] no en data"); return };
    // Baseline: ordinary punctuation is silent (two spoken words).
    let base = t.text_to_ipa("a, b.").unwrap().replace('\n', " ");
    assert_eq!(base.split_whitespace().count(), 2, "baseline {base:?}");

    // `--punct` (all): comma and period are announced by name.
    t.options.punct = Some(Vec::new());
    assert_eq!(t.text_to_ipa("a, b.").unwrap(), "ˈeɪ kˈɒmə bˈiː dˈɒt");

    // `--punct=","` (only comma): the period stays silent.
    t.options.punct = Some(vec![',']);
    let only_comma = t.text_to_ipa("a, b.").unwrap();
    assert!(only_comma.contains("kˈɒmə"), "comma should be announced: {only_comma:?}");
    assert!(!only_comma.contains("dˈɒt"), "period should stay silent: {only_comma:?}");
}

#[test]
fn punct_reaches_the_code_stream() {
    // The `-x` / synthesis path (`translate_to_codes`) must also emit punct names.
    let Some(mut t) = en() else { return };
    let count = |t: &Translator, s: &str| {
        t.translate_to_codes(s).unwrap().iter().filter(|c| !c.is_boundary && c.marker.is_none()).count()
    };
    let plain = count(&t, "a, b.");
    t.options.punct = Some(Vec::new());
    let punct = count(&t, "a, b.");
    assert!(punct > plain, "--punct added no codes to -x path: {punct} vs {plain}");
}

#[test]
fn spoken_symbols_reach_the_code_stream() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    // Regression: `translate_to_codes` (the `-x` / synthesis path) dropped
    // spoken symbols even though `--ipa` (`text_to_ipa`) rendered them — the
    // emission match had no `Token::Punctuation` arm.  `&` → "and" must now emit.
    // Pauses don't count as speech: a bracket is silent but still pauses
    // (`LOPT_BRACKET_PAUSE`), so `a (b) c` gains two pauses per bracket.
    let n = |s: &str| {
        t.translate_to_codes(s)
            .expect("codes")
            .iter()
            .filter(|c| {
                !c.is_boundary
                    && c.marker.is_none()
                    && c.code != espeak_ng::phoneme::PHON_PAUSE
            })
            .count()
    };
    assert!(n("a & b") > n("a b"), "spoken '&' added no codes");
    assert!(n("5 % x") > n("5 x"), "spoken '%' added no codes");
    // Non-spoken punctuation stays silent (no extra spoken codes vs plain text).
    assert_eq!(n("a (b) c"), n("a b c"), "parentheses should be silent");
    // …and the pause is there.
    let pauses = |s: &str| {
        t.translate_to_codes(s)
            .expect("codes")
            .iter()
            .filter(|c| c.code == espeak_ng::phoneme::PHON_PAUSE)
            .count()
    };
    assert_eq!(pauses("a (b) c"), pauses("a b c") + 4, "brackets should pause");
}

#[test]
fn ampersand_reads_and_not_ampersand() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    // Confirmed identical to the oracle.
    assert_eq!(t.text_to_ipa("Tom & Jerry").unwrap(), "tˈɒm and dʒˈɛɹi");
}

#[test]
fn percent_after_number() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = t.text_to_ipa("50 % off").unwrap();
    assert!(ipa.contains("pəsˈɛnt"), "expected 'percent' in {ipa:?}");
}

#[test]
fn numero_sign_reads_number_word() {
    // № is read as each language's own "numero" word, but only where espeak-ng
    // defines one.  In these languages the sign is spoken, so "№5" differs from
    // a bare "5".  (Values follow the C oracle.)
    for lang in ["en", "de", "it", "pt", "nl", "ru", "bg", "cs"] {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let n = t.text_to_ipa("№5").unwrap();
        let bare = t.text_to_ipa("5").unwrap();
        assert!(!n.is_empty() && n != bare, "{lang} №5 dropped the numero sign: {n:?} (bare {bare:?})");
    }
    // English reads "numero" (matching espeak-ng), not "number".
    if let Some(t) = en() {
        assert!(t.text_to_ipa("№5").unwrap().contains("njˈuːmə"), "en №5 = numero five");
    }
    // Other languages have no numero word of their own, and C's switch to
    // English finds no listed word for it, so the sign is silent (C master:
    // `-v fr №5` is `sˈɛ̃k`, `-v es` `θˈinko`, `-v uk` `pjˈat`, `-v pl` `pʲˈɛɲtɕ`).
    for lang in ["fr", "es", "uk", "pl"] {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa("№5").unwrap();
        assert_eq!(got, t.text_to_ipa("5").unwrap(), "{lang} №5: {got}");
    }
}

#[test]
fn multiply_and_divide_read_across_languages() {
    // C reads × and ÷ only where the language's own data names them (pl
    // "razy", cs "krát", sv "gånger", pt "dividido por"); elsewhere it is
    // silent.  Expected values are C master output.
    let cases: &[(&str, &str, &str)] = &[
        ("pt", "2×3", "dˈoɪʒ vˈezɨʃ tɹˈeʃ"),
        ("pl", "2×3", "dvˈa rˈazɨ tʃˈɨ"),
        ("cs", "2×3", "dvˈa krˈaːt tr̝̊ˈi"),
        ("ro", "2×3", "dˈoɪ trˈeɪ"),
        ("sv", "2×3", "tvˈoː ɡˈɔŋər trˈeː"),
        ("da", "2×3", "tˈo ɡˈaŋe tʁˈʔe"),
        ("nb", "2×3", "tˈuː trˈeː"),
        ("tr", "2×3", "icˈi ˈytʃ"),
        ("ru", "2×3", "dvˈɑ trʲˈi"),
        ("uk", "2×3", "dβˈa trˈɪ"),
        ("bg", "2×3", "dvˈe trˈi"),
        ("pt", "10÷2", "dˈɛʒ dˌividˈidʊpˈor dˈoɪʃ"),
        ("pl", "10÷2", "dʑˈɛɕɛɲdʑ dʑɛlˈɛɲʲɛ dvˈa"),
        ("sv", "10÷2", "tˈiːuː dˈeːlat meːd tvˈoː"),
        ("ru", "10÷2", "dʲˈesʲɪtʲ dvˈɑ"),
    ];
    for (lang, text, want) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{lang} {text:?}");
    }
}

#[test]
fn plus_minus_sign_reads_across_languages() {
    // Languages with an own `±` entry read it; the rest switch to English
    // ("plus or minus").  Expected values are C master output.
    let cases: &[(&str, &str)] = &[
        ("es", "(en)plˈʌs ɔː mˈaɪnəs(es) θˈinko"),
        ("it", "(en)plˈʌs ɔː mˈaɪnəs(it) tʃˈinkwe"),
        ("pt", "(en)plˈʌs ɔː mˈaɪnəs(pt-pt) sˈiŋku"),
        ("pl", "plusmˈinus pʲˈɛɲtɕ"),
        ("cs", "plˈus nˌebo mˈiːnus pjˈet"),
        ("sv", "plˈʉːs ˌɛlər mˈiːnʉːs fˈɛm"),
        ("ru", "(en)plˈʌs ɔː mˈaɪnəs(ru) pʲˈɑtʲ"),
        ("uk", "(en)plˈʌs ɔː mˈaɪnəs(uk) pjˈat"),
    ];
    for (lang, want) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa("±5").unwrap(), want, "{lang} ±5");
    }
}

#[test]
fn hebrew_and_ukrainian_operators() {
    // Hebrew: + − × ÷ ° were dropped; now read (via niqqud-pointed words that
    // espeak-Hebrew can pronounce).  Ukrainian: ° was dropped.
    if let Some(he) = Translator::new("he", None).ok() {
        assert_eq!(he.text_to_ipa("2+3").unwrap(), "ʃtˈajim (en)plˈʌs(he) ʃalˈoʃ");
        assert_eq!(he.text_to_ipa("10÷2").unwrap(), "ˈeser ʃtˈajim");
        // A leading `-` before a number is an *operator*: it is read from the
        // language's own rules or list, and Hebrew's data has neither, so the
        // sign is dropped — as it is in C.
        assert_eq!(he.text_to_ipa("-5").unwrap(), he.text_to_ipa("5").unwrap(), "he -5");
        // The sign written literally still reads by its dictionary name.
        assert_eq!(he.text_to_ipa("−").unwrap(), "sˈimnə mˈinvs");
        assert_eq!(he.text_to_ipa("20°").unwrap(), "esrˈim");
    }
    if let Some(uk) = Translator::new("uk", None).ok() {
        assert_eq!(uk.text_to_ipa("20°").unwrap(), "dvˈatsjat");
        assert_eq!(uk.text_to_ipa("2×3").unwrap(), "dβˈa trˈɪ");
    }
}

#[test]
fn degree_sign_reads_across_languages() {
    // C master output: sv reads "grader", da/nb/ru/bg/uk are silent for `°`.
    let cases: &[(&str, &str, &str)] = &[
        ("sv", "20°", "ɕˈʉːɡɔ ɡrˈadər"),
        ("da", "20°", "tˈyːʔu"),
        ("nb", "20°", "ʂˈʉːa"),
        ("ru", "20°", "dvˈɑtsʌtʲ"),
        ("bg", "20°", "dvˈadesˌet"),
        ("sv", "-40°", "mˈiːnɵs fˈœʈɪ ɡrˈadər"),
        ("nb", "-40°", "mˈiːnʉːs fˈøːrtiː"),
    ];
    for (lang, text, want) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{lang} {text:?}");
    }
}
