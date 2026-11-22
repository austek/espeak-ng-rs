// tests/cli_phonemes_with_wav.rs
//
// `-x`, `--ipa` and `-X` combined with `-w`: C prints the phoneme text
// *and* writes the same WAV as without them; `-q` silences the WAV (no file at
// all) but not `-x`/`--ipa`.
//
// Expected values come from a C espeak-ng built from f0fb11a8: FNV-1a 64 over
// the whole file, and for `-w stdout` the phoneme text followed by the WAV.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_espeak_cli")
}

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-pw-{}-{n}-{name}", std::process::id()))
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn have_data() -> bool {
    Path::new("espeak-ng-data/en_dict").exists()
}

fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().expect("run")
}

/// Run `args` with `-w <tmp file>`; return stdout text and the file's bytes, if any.
fn with_file(args: &[&str]) -> (String, Option<Vec<u8>>) {
    let path = tmp("out.wav");
    let out = Command::new(bin()).args(args).arg("-w").arg(&path).output().expect("run");
    assert!(out.status.success());
    let bytes = std::fs::read(&path).ok();
    let _ = std::fs::remove_file(&path);
    (String::from_utf8(out.stdout).unwrap(), bytes)
}

fn assert_wav(args: &[&str], text: &str, phonemes: &str, len: usize, hash: u64) {
    let (stdout, wav) = with_file(args);
    let wav = wav.unwrap_or_else(|| panic!("no WAV written for {args:?}"));
    assert_eq!(stdout, phonemes, "{args:?}");
    assert_eq!((wav.len(), fnv(&wav)), (len, hash), "{args:?} {text}");
}

#[test]
fn x_with_w_writes_the_same_wav_per_language() {
    if !have_data() {
        return;
    }
    let cases: [(&str, &str, &str, usize, u64); 4] = [
        ("en", "hello world", "h@l'oU w'3:ld\n", 45394, 0xc422_762e_6983_c708),
        ("de", "guten Tag", "g'u:t@n t'A:k\n", 44184, 0x19e8_901c_e34b_a192),
        ("fr", "bonjour le monde", "bO~Z'ur l@- m'O~d\n", 45880, 0x1fa7_c935_b679_dedd),
        ("ru", "привет мир", "p@-r;iv;'et m;'ir\n", 42890, 0xd8f4_a321_00dd_dd4d),
    ];
    for (lang, text, ph, len, hash) in cases {
        assert_wav(&["-v", lang, "-x", text], text, ph, len, hash);
    }
}

#[test]
fn ipa_with_w_writes_the_same_wav() {
    if !have_data() {
        return;
    }
    assert_wav(&["--ipa", "hello world"], "hello world", "həlˈəʊ wˈɜːld\n", 45394, 0xc422_762e_6983_c708);
    assert_wav(&["-v", "de", "--ipa", "guten Tag"], "guten Tag", "ɡˈuːtən tˈɑːk\n", 44184, 0x19e8_901c_e34b_a192);
}

#[test]
fn x_and_ipa_wav_matches_plain_wav_across_clauses() {
    if !have_data() {
        return;
    }
    assert_wav(&["-x", "hi. there"], "hi. there", "h'aI D'e@\n", 39208, 0xa975_e1b9_c2da_6df0);
    assert_wav(&["--ipa", "hi. there"], "hi. there", "hˈaɪ ðˈeə\n", 39208, 0xa975_e1b9_c2da_6df0);
}

#[test]
fn trace_and_ipa_tie_with_w_still_write_the_wav() {
    if !have_data() {
        return;
    }
    let (_, wav) = with_file(&["-X", "hi"]);
    let wav = wav.expect("-X -w writes a WAV");
    assert_eq!((wav.len(), fnv(&wav)), (27952, 0xa93c_f48e_7496_5d0b));
    let (out, wav) = with_file(&["--ipa=1", "hi"]);
    assert_eq!(out, "h_ˈaɪ\n");
    assert_eq!(fnv(&wav.expect("--ipa=1 -w writes a WAV")), 0xa93c_f48e_7496_5d0b);
}

#[test]
fn quiet_keeps_the_phonemes_and_writes_no_file() {
    if !have_data() {
        return;
    }
    for opt in ["-x", "--ipa"] {
        let (out, wav) = with_file(&["-q", opt, "hi"]);
        assert!(out.starts_with('h'), "{opt}: {out}");
        assert!(wav.is_none(), "-q {opt} -w must not create the file");
    }
    let (_, wav) = with_file(&["-q", "hi"]);
    assert!(wav.is_none());
}

#[test]
fn stdout_carries_the_phonemes_then_the_wav() {
    if !have_data() {
        return;
    }
    let plain = run(&["-w", "stdout", "hi. there"]).stdout;
    assert_eq!((plain.len(), fnv(&plain)), (39208, 0x2b46_1788_c3ad_cb13));
    for (opt, text) in [("-x", "h'aI D'e@\n"), ("--ipa", "hˈaɪ ðˈeə\n")] {
        let out = run(&[opt, "-w", "stdout", "hi. there"]).stdout;
        assert!(out.starts_with(text.as_bytes()), "{opt}");
        let wav = &out[text.len()..];
        assert_eq!((wav.len(), fnv(wav)), (39208, 0x2b46_1788_c3ad_cb13), "{opt}");
    }
    let quiet = run(&["-x", "-q", "-w", "stdout", "hi"]).stdout;
    assert_eq!(quiet, b"h'aI\n");
}

#[test]
fn breath_voice_wav_has_the_plain_length() {
    if !have_data() {
        return;
    }
    let (_, wav) = with_file(&["-v", "en+whisper", "-x", "hello world"]);
    assert_eq!(wav.expect("WAV written").len(), 45414);
}

#[test]
fn unwritable_wav_target_with_x_reports_and_exits_zero() {
    if !have_data() {
        return;
    }
    let out = run(&["-x", "-w", "/nonexistent-dir/x.wav", "hi"]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "h'aI\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("Can't write to: '/nonexistent-dir/x.wav'"));
}

// ---------------------------------------------------------------------------
// Per-clause interleaving on stdout, `--pho -w`, and one synthesis per stdin
// line. Expected (length, FNV-1a 64) come from C f0fb11a8.
// ---------------------------------------------------------------------------

fn run_with_stdin(args: &[&str], stdin: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(bin())
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().expect("wait")
}

fn assert_stdout(args: &[&str], stdin: &str, len: usize, hash: u64) {
    let out = run_with_stdin(args, stdin).stdout;
    assert_eq!((out.len(), fnv(&out)), (len, hash), "{args:?}");
}

/// Run `args -w <file>` with `stdin`; the file's bytes, if the run wrote one.
fn file_with_stdin(args: &[&str], stdin: &str) -> (String, Option<Vec<u8>>) {
    let path = tmp("stdin.wav");
    let mut full = args.to_vec();
    let target = path.to_string_lossy().into_owned();
    full.extend(["-w", &target]);
    let out = run_with_stdin(&full, stdin);
    let bytes = std::fs::read(&path).ok();
    let _ = std::fs::remove_file(&path);
    (String::from_utf8(out.stdout).unwrap(), bytes)
}

fn assert_file(args: &[&str], stdin: &str, len: usize, hash: u64) {
    let (_, wav) = file_with_stdin(args, stdin);
    let wav = wav.unwrap_or_else(|| panic!("no WAV for {args:?}"));
    assert_eq!((wav.len(), fnv(&wav)), (len, hash), "{args:?}");
}

const EN_CLAUSES: &str = "Hello world. This is a test, with two clauses!";

#[test]
fn stdout_places_each_clauses_phonemes_before_its_audio() {
    if !have_data() {
        return;
    }
    let out = run(&["-x", "-w", "stdout", EN_CLAUSES]).stdout;
    assert_eq!(out.len(), 143437);
    assert_eq!(&out[..14], b"h@l'oU w'3:ld\n");
    assert_eq!(&out[14..18], b"RIFF");
    assert_eq!(&out[45408..45408 + 16], b"DIs Iz a# t'Est\n");
    assert_eq!(&out[83650..83650 + 19], b"wID t'u: kl'O:zI2z\n");
}

#[test]
fn stdout_interleaving_matches_c_across_languages_and_formats() {
    if !have_data() {
        return;
    }
    let de = "Guten Tag. Wie geht es dir, mein Freund!";
    let fr = "Bonjour le monde. Ceci est un test!";
    let ru = "Привет мир. Это тест, с двумя клаузами!";
    let ar = "مرحبا بالعالم. هذا اختبار، شكرا!";
    let cmn = "你好世界。这是一个测试，谢谢！";
    let rows: [(&[&str], &str, usize, u64); 16] = [
        (&["-v", "en", "-x"], EN_CLAUSES, 143437, 0xadb3_a0ef_698f_bb2a),
        (&["-v", "en", "--ipa"], EN_CLAUSES, 143456, 0xf322_a70c_c644_aec3),
        (&["-v", "en", "--ipa=1"], EN_CLAUSES, 143476, 0x0685_c05b_b3f4_bc29),
        (&["-v", "en", "--ipa=3"], EN_CLAUSES, 143459, 0x4da4_8e97_1899_2f1e),
        (&["-v", "en", "--sep=_", "-x"], EN_CLAUSES, 143457, 0xbeb8_5f05_680a_9610),
        (&["-v", "en", "--sep=_", "--ipa"], EN_CLAUSES, 143476, 0x0685_c05b_b3f4_bc29),
        (&["-v", "de", "-x"], de, 131860, 0x04a2_45e0_54a5_6e89),
        (&["-v", "de", "--ipa"], de, 131877, 0x1309_f30f_db7a_5f64),
        (&["-v", "fr", "-x"], fr, 99586, 0xcbca_d39d_5919_1cd4),
        (&["-v", "fr", "--ipa"], fr, 99600, 0xffcc_e094_e9a6_1215),
        (&["-v", "ru", "-x"], ru, 122294, 0x6355_8678_c4ed_b7eb),
        (&["-v", "ru", "--ipa"], ru, 122310, 0x35d5_29d1_ea3a_b9fc),
        (&["-v", "ar", "-x"], ar, 172093, 0x3a7f_3b8c_0a9e_4320),
        (&["-v", "ar", "--ipa"], ar, 172110, 0x7d51_e4dc_7677_ea03),
        (&["-v", "cmn", "-x"], cmn, 162235, 0xa0bb_6485_7b1c_5b6f),
        (&["-v", "cmn", "--ipa"], cmn, 162220, 0x0813_3096_1f76_a2d2),
    ];
    for (opts, text, len, hash) in rows {
        let mut args = opts.to_vec();
        args.extend(["-w", "stdout", text]);
        let out = run(&args).stdout;
        assert_eq!((out.len(), fnv(&out)), (len, hash), "{opts:?} {text}");
    }
}

#[test]
fn pho_with_w_prints_nothing_and_writes_the_plain_wav() {
    if !have_data() {
        return;
    }
    let (out, wav) = with_file(&["--pho", "hello world"]);
    assert_eq!(out, "");
    let wav = wav.expect("--pho -w writes a WAV");
    assert_eq!((wav.len(), fnv(&wav)), (45394, 0xc422_762e_6983_c708));
    assert_stdout(&["-v", "en", "--pho", "-w", "stdout", EN_CLAUSES], "", 143388, 0x2ad2_ed16_3a83_0b4e);
}

#[test]
fn pho_with_x_prints_the_phonemes_and_writes_the_wav() {
    if !have_data() {
        return;
    }
    assert_stdout(&["-v", "en", "--pho", "-x", "-w", "stdout", EN_CLAUSES], "", 143437, 0xadb3_a0ef_698f_bb2a);
    let (out, wav) = with_file(&["-v", "en", "--pho", "-x", EN_CLAUSES]);
    assert_eq!(out, "h@l'oU w'3:ld\nDIs Iz a# t'Est\nwID t'u: kl'O:zI2z\n");
    let wav = wav.expect("WAV written");
    assert_eq!((wav.len(), fnv(&wav)), (143388, 0x982c_0dea_ecad_c29f));
}

const EN_LINES: &str = "hello world\ngood morning\nthank you\n";
const DE_LINES: &str = "guten Tag\nwie geht es\ndanke schoen\n";
const FR_LINES: &str = "bonjour le monde\nbonne nuit\nmerci\n";
const RU_LINES: &str = "привет мир\nдоброе утро\nспасибо\n";

#[test]
fn each_stdin_line_is_its_own_synthesis_for_the_wav_file() {
    if !have_data() {
        return;
    }
    assert_file(&["-v", "en"], EN_LINES, 125868, 0x4063_6405_f69a_6b8d);
    assert_file(&["-v", "de"], DE_LINES, 128530, 0x45de_238e_91b9_2a91);
    assert_file(&["-v", "fr"], FR_LINES, 107812, 0x19c4_d5ee_c8c3_8afd);
    assert_file(&["-v", "ru"], RU_LINES, 111516, 0x0e53_4458_0bd9_03ad);
    assert_file(&["-v", "en", "-x"], EN_LINES, 125868, 0x4063_6405_f69a_6b8d);
    assert_file(&["-v", "de", "-x"], DE_LINES, 128530, 0x45de_238e_91b9_2a91);
    assert_file(&["-v", "fr", "-x"], FR_LINES, 107812, 0x19c4_d5ee_c8c3_8afd);
    assert_file(&["-v", "ru", "-x"], RU_LINES, 111516, 0x0e53_4458_0bd9_03ad);
}

#[test]
fn stdin_line_phonemes_print_once_per_line_in_order() {
    if !have_data() {
        return;
    }
    let (out, _) = file_with_stdin(&["-v", "en", "-x"], EN_LINES);
    assert_eq!(out, "h@l'oU w'3:ld\ng'Ud m'O@nIN\nT'aNk ju:\n");
    let (out, _) = file_with_stdin(&["-v", "de", "-x"], DE_LINES);
    assert_eq!(out, "g'u:t@n t'A:k\nvi: g'e:t _|Es\nd'aNk@ S'Y:n\n");
}

#[test]
fn stdin_lines_on_stdout_alternate_phonemes_and_audio() {
    if !have_data() {
        return;
    }
    assert_stdout(&["-v", "en", "-w", "stdout"], EN_LINES, 125868, 0x80bd_5d34_57b5_2d65);
    assert_stdout(&["-v", "en", "-x", "-w", "stdout"], EN_LINES, 125905, 0x536c_a989_841e_ad98);
    assert_stdout(&["-v", "en", "--ipa", "-w", "stdout"], EN_LINES, 125924, 0x883e_a6c5_0e05_41ab);
    assert_stdout(&["-v", "de", "-x", "-w", "stdout"], DE_LINES, 128572, 0xc4f1_2070_51c1_ba4b);
    assert_stdout(&["-v", "fr", "-x", "-w", "stdout"], FR_LINES, 107847, 0xca5f_1ef6_3a8e_16df);
    assert_stdout(&["-v", "ru", "-x", "-w", "stdout"], RU_LINES, 111563, 0xdfb7_a139_5b66_6af8);
    assert_stdout(&["-v", "de", "-w", "stdout"], DE_LINES, 128530, 0xc6de_4301_cb74_f00c);
}

#[test]
fn blank_and_unterminated_stdin_lines_follow_fgets() {
    if !have_data() {
        return;
    }
    let input = "one two\n\nthree four.\nfive";
    assert_file(&[], input, 109032, 0xad6a_7b15_3225_b06c);
    let (out, wav) = file_with_stdin(&["-x"], input);
    assert_eq!(out, "w'0n t'u:\n\nTr'i: f'o@\nf'aIv\n");
    assert_eq!(wav.expect("WAV").len(), 109032);
    assert_stdout(&["-x", "-w", "stdout"], input, 109060, 0x3806_9e99_0f9d_2c11);
}

#[test]
fn a_line_longer_than_fgets_reads_is_split_like_c() {
    if !have_data() {
        return;
    }
    let long = format!("{}end\nnext line\n", "Hi there. ".repeat(200));
    assert_file(&[], &long, 7903860, 0x5172_0fcf_7071_d56b);
    assert_file(&["-x"], &long, 7903860, 0x5172_0fcf_7071_d56b);
}

#[test]
fn unwritable_target_prints_each_inputs_first_line_and_reports_each_time() {
    if !have_data() {
        return;
    }
    let msg = "Can't write to: '/nonexistent-dir/x.wav'\n";
    let bad = ["-w", "/nonexistent-dir/x.wav"];
    let run_bad = |opts: &[&str], stdin: &str| {
        let mut args = opts.to_vec();
        args.extend(bad);
        let out = run_with_stdin(&args, stdin);
        (String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap())
    };
    assert_eq!(run_bad(&["-x", EN_CLAUSES], ""), ("h@l'oU w'3:ld\n".to_string(), msg.to_string()));
    let two = "hello world\ngood morning\n";
    assert_eq!(
        run_bad(&["-x"], two),
        ("h@l'oU w'3:ld\ng'Ud m'O@nIN\n".to_string(), format!("{msg}{msg}"))
    );
    assert_eq!(run_bad(&[], two), (String::new(), format!("{msg}{msg}")));
}

#[test]
fn empty_stdin_makes_no_synthesis_call_so_no_file_and_no_output() {
    if !have_data() {
        return;
    }
    let variants: [&[&str]; 10] = [
        &[],
        &["-x"],
        &["--ipa"],
        &["--ipa=1"],
        &["--sep=_", "-x"],
        &["-X"],
        &["--pho"],
        &["--pho", "-x"],
        &["-x", "--stdin"],
        &["-m", "-x"],
    ];
    for opts in variants {
        let (out, wav) = file_with_stdin(opts, "");
        assert_eq!(out, "", "{opts:?}");
        assert!(wav.is_none(), "{opts:?} created a file");
    }
    for target in [&["--stdout"][..], &["-w", "stdout"][..]] {
        let mut args = vec!["-x"];
        args.extend(target);
        assert_eq!(run_with_stdin(&args, "").stdout, b"", "{args:?}");
    }
    let out = run_with_stdin(&["-x", "-w", "/nonexistent-dir/x.wav"], "");
    assert_eq!((out.stdout.len(), out.stderr.len()), (0, 0));
    let out = run_with_stdin(&["-w", "/nonexistent-dir/x.wav"], "");
    assert_eq!((out.stdout.len(), out.stderr.len()), (0, 0));
}

#[test]
fn an_empty_argument_is_still_one_synthesis() {
    if !have_data() {
        return;
    }
    let (out, wav) = with_file(&["-x", ""]);
    assert_eq!(out, "\n");
    let wav = wav.expect("an empty text still opens the WAV");
    assert_eq!((wav.len(), fnv(&wav)), (352, 0x1952_cab4_3a67_922a));
    assert_stdout(&["-x", "-w", "stdout", ""], "", 353, 0xad88_a4c7_d2dd_a94e);
}

#[test]
fn markup_phonemes_follow_each_clauses_audio_on_stdout() {
    if !have_data() {
        return;
    }
    let text = "Hello world. This is a test.";
    let out = run(&["-x", "-m", "-w", "stdout", text]).stdout;
    assert_eq!((out.len(), fnv(&out)), (90576, 0x9a14_11a6_bb25_fc9a));
    assert_eq!(&out[45408..45408 + 16], b"DIs Iz a# t'Est\n");
    let ssml = "<speak>Hello world. This is a test.</speak>";
    assert_stdout(&["-x", "-m", "-w", "stdout", ssml], "", 90885, 0xbc7f_c2fe_d1a6_4fb0);
    assert_stdout(&["-m", "-w", "stdout", text], "", 90546, 0x94c9_5490_c0b5_4948);
}

#[test]
fn markup_stdin_lines_carry_the_synthesizer_state() {
    if !have_data() {
        return;
    }
    let two = "hello world\ngood morning\n";
    assert_file(&["-m"], two, 87420, 0x754c_9cd4_5b70_2492);
    let (out, wav) = file_with_stdin(&["-m", "-x"], two);
    assert_eq!(out, "h@l'oU w'3:ld\ng'Ud m'O@nIN\n");
    assert_eq!(fnv(&wav.expect("WAV")), 0x754c_9cd4_5b70_2492);
    assert_stdout(&["-m", "-x", "-w", "stdout"], two, 87447, 0x270e_09bc_6f89_1623);
    let ssml = "<speak>there. you</speak>\n<speak>Hi</speak>\n";
    assert_file(&["-m"], ssml, 62068, 0xaea0_5540_4059_c443);
    assert_stdout(&["-m", "-x", "-w", "stdout"], ssml, 62084, 0x22bf_8fde_c3ab_86b9);
}

#[test]
fn split_sends_the_wav_to_a_numbered_file_not_to_stdout() {
    if !have_data() {
        return;
    }
    let dir = tmp("split-dir");
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new(bin())
        .args(["--split=1", "-x", "-w", "stdout", "hello world"])
        .env("ESPEAK_DATA_PATH", std::fs::canonicalize("espeak-ng-data").unwrap())
        .current_dir(&dir)
        .output()
        .expect("run");
    assert_eq!(out.stdout, b"h@l'oU w'3:ld\n");
    let part = std::fs::read(dir.join("stdout_01")).expect("stdout_01 written");
    assert_eq!((part.len(), fnv(&part)), (45394, 0xc422_762e_6983_c708));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn trace_with_w_matches_c_for_a_single_clause_word() {
    if !have_data() {
        return;
    }
    assert_stdout(&["-X", "-w", "stdout", "hello"], "", 31672, 0xc20f_2b72_ee47_156a);
    let (out, wav) = with_file(&["-X", "hello"]);
    assert_eq!(out.len(), 32);
    let wav = wav.expect("WAV");
    assert_eq!((wav.len(), fnv(&wav)), (31640, 0xb542_7ca8_7ab5_ead9));
}

#[test]
fn bulk_stdin_is_one_synthesis() {
    if !have_data() {
        return;
    }
    let input = "hello world\ngood morning\n";
    let (out, wav) = file_with_stdin(&["-x", "--stdin"], input);
    assert_eq!(out, "h@l'oU w'3:ld g'Ud m'O@nIN\n");
    let wav = wav.expect("WAV");
    assert_eq!((wav.len(), fnv(&wav)), (71010, 0xd8ba_42bf_13bc_dc8a));
}
