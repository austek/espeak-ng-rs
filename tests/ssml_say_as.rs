// `<say-as>` spoken through the `Y` embedded command, against C espeak-ng master (f0fb11a8).
// Expected values: whole stdout of `espeak-ng -q -m -v en --ipa|-x <text>`, and the sample
// count `(bytes - 44) / 2` of `espeak-ng -m -v en -w`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn samples(text: &str) -> usize {
    let dir = std::env::temp_dir().join(format!("ssml_say_as_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", text.bytes().map(usize::from).sum::<usize>() * 31 + text.len()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-m", "-v", "en", "-w"])
        .arg(&path)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav").len();
    let _ = std::fs::remove_file(&path);
    bytes.saturating_sub(44) / 2
}

fn have_data() -> bool {
    let ok = std::path::Path::new("espeak-ng-data/en_dict").exists();
    if !ok {
        eprintln!("[SKIP] no data");
    }
    ok
}

/// `(input, --ipa, -x)` as printed by C.
type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !have_data() {
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(text, ipa, x)| [("--ipa", ipa), ("-x", x)].map(move |(f, want)| (text, f, want)))
        .filter_map(|(text, flag, want)| {
            let got = run(flag, text);
            (got != want).then(|| format!("{flag} {text:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

fn check_samples(cases: &[(&str, usize)]) {
    if !have_data() {
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(t, want)| {
            let got = samples(t);
            (got != want).then(|| format!("{t:?}: got {got}, C {want}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn characters_spell_each_letter_with_pauses() {
    check(&[
        (
            r#"<say-as interpret-as="characters">abc</say-as>"#,
            "ˌeɪbˌiːsˈiː\n",
            ",eI_|b,i:_|s'i:_!\n",
        ),
        (
            r#"<say-as interpret-as="characters">ab cd</say-as>"#,
            "ˌeɪbˈiː sˌiːdˈiː\n",
            ",eI_|b'i:_!_:_: s,i:_|d'i:_!\n",
        ),
        (
            r#"<say-as interpret-as="characters">Hi!</say-as> there"#,
            "ˌeɪtʃˌaɪˌɛkskləmˈeɪʃən ðˈeə\n",
            ",eItS_|,aI_|,Ekskl@m'eIS@n_! D'e@\n",
        ),
        (
            r#"<say-as interpret-as="characters">A1b</say-as>"#,
            "ˌeɪwˌɒnbˈiː\n",
            ",eI_|w,0n_|b'i:_!\n",
        ),
        (
            r#"<say-as interpret-as="characters">a.b</say-as>"#,
            "ˌeɪdˌɒtbˈiː\n",
            ",eI_|d,0t_|b'i:_!\n",
        ),
        (
            r#"x <say-as interpret-as="characters">US</say-as> y"#,
            "ˈɛks jˌuːˈɛs wˈaɪ\n",
            "'Eks_:_: j,u:_|'Es_! w'aI\n",
        ),
        (
            r#"before <say-as interpret-as="characters">ab</say-as> after"#,
            "bɪfˌɔː ˌeɪbˈiː ˈaftə\n",
            "bI2f,o@_:_: ,eI_|b'i:_! 'aaft3\n",
        ),
    ]);
}

#[test]
fn single_characters_and_glyphs_pause_after_every_letter() {
    check(&[
        (r#"<say-as interpret-as="tts:char">abc</say-as>"#, "ˌeɪbˌiːsˈiː\n", ",eI_!b,i:_!s'i:_!\n"),
        (r#"<say-as interpret-as="tts:char">a b</say-as>"#, "ˈeɪ bˈiː\n", "'eI_!_:_: b'i:_!\n"),
        (r#"<say-as interpret-as="characters" format="glyphs">ab</say-as>"#, "ˌeɪbˈiː\n", ",eI_!b'i:_!\n"),
        (
            r#"<say-as interpret-as="characters" format="glyphs">Ab</say-as>"#,
            "kˌapətəlˌeɪbˈiː\n",
            "k,ap@-t@L,eI_!b'i:_!\n",
        ),
    ]);
}

#[test]
fn digits_without_detail_are_spoken_one_by_one_and_joined() {
    check(&[
        (
            r#"<say-as interpret-as="tts:digits">1234</say-as>"#,
            "wˈɒntˈuːθɹˈiːfˈɔː\n",
            "w'0nt'u:_!Tr'i:f'o@_!\n",
        ),
        (r#"<say-as interpret-as="tts:digits">42</say-as>"#, "fˈɔːtˈuː\n", "f'o@t'u:_!\n"),
        (
            r#"<say-as interpret-as="telephone">555 1234</say-as>"#,
            "fˈaɪvfˈaɪvfˈaɪv wˈɒntˈuːθɹˈiːfˈɔː\n",
            "f'aIvf'aIv_!f'aIv w'0nt'u:_!Tr'i:f'o@_!\n",
        ),
        (
            r#"<say-as interpret-as="tts:digits">12</say-as> and <say-as interpret-as="telephone">34</say-as>"#,
            "wˈɒntˈuː and θɹˈiːfˈɔː\n",
            "w'0nt'u:_! and Tr'i:f'o@_!\n",
        ),
        (
            r#"<say-as interpret-as="telephone">+1 (555) 123-4567</say-as>"#,
            "plˈʌs wˈɒn fˈaɪvfˈaɪvfˈaɪv wˈɒntˈuːθɹˈiː dˈaʃ fˈɔːfˈaɪvsˈɪkssˈɛvən\n",
            "pl'Vs w'0n_:_: f'aIvf'aIv_!f'aIv_:_: w'0nt'u:_!Tr'i: d'aS f'o@f'aIv_!s'Ikss'Ev@n_!\n",
        ),
    ]);
}

#[test]
fn digits_with_detail_are_read_in_groups_of_that_size() {
    check(&[
        (
            r#"<say-as interpret-as="tts:digits" detail="2">123456</say-as>"#,
            "twˈɛlv θˈɜːti fˈɔː fˈɪfti sˈɪks\n",
            "tw'Elv T'3:ti f'o@ f'Ifti s'Iks\n",
        ),
        (
            r#"<say-as interpret-as="tts:digits" detail="3">1234567</say-as>"#,
            "wˈɒnhˈʌndɹɪdən twˈɛnti θɹˈiː fˈɔːhˈʌndɹɪdən fˈɪfti sˈɪks sˈɛvən\n",
            "w'0nh'VndrI2d@n tw'Enti Tr'i: f'o@h'VndrI2d@n f'Ifti s'Iks s'Ev@n\n",
        ),
        (
            r#"<say-as interpret-as="tts:digits" detail="2">12 345</say-as>"#,
            "twˈɛlv θˈɜːti fˈɔː fˈaɪv\n",
            "tw'Elv T'3:ti f'o@ f'aIv\n",
        ),
    ]);
}

#[test]
fn key_names_are_spelled_when_one_character_long() {
    check(&[
        (r#"<say-as interpret-as="tts:key">a</say-as>"#, "ˈeɪ\n", "'eI_!\n"),
        (r#"<say-as interpret-as="tts:key">A</say-as>"#, "ˈeɪ\n", "'eI_!\n"),
        (r#"<say-as interpret-as="tts:key">7</say-as>"#, "sˈɛvən\n", "s'Ev@n_!\n"),
        (r#"<say-as interpret-as="tts:key">space</say-as>"#, "spˈeɪs\n", "sp'eIs_!\n"),
        (r#"<say-as interpret-as="tts:key">tab</say-as>"#, "tˈab\n", "t'ab_!\n"),
        (r#"<say-as interpret-as="tts:key">underscore</say-as>"#, "ˈʌndəskˌɔː\n", "'Vnd3sk,o@_!\n"),
        (r#"<say-as interpret-as="tts:key">double-quote</say-as>"#, "kwˈəʊts\n", "kw'oUts_!\n"),
        (r#"<say-as interpret-as="tts:key">enter</say-as>"#, "ˈɛntə\n", "'Ent3\n"),
        (r#"<say-as interpret-as="tts:key">ab</say-as>"#, "ˈab\n", "'ab\n"),
        (r#"<say-as interpret-as="tts:key">'abk</say-as>"#, "ˈabk\n", "'abk\n"),
    ]);
}

#[test]
fn say_as_modes_have_the_c_duration() {
    check_samples(&[
        (r#"<say-as interpret-as="tts:digits">1234</say-as>"#, 28611),
        (r#"<say-as interpret-as="tts:digits">42</say-as>"#, 19268),
        (r#"<say-as interpret-as="tts:digits" detail="2">123456</say-as>"#, 44814),
        (r#"<say-as interpret-as="tts:digits" detail="3">1234567</say-as>"#, 76773),
        (r#"<say-as interpret-as="tts:digits" detail="2">12 345</say-as>"#, 37438),
        (r#"<say-as interpret-as="telephone">555 1234</say-as>"#, 49322),
        (r#"<say-as interpret-as="telephone">+1 (555) 123-4567</say-as>"#, 93556),
        (r#"<say-as interpret-as="tts:digits">12</say-as> and <say-as interpret-as="telephone">34</say-as>"#, 34723),
        (r#"<say-as interpret-as="tts:key">a</say-as>"#, 12700),
        (r#"<say-as interpret-as="tts:key">7</say-as>"#, 17161),
        (r#"<say-as interpret-as="tts:key">underscore</say-as>"#, 20617),
        (r#"<say-as interpret-as="tts:key">double-quote</say-as>"#, 19385),
        (r#"<say-as interpret-as="tts:key">enter</say-as>"#, 14426),
        (r#"<say-as interpret-as="tts:key">ab</say-as>"#, 12656),
    ]);
}

#[test]
fn spelled_characters_have_the_c_duration() {
    check_samples(&[
        (r#"<say-as interpret-as="characters">abc</say-as>"#, 19996),
        (r#"<say-as interpret-as="characters">ab cd</say-as>"#, 27745),
        (r#"<say-as interpret-as="tts:char">abc</say-as>"#, 20508),
        (r#"<say-as interpret-as="tts:char">a b</say-as>"#, 19888),
        (r#"<say-as interpret-as="characters" format="glyphs">ab</say-as>"#, 16468),
        (r#"<say-as interpret-as="characters" format="glyphs">Ab</say-as>"#, 26181),
        (r#"x <say-as interpret-as="characters">US</say-as> y"#, 31029),
    ]);
}
