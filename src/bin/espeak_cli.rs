// Shared CLI implementation for `espeak-ng` and `espeak-ng-rs` binaries.

use std::ffi::OsString;
use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};

use espeak_ng::engine::{EspeakNg, Parameter};
use espeak_ng::getopt::{self, HasArg, LongOpt};
use espeak_ng::phoneme::PhonemeData;
use espeak_ng::translate::{default_data_dir, PhonemeCode, Translator};
use espeak_ng::voices::{listed_voices, resolve_voice};

#[derive(Debug, Default)]
struct Cli {
    voice:         String,
    quiet:         bool,
    ipa:           bool,
    x_mnem:        bool,
    markup:        bool,
    path_override: Option<PathBuf>,
    phonout:       Option<PathBuf>,
    sep:           Option<char>,
    tie:           Option<char>,
    stdout_wav:    bool,
    wave_out:      Option<PathBuf>,
    print_help:    bool,
    print_version: bool,
    rate:          Option<u32>,
    pitch:         Option<u32>,
    range:         Option<u32>,
    amplitude:     Option<u32>,
    capitals:      Option<u32>,
    word_gap:      Option<u32>,
    file_input:    Option<PathBuf>,
    stdin_all:     bool,
    list_voices:   bool,
    voices_filter: Option<String>,
    /// `--punct` (announce all punctuation) / `--punct="chars"` (only those).
    punct:         Option<Vec<char>>,
    /// `-z`: no final sentence pause (suppress end-of-utterance silence).
    no_final_pause: bool,
    /// `-D`: deterministic random mode — pin the noise seed to upstream's value.
    deterministic: bool,
    /// `-X`: trace the translation rules that match each letter.
    trace_rules: bool,
    /// `-b <n>`: input text encoding for `-f`/`--stdin` (1=UTF-8, 2=8-bit, 4=16-bit).
    input_encoding: Option<u32>,
    /// `--compile[=<lang>]` / `--compile-debug[=<lang>]`: compile `<lang>`'s
    /// dictsource (`<data>/<lang>_list`+`_rules`) into the binary `<lang>_dict`.
    /// `Some("")` = compile the `-v` voice's language.
    compile: Option<String>,
    /// `--compile-phonemes[=<source>]`: compile a phoneme-table source file into
    /// the binary `phontab`.  `Some("")` = the data dir's `phonemes` file.
    compile_phonemes: Option<String>,
    /// `--compile-mbrola=<source>`: compile an MBROLA phoneme-translation source
    /// into `<data>/mbrola_ph/<name>_phtrans`.
    compile_mbrola: Option<String>,
    /// `--pho`: write the MBROLA `.pho` phoneme/duration/pitch script instead of
    /// speaking.
    pho: bool,
    /// `--split[=<minutes>]`: start a new WAV file at the first sentence
    /// boundary after this many minutes of audio.  `Some(0)` never splits
    /// (upstream's `atoi` of a non-number); the bare option means 30 minutes.
    split_minutes: Option<u32>,
    /// `--load`: `-v` names a voice *file* to read, not an installed voice.
    load_voice: bool,
    /// The `-v` value exactly as given.  `voice` is normalized as a language
    /// tag (lower-cased, `_` → `-`), which would corrupt the file path
    /// `--load` expects.
    voice_raw:     Option<String>,
    /// `--ssml-break=<percentage>`: scale every SSML `<break time=…>`.
    ssml_break: Option<u32>,
    /// `-d <device>`: audio output device.  Accepted for parity; this port has
    /// no playback, so it is never used.
    device: Option<String>,
    positionals:   Vec<OsString>,
    /// A message for stderr and the exit status, set where C leaves `getopt`.
    error_exit:    Option<(String, i32)>,
    /// Set by `--tie` and `--ipa=2|3`; only then does `sep` hold a tie.
    tie_mode:      bool,
    /// `-l <n>`: accepted for parity; this port has no line-length clauses.
    line_length:   Option<u32>,
    /// `--compile-intonations`.
    compile_intonations: bool,
}

fn argv0_display() -> String {
    std::env::args_os()
        .next()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "espeak-ng".to_string())
}

/// The version `espeak_Info` reports in the C reference this port tracks.
const REFERENCE_VERSION: &str = "1.53.0";

fn version_line(data_dir: &Path) -> String {
    format!(
        "eSpeak NG text-to-speech: {REFERENCE_VERSION}  Data at: {}",
        data_dir.display()
    )
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> std::io::Result<()> {
    let mut cli = parse_args(std::env::args_os().skip(1).collect());
    if let Some((message, status)) = cli.error_exit.take() {
        eprintln!("{message}");
        std::process::exit(status);
    }
    if cli.print_help {
        println!("\n{}", version_line(&resolved_data_dir(cli.path_override.as_deref())));
        print!("{HELP_TEXT}");
        return Ok(());
    }
    if cli.print_version {
        println!("{}", version_line(&resolved_data_dir(cli.path_override.as_deref())));
        return Ok(());
    }
    if cli.list_voices {
        print_voices(&cli);
        return Ok(());
    }
    if cli.compile_intonations {
        return run_compile_intonations(&cli);
    }
    if let Some(src_arg) = cli.compile_phonemes.clone() {
        return run_compile_phonemes(&src_arg, &cli);
    }
    if let Some(src_arg) = cli.compile_mbrola.clone() {
        return run_compile_mbrola(&src_arg, &cli);
    }
    // `--load`: `-v` is the path to a voice definition, not an installed voice.
    // Its `language` (or `dictionary`) picks the translation, and its `pitch`
    // line reaches the synthesizer below; the rest of a language's settings
    // still come from the data directory.
    create_phonout(&cli);
    let data = resolved_data_dir(cli.path_override.as_deref());
    if !data.join("phontab").exists() {
        eprintln!("Error processing file '{}': No such file or directory.", data.join("phontab").display());
        std::process::exit(1);
    }
    let text_outputs = cli.trace_rules || cli.pho || cli.ipa || cli.x_mnem;
    if plays_audio(&cli) && !text_outputs {
        fail_with(ENS_AUDIO_ERROR);
    }
    if let Some(lang) = cli.compile.as_deref() {
        cli.voice = lang.to_ascii_lowercase();
    }
    let mut loaded_pitch = None;
    let mut loaded_voice = None;
    if cli.load_voice {
        let path = PathBuf::from(cli.voice_raw.as_deref().unwrap_or(&cli.voice));
        match espeak_ng::voices::load_voice_file(&path) {
            Some(v) => {
                loaded_pitch = v.pitch;
                loaded_voice = Some(v.clone());
                let data = resolved_data_dir(cli.path_override.as_deref());
                let want = v.dictionary.clone().unwrap_or_else(|| v.language.clone());
                cli.voice = espeak_ng::translate::resolve_dict_stem(&data, &want)
                    .unwrap_or(want);
            }
            None => fail_with(ENS_VOICE_NOT_FOUND),
        }
    } else {
        // Resolve a voice *name* (e.g. "French (France)") to a language tag; a
        // plain language tag is left unchanged.
        let data = resolved_data_dir(cli.path_override.as_deref());
        let spelled = cli.voice_raw.clone().unwrap_or_else(|| cli.voice.clone());
        let wanted = match espeak_ng::voices::voice_file_entry(&data, split_variant_base(&spelled)) {
            Some(v) => {
                loaded_pitch = v.pitch;
                loaded_voice = Some(v);
                spelled
            }
            None => with_spelled_variant(&cli.voice, &spelled),
        };
        let compiling_without_voices =
            cli.compile.is_some() && espeak_ng::voices::list_catalog(&data).is_empty();
        match resolve_voice(&wanted, &data) {
            Some(v) => cli.voice = v,
            None if compiling_without_voices => {}
            None => fail_with(ENS_VOICE_NOT_FOUND),
        }
    }
    if let Some(lang_arg) = cli.compile.clone() {
        return run_compile(&lang_arg, &cli);
    }
    if text_outputs && !cli.pho {
        if let Some(mb) = espeak_ng::voices::load_mbrola_voice(&data, &cli.voice) {
            cli.voice = espeak_ng::translate::resolve_dict_stem(&data, &mb.language)
                .unwrap_or(mb.language);
        }
    }

    let texts = gather_texts(&cli)?;
    let text = texts.concat();

    if cli.trace_rules || cli.pho || cli.ipa || cli.x_mnem {
        return texts.iter().try_for_each(|t| text_output(&cli, t));
    }

    if cli.quiet {
        return Ok(());
    }

    let data_dir = resolved_data_dir(cli.path_override.as_deref());

    // An `mb-*` voice synthesizes through the external MBROLA binary.  Neither
    // the binary nor the diphone databases ship here, so fall back to the
    // built-in formant synthesizer on the voice's own language when either is
    // missing — with a warning, rather than refusing to speak.
    if let Some(mb) = espeak_ng::voices::load_mbrola_voice(&data_dir, &cli.voice) {
        match speak_mbrola(&cli, &data_dir, &mb, &text) {
            Ok(()) => return Ok(()),
            Err(e) => {
                eprintln!(
                    "{}: MBROLA voice '{}' unavailable ({e}); using the built-in \
                     synthesizer for '{}'",
                    argv0_display(),
                    mb.database,
                    mb.language
                );
                // The voice file may name a locale the port has no dict for
                // (`en-uk`); fall back to whatever stem actually has data.
                cli.voice = espeak_ng::translate::resolve_dict_stem(&data_dir, &mb.language)
                    .unwrap_or_else(|| mb.language.clone());
            }
        }
    }

    let mut b = EspeakNg::builder().voice(&cli.voice).data_dir(&data_dir);
    if let Some(r) = cli.rate {
        b = b.rate(r);
    }
    if let Some(p) = cli.pitch {
        b = b.pitch(p);
    }
    if let Some(v) = cli.amplitude {
        b = b.volume(v);
    }
    if let Some(pr) = cli.range {
        b = b.range(pr);
    }
    let mut engine = b.build().map_err(|e| {
        std::io::Error::other(format!("{e}"))
    })?;
    apply_runtime_params(&mut engine, &cli);
    engine.set_voice_file(loaded_voice);
    if let Some(p) = loaded_pitch {
        engine.set_voice_pitch(Some(p));
    }
    // A document-level `<prosody>` (wrapping the whole utterance) sets the
    // global rate/pitch/volume for the audio.  Scoped prosody is left to the
    // (unaffected) phoneme output.
    if cli.markup {
        if let Some(p) = espeak_ng::translate::ssml::document_prosody(&text) {
            apply_prosody(&mut engine, p);
        }
    }

    // `--split=<minutes>`: a new WAV file at the first sentence boundary past
    // the limit.  Only for `-w` — a split stream makes no sense on stdout.
    if let (Some(minutes), Some(path)) =
        (cli.split_minutes.filter(|&m| m > 0), cli.wave_out.clone())
    {
        if !cli.stdout_wav {
            let (pcm, rate, events) = engine.synth_with_events(&text).map_err(|e| {
                std::io::Error::other(format!("{e}"))
            })?;
            return write_split_wavs(&path, &pcm, rate, &events, minutes);
        }
    }

    if cli.stdout_wav {
        return stream_wav(&engine, &text, wav_stdout_target());
    }
    if let Some(ref p) = cli.wave_out {
        return match std::fs::File::create(p) {
            Ok(file) => {
                let file = std::io::BufWriter::with_capacity(1 << 16, file);
                stream_wav(&engine, &text, Box::new(file) as Box<dyn WavOut>)
            }
            Err(_) => {
                eprintln!("Can't write to: '{}'", p.display());
                Ok(())
            }
        };
    }

    let _ = engine.synth(&text).map_err(std::io::Error::other)?;
    fail_with(ENS_AUDIO_ERROR)
}

/// C reports an unwritable `-w` target and still exits 0.
fn write_wave_file(path: &Path, bytes: &[u8]) {
    if std::fs::write(path, bytes).is_err() {
        eprintln!("Can't write to: '{}'", path.display());
    }
}

/// Speak through the external MBROLA binary: translate, build the `.pho`, hand
/// it to `mbrola <database>`, and write the PCM it returns.
///
/// Errors (a missing binary or database) are the caller's cue to fall back.
fn speak_mbrola(
    cli: &Cli,
    data_dir: &Path,
    mb: &espeak_ng::voices::MbrolaVoice,
    text: &str,
) -> std::io::Result<()> {
    use espeak_ng::mbrola::{self, write_pho, MbrolaTable};
    use espeak_ng::synthesize::{Synthesizer, VoiceParams};

    let ioerr = |e: espeak_ng::Error| std::io::Error::other(format!("{e}"));
    let database = mbrola::find_database(data_dir, &mb.database).map_err(ioerr)?;
    let bytes = std::fs::read(data_dir.join("mbrola_ph").join(&mb.phtrans))?;
    let table = MbrolaTable::parse(&bytes).map_err(ioerr)?;

    let mut translator = Translator::new(&mb.language, Some(data_dir)).map_err(ioerr)?;
    if let Some(g) = cli.word_gap {
        translator.options.word_gap = g as i32;
    }
    let codes = translator.translate_to_codes(text).map_err(ioerr)?;
    let mut phdata = PhonemeData::load(data_dir).map_err(ioerr)?;
    espeak_ng::translate::select_phoneme_table(
        &mut phdata,
        data_dir,
        translator.options.lang.as_str(),
    )
    .map_err(ioerr)?;

    let mut voice = VoiceParams::default();
    if let Some(r) = cli.rate {
        // Same mapping the engine uses (upstream's `speed_lookup` curve).
        let f = espeak_ng::synthesize::setlengths::speed_duration_factor(r);
        voice.speed_percent = ((100.0 / f).round() as u32).clamp(20, 800);
    }
    // Follow the MBROLA voice's own `pitch <base> <range>` (or the language's).
    if let Some((p1, p2)) = mb
        .pitch
        .or_else(|| espeak_ng::voices::voice_pitch(data_dir, &mb.language))
    {
        voice.pitch_hz = espeak_ng::synthesize::intonation::base_pitch_hz(
            p1 as f64,
            cli.pitch.unwrap_or(50),
        )
        .round() as u32;
        voice.pitch_range_units = (p2 - p1) as f64 * 108.0;
    }
    let rate = voice.sample_rate;
    let synth = Synthesizer::new(voice);
    let pho = write_pho(&table.translate(&synth.mbrola_phonemes(&codes, &phdata)));
    let pcm = mbrola::synthesize(&pho, &database).map_err(ioerr)?;
    if pcm.is_empty() {
        return Err(std::io::Error::other("mbrola produced no audio"));
    }

    if cli.stdout_wav || cli.wave_out.is_some() {
        let mut buf = Vec::new();
        write_wav_16_mono(&mut buf, &pcm, rate).map_err(std::io::Error::other)?;
        if cli.stdout_wav {
            std::io::stdout().write_all(&buf)?;
        } else if let Some(ref p) = cli.wave_out {
            write_wave_file(p, &buf);
        }
        return Ok(());
    }
    fail_with(ENS_AUDIO_ERROR)
}

/// Without `-q`, `-w` or `--stdout`, C plays the audio.
fn plays_audio(cli: &Cli) -> bool {
    !cli.quiet && cli.wave_out.is_none() && !cli.stdout_wav
}

/// One `espeak_Synth` call's phoneme output; `-q` silences `--pho`.
fn text_output(cli: &Cli, text: &str) -> std::io::Result<()> {
    if cli.trace_rules {
        trace_output(cli, text, cli.x_mnem || cli.ipa)?;
    }
    if cli.x_mnem || cli.ipa {
        phoneme_output(cli, text)?;
    }
    if cli.pho && !cli.quiet {
        pho_output(cli, text)?;
    }
    Ok(())
}

/// `fopen(--phonout, "w")` at start-up, so an empty result still leaves a file.
fn create_phonout(cli: &Cli) {
    if let Some(path) = cli.phonout.as_ref().filter(|p| std::fs::File::create(p).is_err()) {
        eprintln!("Can't write to: {}", path.display());
        std::process::exit(1);
    }
}

const ENS_VOICE_NOT_FOUND: &str = "The specified espeak-ng voice does not exist";
const ENS_AUDIO_ERROR: &str = "Cannot initialize the audio device";

/// C's `espeak_ng_PrintStatusCodeMessage` followed by `exit(EXIT_FAILURE)`.
fn fail_with(status_message: &str) -> ! {
    eprintln!("Error: {status_message}.");
    std::process::exit(1);
}

fn split_variant_base(value: &str) -> &str {
    espeak_ng::translate::split_voice_variant(value).0
}

/// `base`, with the `+variant` as the user spelled it: C splits the variant
/// off before it lowercases the voice name, and variant files are named in
/// mixed case (`!v/AnxiousAndy`).
fn with_spelled_variant(base: &str, spelled: &str) -> String {
    match espeak_ng::translate::split_voice_variant(spelled).1 {
        Some(v) => format!("{}+{v}", split_variant_base(base)),
        None => base.to_string(),
    }
}

/// `--compile-intonations`: C reads `<data>/../phsource/intonation`; this port
/// has no intonation compiler, so a present source is reported as unsupported.
fn run_compile_intonations(cli: &Cli) -> std::io::Result<()> {
    let data = resolved_data_dir(cli.path_override.as_deref());
    let source = data.join("../phsource/intonation");
    if source.exists() {
        eprintln!("Error: The requested functionality has not been built into espeak-ng.");
    } else {
        eprintln!(
            "Error processing file '{}': No such file or directory.",
            source.display()
        );
    }
    std::process::exit(1);
}

/// `--compile[=<lang>]`: compile `<lang>`'s dictsource (`<data>/<lang>_list` +
/// `<lang>_rules`) into the binary `<data>/<lang>_dict` — the `espeak-ng.c`
/// `--compile` path via `espeak_ng_CompileDictionary`.  `lang_arg` empty → the
/// `-v` voice's base language.  Phoneme mnemonics are resolved against that
/// language's phoneme table (falling back to `en`).
fn run_compile(lang_arg: &str, cli: &Cli) -> std::io::Result<()> {
    use espeak_ng::dictionary::compile::compile_dictionary;
    use espeak_ng::dictionary::transpose::TransposeConfig;

    let data_dir = resolved_data_dir(cli.path_override.as_deref());
    // Default to the -v voice's *base* language (strip any `+variant`).
    let lang = if !lang_arg.is_empty() {
        lang_arg.to_string()
    } else {
        cli.voice.split('+').next().unwrap_or("en").to_string()
    };

    // Refuse to silently write an empty dict when no dictsource exists (the
    // library layer is lenient about missing files; the CLI should not be).
    let list = data_dir.join(format!("{lang}_list"));
    let rules = data_dir.join(format!("{lang}_rules"));
    if !list.exists() && !rules.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "no dictsource for '{lang}' in {} (expected {lang}_list / {lang}_rules)",
                data_dir.display()
            ),
        ));
    }

    let mut phdata = PhonemeData::load(&data_dir).map_err(|e| {
        std::io::Error::other(format!("cannot load phoneme data: {e}"))
    })?;
    // Resolve mnemonics against the voice's phoneme table — honour a `phonemes
    // <name>` voice directive (e.g. bs→hr, nb→no) so a shared-table language
    // doesn't silently fall back to en and mis-resolve every mnemonic (`U:`→ts).
    let table = espeak_ng::translate::resolve_phoneme_table(&data_dir, &lang);
    if phdata.select_table_by_name(&table).is_err()
        && phdata.select_table_by_name(&lang).is_err()
    {
        phdata.select_table_by_name("en").map_err(|e| {
            std::io::Error::other(
                format!("no phoneme table for '{lang}' (voice table '{table}') or en: {e}"),
            )
        })?;
    }

    let transpose = TransposeConfig::for_lang(&lang);
    let bytes = compile_dictionary(&data_dir, &lang, &phdata, &transpose).map_err(|e| {
        std::io::Error::other(format!("compiling {lang}_dict: {e}"))
    })?;

    let out = data_dir.join(format!("{lang}_dict"));
    std::fs::write(&out, &bytes)?;
    eprintln!("Compiled {} → {} ({} bytes)", lang, out.display(), bytes.len());
    Ok(())
}

/// `--compile-phonemes[=<source>]`: parse a phoneme-table source file
/// (`phonemetable`/`phoneme … endphoneme` blocks) and write the binary `phontab`
/// — the table half of `espeak-ng.c`'s `--compile-phonemes` via
/// `parse_phoneme_dsl` + `compile_phontab`.  `src_arg` empty → the data dir's
/// `phonemes` file.  Only the phoneme *table* is produced (the `phonindex` /
/// `phondata` program+acoustic data are compiled separately and not regenerated).
fn run_compile_phonemes(src_arg: &str, cli: &Cli) -> std::io::Result<()> {
    use espeak_ng::phoneme::parse_phoneme_dsl;
    use espeak_ng::phoneme::table::compile_phontab;

    let data_dir = resolved_data_dir(cli.path_override.as_deref());
    let given = PathBuf::from(src_arg);
    let c_default = data_dir.join("../phsource/phonemes");
    let src_path = match () {
        _ if src_arg.is_empty() && !c_default.exists() && data_dir.join("phonemes").exists() => {
            data_dir.join("phonemes")
        }
        _ if src_arg.is_empty() => c_default,
        _ if given.is_file() => given,
        _ => given.join("phonemes"),
    };
    println!("Compiling phoneme data: {}", src_path.display());
    if !src_path.exists() {
        eprintln!(
            "Error processing file '{}': No such file or directory.",
            src_path.display()
        );
        std::process::exit(1);
    }

    let source = std::fs::read_to_string(&src_path)?;
    let tables = parse_phoneme_dsl(&source);
    if tables.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("no phoneme tables parsed from {}", src_path.display()),
        ));
    }

    let bytes = compile_phontab(&tables);
    let out = data_dir.join("phontab");
    std::fs::write(&out, &bytes)?;
    let n_ph: usize = tables.iter().map(|t| t.phonemes.len()).sum();
    eprintln!(
        "Compiled {} phoneme table(s), {} phonemes → {} ({} bytes)",
        tables.len(), n_ph, out.display(), bytes.len()
    );
    eprintln!("Note: phoneme table only — phonindex/phondata are not regenerated.");
    Ok(())
}

/// `--compile-mbrola=<source>`: compile an MBROLA phoneme-translation source
/// (`phsource/mbrola/<db>`) into the binary `<data>/mbrola_ph/<db>_phtrans` —
/// `espeak-ng.c`'s `--compile-mbrola` via `espeak_ng_CompileMbrolaVoice`.
///
/// The output file is named after the source file, as in C.
fn run_compile_mbrola(src_arg: &str, cli: &Cli) -> std::io::Result<()> {
    use espeak_ng::mbrola::MbrolaTable;

    let src_path = PathBuf::from(src_arg);
    if !src_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("mbrola source not found: {}", src_path.display()),
        ));
    }
    let name = src_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let source = std::fs::read_to_string(&src_path)?;
    let table = MbrolaTable::compile(&source);

    let data_dir = resolved_data_dir(cli.path_override.as_deref());
    let out_dir = data_dir.join("mbrola_ph");
    std::fs::create_dir_all(&out_dir)?;
    let out = out_dir.join(format!("{name}_phtrans"));
    let bytes = table.to_bytes();
    std::fs::write(&out, &bytes)?;
    eprintln!(
        "Mbrola translation file: {} -- {} phonemes",
        out.display(),
        table.entries.len()
    );
    Ok(())
}

type Voice = espeak_ng::voices::VoiceEntry;

fn pad_bytes(s: &str, width: usize) -> String {
    format!("{s}{}", " ".repeat(width.saturating_sub(s.len())))
}

fn voice_line(v: &Voice) -> String {
    let age = if v.age == 0 { " --".to_string() } else { format!("{:>3}", v.age) };
    let name = v.name.replace(' ', "_");
    let mut line = format!(
        "{:>2}  {}{}/{}      {} {} ",
        v.languages[0].1,
        pad_bytes(&v.languages[0].0, 15),
        age,
        v.gender,
        pad_bytes(&name, 18),
        pad_bytes(&v.file, 20),
    );
    for (code, prio) in &v.languages[1..] {
        line.push_str(&format!("({code} {prio})"));
    }
    line
}

/// `--voices[=<lang>]`, byte for byte as C's `DisplayVoices`.
fn print_voices(cli: &Cli) {
    let data = resolved_data_dir(cli.path_override.as_deref());
    let voices =
        listed_voices(&data, espeak_ng::voices::list_catalog(&data), cli.voices_filter.as_deref());
    println!(
        "Pty Language       Age/Gender VoiceName          File                 Other Languages"
    );
    for v in &voices {
        println!("{}", voice_line(v));
    }
}

/// Apply document-level `<prosody>` multipliers to the engine's rate/pitch/
/// volume (each scaled from its current value; `set_parameter` clamps).
fn apply_prosody(engine: &mut EspeakNg, p: espeak_ng::translate::ssml::Prosody) {
    let scale = |cur: i32, m: Option<f32>| m.map(|m| (cur as f32 * m).round() as i32);
    if let Some(r) = scale(engine.get_parameter(Parameter::Rate), p.rate) {
        engine.set_parameter(Parameter::Rate, r);
    }
    if let Some(v) = scale(engine.get_parameter(Parameter::Pitch), p.pitch) {
        engine.set_parameter(Parameter::Pitch, v);
    }
    if let Some(v) = scale(engine.get_parameter(Parameter::Volume), p.volume) {
        engine.set_parameter(Parameter::Volume, v);
    }
}

fn apply_runtime_params(engine: &mut EspeakNg, cli: &Cli) {
    if let Some(r) = cli.rate {
        engine.set_parameter(Parameter::Rate, r as i32);
    }
    if let Some(p) = cli.pitch {
        engine.set_parameter(Parameter::Pitch, p as i32);
    }
    if let Some(v) = cli.amplitude {
        engine.set_parameter(Parameter::Volume, v as i32);
    }
    if let Some(g) = cli.range {
        engine.set_parameter(Parameter::Range, g as i32);
    }
    // Runtime text options that must also reach the audio path (previously only
    // wired into `-x`/`--ipa`).
    if let Some(g) = cli.word_gap {
        engine.set_parameter(Parameter::WordGap, g as i32);
    }
    if let Some(k) = cli.capitals {
        engine.set_parameter(Parameter::Capitals, k as i32);
    }
    if let Some(b) = cli.ssml_break {
        engine.set_parameter(Parameter::SsmlBreak, b as i32);
    }
    if let Some(ref p) = cli.punct {
        engine.set_punctuation_list(Some(p.clone()));
    }
    if cli.no_final_pause {
        engine.set_no_final_pause(true);
    }
    if cli.deterministic {
        // Upstream's `-D` pins `espeak_srand` to 1 (it otherwise seeds from the
        // clock).  This port never seeds from the clock, so `-D` only selects
        // the same noise sequence upstream's deterministic mode uses.
        engine.set_rand_seed(1);
    }
    // `-m` / `--markup`: interpret SSML in synthesis (currently the `<audio>`
    // sound-icon tag).
    engine.set_markup(cli.markup);
}

/// `DecodePhonemes` — phoneme codes to mnemonics, for the `-X` trace.
///
/// A stress phoneme prints as one of `= , '` rather than its mnemonic (C's
/// `stress_chars`), and the level-1 "unstressed" marker prints as nothing.
fn decode_phonemes(codes: &[u8], phdata: &PhonemeData) -> String {
    const STRESS_CHARS: [char; 8] = ['=', '=', ',', ',', '\'', '*', ' ', ' '];
    let mut out = String::new();
    for &c in codes {
        if c == 0 || c == 255 {
            continue;
        }
        let Some(ph) = phdata.get(c) else { continue };
        // `phSTRESS` == 1
        if ph.typ == 1 && ph.std_length <= 4 && ph.program == 0 {
            if ph.std_length > 1 {
                out.push(STRESS_CHARS[ph.std_length as usize]);
            }
        } else {
            out.push_str(&ph.mnemonic_str());
        }
    }
    out
}

/// `-X`: print the letter-to-sound rules that match each letter, with their
/// scores — `espeak-ng.c`'s `-X` (`option_phonemes |= espeakPHONEMES_TRACE`).
///
/// Each word gets a `Translate '<word>'` header, then one line per matching
/// rule: `<score>\t<rule> [<phonemes>]`, with a blank line between letter
/// positions.  The rule is rendered back to source form by `DecodeRule`.
fn trace_output(cli: &Cli, text: &str, phonemes_follow: bool) -> std::io::Result<()> {
    use espeak_ng::dictionary::rules::{trace_begin, trace_take, TraceEvent};

    let ioerr = |e: espeak_ng::Error| std::io::Error::other(format!("{e}"));
    let data_dir = resolved_data_dir(cli.path_override.as_deref());
    let translator = Translator::new(&cli.voice, Some(&data_dir)).map_err(ioerr)?;
    let mut phdata = PhonemeData::load(&data_dir).map_err(ioerr)?;
    espeak_ng::translate::select_phoneme_table(
        &mut phdata,
        &data_dir,
        translator.options.lang.as_str(),
    )
    .map_err(ioerr)?;

    trace_begin();
    let _ = translator.text_to_ipa(text);
    let events = trace_take();

    let mut out = String::new();
    let mut last_pos: Option<usize> = None;
    // C emits the blank line before the phoneme output only when rules ran; a
    // dictionary hit prints `Found:` and goes straight to the phonemes.
    let mut rules_ran = false;
    for ev in events {
        match ev {
            TraceEvent::Word(w) => {
                out.push_str(&format!("Translate '{w}'\n"));
                last_pos = None;
            }
            TraceEvent::Found(w, ph, flags) => {
                // A dictionary hit short-circuits the rules, so C prints this
                // *instead* of a `Translate` header and any rule lines.
                let flags = espeak_ng::dictionary::rules::decode_dict_flags(flags);
                out.push_str(&format!(
                    "Found: '{w}' [{}]  {flags}\n",
                    decode_phonemes(&ph, &phdata)
                ));
                last_pos = None;
            }
            TraceEvent::Rule(r) => {
                // C prints a blank line when it moves on to the next letter.
                if last_pos.is_some_and(|p| p != r.word_pos) {
                    out.push('\n');
                }
                last_pos = Some(r.word_pos);
                rules_ran = true;
                let mnemonics = decode_phonemes(&r.phonemes, &phdata);
                out.push_str(&format!("{:3}\t{} [{}]\n", r.points, r.rule, mnemonics));
            }
        }
    }

    if rules_ran {
        out.push('\n');
    }
    if !phonemes_follow {
        let codes = translator.translate_to_codes(text).map_err(ioerr)?;
        out.push_str(&format_x(&codes, &phdata, cli.sep, cli.tie, &data_dir, translator.options.lang.as_str()));
        out.push('\n');
    }

    emit(cli, &out)
}

/// Write phoneme output to `--phonout` (created empty at start-up) or stdout.
fn emit(cli: &Cli, text: &str) -> std::io::Result<()> {
    match cli.phonout {
        Some(ref path) => std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?
            .write_all(text.as_bytes()),
        None => std::io::stdout().write_all(text.as_bytes()),
    }
}

/// `--pho`: write the MBROLA `.pho` script (phoneme, duration, pitch targets)
/// instead of speaking — `espeak-ng.c`'s `--pho`, via `MbrolaTranslate`.
///
/// With an `mb-*` voice the eSpeak phoneme names are translated through that
/// diphone database's `_phtrans` table.  C only produces `.pho` for an MBROLA
/// voice; here any voice works, and a non-MBROLA one simply keeps eSpeak's own
/// phoneme names (an empty translation table is the identity mapping).
fn pho_output(cli: &Cli, text: &str) -> std::io::Result<()> {
    use espeak_ng::mbrola::{write_pho, MbrolaTable};
    use espeak_ng::synthesize::Synthesizer;

    let ioerr = |e: espeak_ng::Error| std::io::Error::other(format!("{e}"));
    let data_dir = resolved_data_dir(cli.path_override.as_deref());

    // An `mb-<db>` voice speaks the language its voice file names and maps
    // phoneme names through that database's translation table.
    let mb = espeak_ng::voices::load_mbrola_voice(&data_dir, &cli.voice);
    let (lang, table) = match &mb {
        Some(v) => {
            let bytes = std::fs::read(data_dir.join("mbrola_ph").join(&v.phtrans))?;
            (v.language.clone(), MbrolaTable::parse(&bytes).map_err(ioerr)?)
        }
        None => (cli.voice.clone(), MbrolaTable::default()),
    };

    let mut translator = Translator::new(&lang, Some(&data_dir)).map_err(ioerr)?;
    if let Some(g) = cli.word_gap {
        translator.options.word_gap = g as i32;
    }
    let codes = translator.translate_to_codes(text).map_err(ioerr)?;
    let mut phdata = PhonemeData::load(&data_dir).map_err(ioerr)?;
    espeak_ng::translate::select_phoneme_table(
        &mut phdata,
        &data_dir,
        translator.options.lang.as_str(),
    )
    .map_err(ioerr)?;

    let mut voice = espeak_ng::synthesize::VoiceParams::default();
    if let Some(r) = cli.rate {
        // Same mapping the engine uses (upstream's `speed_lookup` curve).
        let f = espeak_ng::synthesize::setlengths::speed_duration_factor(r);
        voice.speed_percent = ((100.0 / f).round() as u32).clamp(20, 800);
    }
    // The `.pho` pitch targets follow the voice's own `pitch <base> <range>` —
    // the MBROLA voice file's if it has one, else the language's.
    let pitch_line = mb
        .as_ref()
        .and_then(|v| v.pitch)
        .or_else(|| espeak_ng::voices::voice_pitch(&data_dir, &lang));
    if let Some((p1, p2)) = pitch_line {
        voice.pitch_hz =
            espeak_ng::synthesize::intonation::base_pitch_hz(p1 as f64, cli.pitch.unwrap_or(50))
                .round() as u32;
        voice.pitch_range_units = (p2 - p1) as f64 * 108.0;
    }
    let synth = Synthesizer::new(voice);
    let out = write_pho(&table.translate(&synth.mbrola_phonemes(&codes, &phdata)));

    emit(cli, &out)
}

fn phoneme_output(cli: &Cli, text: &str) -> std::io::Result<()> {
    let data_dir = resolved_data_dir(cli.path_override.as_deref());
    let mut translator = Translator::new(&cli.voice, Some(&data_dir)).map_err(|e| {
        std::io::Error::other(format!("{e}"))
    })?;
    if let Some(k) = cli.capitals {
        translator.options.capitals = k as u8;
    }
    if let Some(g) = cli.word_gap {
        translator.options.word_gap = g as i32;
    }
    if let Some(ref p) = cli.punct {
        translator.options.punct = Some(p.clone());
    }
    // `--tie` and `--sep` have to reach the IPA writer: it is the only place
    // that knows where one phoneme's name ends.
    translator.options.ipa_tie = cli.tie;
    translator.options.ipa_separator = cli.sep;

    let line = if cli.ipa {
        let ipa = if cli.markup {
            translator.text_to_ipa_ssml(text)
        } else {
            translator.text_to_ipa_clauses(text)
        }
        .map_err(|e| {
            std::io::Error::other(format!("{e}"))
        })?;
        ipa
    } else {
        // `-x` with `-m`: the codes path takes plain text, so process SSML first
        // (strip tags, decode entities, apply say-as) — matching `--ipa -m`.
        // (Cross-`<voice>` language switching in `-x` is not yet wired.)
        let src = if cli.markup {
            espeak_ng::translate::ssml_to_speech_text_opts(
                text,
                translator.options.lang.as_str(),
                cli.ssml_break.unwrap_or(100),
            )
        } else {
            text.to_string()
        };
        let codes = if cli.markup {
            translator.translate_to_codes(&src)
        } else {
            translator.translate_to_clause_codes(&src)
        }
        .map_err(|e| std::io::Error::other(format!("{e}")))?;
        let mut phdata = PhonemeData::load(&data_dir).map_err(|e| {
            std::io::Error::other(format!("{e}"))
        })?;
        // Same resolution as the library path (voice `phonemes` directive, then
        // the primary subtag) so a regional locale without its own table — pt-br,
        // fr-ca, en-gb — doesn't fail here after working in `--ipa`.
        espeak_ng::translate::select_phoneme_table(
            &mut phdata,
            &data_dir,
            translator.options.lang.as_str(),
        )
        .map_err(|e| std::io::Error::other(format!("{e}")))?;
        format_x(&codes, &phdata, cli.sep, cli.tie, &data_dir, translator.options.lang.as_str())
    };

    let closing = if cli.markup && ends_with_clause_tag(text) { "\n" } else { "" };
    emit(cli, &format!("{line}\n{closing}"))
}

/// C's `ProcessSsmlTag` returns a clause terminator for these tags.  When one
/// is the last thing in the text, `ReadClause` is called once more and the empty
/// clause it returns prints a line of its own.
fn ends_with_clause_tag(text: &str) -> bool {
    let trimmed = text.trim_end();
    let Some(body) = trimmed.strip_suffix('>') else { return false };
    let Some(open) = body.rfind('<') else { return false };
    let inner = &body[open + 1..];
    let starts_tag = inner.chars().next().is_some_and(|c| c == '/' || c.is_alphabetic());
    starts_tag && tag_ends_clause(inner.strip_suffix('/').unwrap_or(inner))
}

fn tag_ends_clause(inner: &str) -> bool {
    let (closing, rest) = match inner.strip_prefix('/') {
        Some(r) => (true, r),
        None => (false, inner),
    };
    let name_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let name = rest[..name_end].to_ascii_lowercase();
    let attrs = &rest[name_end..];
    let block = matches!(
        name.as_str(),
        "s" | "p" | "h1" | "h2" | "h3" | "h4" | "hr" | "br" | "li" | "dd" | "img" | "td"
    );
    match name.as_str() {
        "speak" => closing,
        "break" => !closing && break_returns_terminator(attrs),
        _ => block,
    }
}

fn break_returns_terminator(attrs: &str) -> bool {
    let strong = ["medium", "strong", "x-strong"]
        .iter()
        .any(|s| attrs.contains(&format!("strength=\"{s}\"")));
    let has_strength = attrs.contains("strength=");
    !has_strength || strong || attrs.contains("time=")
}


fn format_x(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    sep: Option<char>,
    tie: Option<char>,
    data_dir: &Path,
    lang: &str,
) -> String {
    espeak_ng::phoneme_text::format_mnemonics(codes, phdata, sep, tie, data_dir, lang)
}

/// Map the `-b` value to an encoding and decode mode.  Unset = auto (UTF-8,
/// falling back to Latin-1 at the first invalid byte, like `espeakCHARS_AUTO`),
/// 1 = strict UTF-8, 2 = ISO-8859-1, 4 = UCS-2 (16-bit little-endian).
fn input_encoding(b: Option<u32>) -> (espeak_ng::Encoding, espeak_ng::DecodeMode) {
    use espeak_ng::{DecodeMode, Encoding};
    match b {
        None => (Encoding::Iso8859_1, DecodeMode::Auto),
        Some(2) => (Encoding::Iso8859_1, DecodeMode::Strict),
        Some(4) => (Encoding::Ucs2, DecodeMode::Strict),
        Some(_) => (Encoding::Utf8, DecodeMode::Strict),
    }
}

/// `fgets(buf, 1000)` over `bytes`: lines up to and including `\n`, at most 999
/// bytes each.
fn fgets_chunks(bytes: &[u8]) -> Vec<&[u8]> {
    let mut rest = bytes;
    let mut out = Vec::new();
    while !rest.is_empty() {
        let limit = rest.len().min(999);
        let end = rest[..limit].iter().position(|&b| b == b'\n').map_or(limit, |p| p + 1);
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    out
}

fn read_stdin() -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;
    Ok(buf)
}

fn read_input_file(path: &Path) -> Vec<u8> {
    if std::fs::metadata(path).is_err() {
        eprintln!("Failed to stat() file '{}'", path.display());
        std::process::exit(1);
    }
    std::fs::read(path).unwrap_or_else(|_| {
        eprintln!("Failed to read file '{}'", path.display());
        std::process::exit(1);
    })
}

/// The pieces of text C hands to `espeak_Synth`: the `-f` file or the bulk
/// `--stdin` as one, the first word of the command line as one, otherwise one
/// per line of stdin.
fn gather_texts(cli: &Cli) -> std::io::Result<Vec<String>> {
    let (enc, mode) = input_encoding(cli.input_encoding);
    let decode = |bytes: &[u8]| -> String {
        espeak_ng::TextDecoder::new(bytes, enc, mode)
            .map(|mut d| d.decode_to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
    };

    if let Some(ref path) = cli.file_input {
        return Ok(vec![decode(&read_input_file(path))]);
    }
    if let (Some(first), false) = (cli.positionals.first(), cli.stdin_all) {
        return Ok(vec![decode(first.to_string_lossy().as_bytes())]);
    }
    let input = read_stdin()?;
    if cli.stdin_all {
        return Ok(if input.is_empty() { Vec::new() } else { vec![decode(&input)] });
    }
    Ok(fgets_chunks(&input).into_iter().map(decode).collect())
}

fn resolved_data_dir(path_arg: Option<&Path>) -> PathBuf {
    path_arg
        .and_then(espeak_ng::translate::data_dir_from_path)
        .unwrap_or_else(|| PathBuf::from(default_data_dir()))
}

/// `--split=<minutes>`: write the audio as `<name>_01<ext>`, `<name>_02<ext>`, …
/// cutting at the first sentence boundary after each `minutes` of audio — the
/// `samples_total > samples_split` test in upstream's `SynthCallback`, which
/// numbers the parts with `%s_%.2d%s` and keeps an extension of up to four
/// characters (`out.wav` → `out_01.wav`, `out` → `out_01`).
fn write_split_wavs(
    path: &Path,
    pcm: &[i16],
    rate: u32,
    events: &[espeak_ng::engine::SynthEvent],
    minutes: u32,
) -> std::io::Result<()> {
    use espeak_ng::engine::EventKind;

    let name = path.to_string_lossy().into_owned();
    let (stem, ext) = match name.rfind('.') {
        Some(ix) if name.len() - ix <= 4 => (&name[..ix], &name[ix..]),
        _ => (name.as_str(), ""),
    };

    let limit = minutes as usize * 60 * rate as usize;
    let at_sample = |ms: u32| (ms as u64 * rate as u64 / 1000) as usize;
    let mut cuts = vec![0usize];
    for e in events {
        if e.kind != EventKind::Sentence {
            continue;
        }
        let at = at_sample(e.audio_position_ms).min(pcm.len());
        if at > *cuts.last().unwrap() && at - *cuts.last().unwrap() > limit {
            cuts.push(at);
        }
    }
    cuts.push(pcm.len());

    for (n, w) in cuts.windows(2).enumerate() {
        let mut buf = Vec::new();
        write_wav_16_mono(&mut buf, &pcm[w[0]..w[1]], rate)
            .map_err(|e| std::io::Error::other(format!("{e}")))?;
        std::fs::write(format!("{stem}_{:02}{ext}", n + 1), &buf)?;
    }
    Ok(())
}

fn write_wav_16_mono(w: &mut Vec<u8>, samples: &[i16], sample_rate: u32) -> std::io::Result<()> {
    let data_bytes = samples.len().saturating_mul(2) as u64;
    w.clear();
    w.extend_from_slice(&wav_header(sample_rate, Some(data_bytes)));
    for &s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    Ok(())
}

const SHORT_OPTIONS: &str = "a:b:Dd:f:g:hk:l:mp:P:qs:v:w:xXz";

const fn long(name: &'static str, has_arg: HasArg, abbreviable: bool) -> LongOpt {
    LongOpt { name, has_arg, abbreviable }
}

/// C's `long_options` order, which fixes the order of an "ambiguous" listing.
/// `compile-mbrola` is only accepted spelled out: the reference build leaves
/// it out of its option table.
const LONG_OPTIONS: &[LongOpt] = &[
    long("help", HasArg::No, true),
    long("stdin", HasArg::No, true),
    long("compile-debug", HasArg::Optional, true),
    long("compile", HasArg::Optional, true),
    long("punct", HasArg::Optional, true),
    long("voices", HasArg::Optional, true),
    long("stdout", HasArg::No, true),
    long("split", HasArg::Optional, true),
    long("path", HasArg::Required, true),
    long("phonout", HasArg::Required, true),
    long("pho", HasArg::No, true),
    long("ipa", HasArg::Optional, true),
    long("version", HasArg::No, true),
    long("sep", HasArg::Optional, true),
    long("tie", HasArg::Optional, true),
    long("compile-mbrola", HasArg::Optional, false),
    long("compile-intonations", HasArg::No, true),
    long("compile-phonemes", HasArg::Optional, true),
    long("load", HasArg::No, true),
    long("ssml-break", HasArg::Required, true),
];

fn nonneg(n: i32) -> Option<u32> {
    u32::try_from(n).ok()
}

fn positive(n: i32) -> Option<u32> {
    nonneg(n).filter(|&v| v > 0)
}

/// `sscanf("%d")`: the leading integer, if the text starts with one.
fn scan_int(s: &str) -> Option<i32> {
    let t = s.trim_start().trim_start_matches(['-', '+']);
    t.starts_with(|c: char| c.is_ascii_digit()).then(|| getopt::atoi(s))
}

/// Fold `-b <n>` into the encoding flags: `n <= 4` is OR-ed in, anything else
/// means 8-bit.
fn add_encoding_flag(current: Option<u32>, arg: &str) -> Option<u32> {
    let flag = match scan_int(arg) {
        Some(v) if v <= 4 => nonneg(v).unwrap_or(0),
        _ => 2,
    };
    Some(current.unwrap_or(0) | flag).filter(|&f| f != 0)
}

/// The 44-byte header of a 16-bit mono WAV. `None` writes the placeholder sizes
/// upstream uses when the length is not known yet (a pipe).
fn wav_header(sample_rate: u32, data_bytes: Option<u64>) -> [u8; 44] {
    let (riff_len, data_len) = match data_bytes {
        Some(n) => {
            let n = u32::try_from(n).unwrap_or(u32::MAX);
            (n.saturating_add(36), n)
        }
        None => (0x7fff_ff24, 0x7fff_f000),
    };
    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&riff_len.to_le_bytes());
    h.extend_from_slice(b"WAVEfmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes());
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&sample_rate.saturating_mul(2).to_le_bytes());
    h.extend_from_slice(&2u16.to_le_bytes());
    h.extend_from_slice(&16u16.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    h.try_into().expect("header is 44 bytes")
}

/// A WAV destination that can have its header sizes patched once the length is
/// known (a file), or cannot (a pipe).
trait WavOut: std::io::Write {
    fn patch_header(&mut self, header: &[u8; 44]) -> std::io::Result<bool>;
}

impl WavOut for std::io::BufWriter<std::fs::File> {
    fn patch_header(&mut self, header: &[u8; 44]) -> std::io::Result<bool> {
        use std::io::{Seek, SeekFrom};
        self.seek(SeekFrom::Start(0))?;
        self.write_all(header)?;
        self.flush()?;
        Ok(true)
    }
}

impl WavOut for std::io::BufWriter<std::io::Stdout> {
    fn patch_header(&mut self, _header: &[u8; 44]) -> std::io::Result<bool> {
        Ok(false)
    }
}

/// Standard output as a WAV destination: patchable when it is redirected to a
/// regular file, a plain stream when it is a pipe or terminal.
fn wav_stdout_target() -> Box<dyn WavOut> {
    #[cfg(unix)]
    {
        use std::io::Seek;
        use std::os::fd::AsFd;
        let seekable = std::io::stdout()
            .as_fd()
            .try_clone_to_owned()
            .map(std::fs::File::from)
            .and_then(|mut f| f.stream_position().map(|_| f));
        if let Ok(file) = seekable {
            return Box::new(std::io::BufWriter::with_capacity(1 << 16, file));
        }
    }
    Box::new(std::io::BufWriter::with_capacity(1 << 16, std::io::stdout()))
}

/// Writes the utterance as a WAV clause by clause, so memory stays flat however
/// long the text is. The header is a placeholder until the length is known.
fn stream_wav(engine: &EspeakNg, text: &str, mut out: Box<dyn WavOut>) -> std::io::Result<()> {
    let rate = engine.sample_rate();
    let mut written: u64 = 0;
    let mut failure: Option<std::io::Error> = None;
    out.write_all(&wav_header(rate, None))?;
    let mut buf: Vec<u8> = Vec::new();
    engine
        .synth_streaming(text, |chunk, _final| {
            buf.clear();
            buf.extend(chunk.iter().flat_map(|s| s.to_le_bytes()));
            written += buf.len() as u64;
            failure = out.write_all(&buf).err();
            failure.is_some()
        })
        .map_err(std::io::Error::other)?;
    if let Some(e) = failure {
        return Err(e);
    }
    out.flush()?;
    out.patch_header(&wav_header(rate, Some(written)))?;
    Ok(())
}

fn parse_args(args: Vec<OsString>) -> Cli {
    let words: Vec<String> = args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    let parsed = getopt::parse(&words, SHORT_OPTIONS, LONG_OPTIONS);
    let mut cli = Cli { voice: "en".to_string(), ..Default::default() };
    for opt in &parsed.opts {
        if apply_option(&mut cli, opt) {
            return cli;
        }
    }
    cli.positionals = parsed.positionals.into_iter().map(OsString::from).collect();
    cli.error_exit = parsed.error.map(|m| (format!("{}: {m}", argv0_display()), 0));
    // Upstream keeps one separator *character* and a sticky tie *flag*, so the
    // character belongs to whichever mode the last of `--sep`/`--tie` selected.
    if cli.tie_mode {
        cli.tie = cli.sep.take().or(Some('\u{361}'));
    }
    cli
}

/// Apply one option in command-line order; `true` means C would have left
/// `getopt` at this option (help, version, voices, a compile action, an error).
fn apply_option(cli: &mut Cli, opt: &getopt::Opt) -> bool {
    let arg = opt.arg.as_deref().unwrap_or("");
    match opt.short {
        Some(c) => apply_short(cli, c, arg),
        None => apply_long(cli, &opt.long, opt.arg.as_deref()),
    }
}

fn apply_short(cli: &mut Cli, c: char, arg: &str) -> bool {
    let n = getopt::atoi(arg);
    match c {
        'a' => cli.amplitude = nonneg(n),
        'b' => cli.input_encoding = add_encoding_flag(cli.input_encoding, arg),
        'D' => cli.deterministic = true,
        'd' => cli.device = Some(arg.to_string()),
        'f' => cli.file_input = (!arg.is_empty()).then(|| PathBuf::from(arg)),
        'g' => cli.word_gap = nonneg(n),
        'h' => {
            cli.print_help = true;
            return true;
        }
        'k' => cli.capitals = nonneg(n),
        'l' => cli.line_length = positive(n),
        'm' => cli.markup = true,
        'p' => cli.pitch = nonneg(n),
        'P' => cli.range = nonneg(n),
        'q' => cli.quiet = true,
        's' => cli.rate = positive(n),
        'v' => set_voice(cli, arg),
        'w' => set_wave_out(cli, arg),
        'x' => cli.x_mnem = true,
        'X' => cli.trace_rules = true,
        _ => cli.no_final_pause = true,
    }
    false
}

fn set_voice(cli: &mut Cli, arg: &str) {
    let name: String = arg.chars().take(39).collect();
    cli.voice = if name.is_empty() { "en".to_string() } else { name.to_ascii_lowercase() };
    cli.voice_raw = Some(name);
}

fn set_wave_out(cli: &mut Cli, arg: &str) {
    cli.stdout_wav = arg == "stdout";
    cli.wave_out = (!cli.stdout_wav).then(|| PathBuf::from(arg));
}

fn apply_long(cli: &mut Cli, name: &str, arg: Option<&str>) -> bool {
    match name {
        "help" => cli.print_help = true,
        "version" => cli.print_version = true,
        "voices" => {
            cli.list_voices = true;
            cli.voices_filter = arg.map(str::to_string);
        }
        "compile" | "compile-debug" => return set_compile(cli, name, arg),
        // Deferred to the end of parsing so a later `--path` still applies.
        "compile-mbrola" => {
            cli.compile_mbrola = Some(arg.unwrap_or("").to_string());
            return false;
        }
        "compile-intonations" => {
            cli.compile_intonations = true;
            return false;
        }
        "compile-phonemes" => {
            cli.compile_phonemes = Some(arg.unwrap_or("").to_string());
            return false;
        }
        _ => {
            apply_plain_long(cli, name, arg);
            return false;
        }
    }
    true
}

fn set_compile(cli: &mut Cli, name: &str, arg: Option<&str>) -> bool {
    match arg.filter(|a| !a.is_empty()) {
        Some(voice) => {
            cli.compile = Some(voice.to_string());
            cli.quiet = true;
            false
        }
        None => {
            cli.error_exit = Some((format!("Voice name to '--{name}' not specified."), 1));
            true
        }
    }
}

fn apply_plain_long(cli: &mut Cli, name: &str, arg: Option<&str>) {
    match name {
        "stdin" => cli.stdin_all = true,
        "stdout" => set_wave_out(cli, "stdout"),
        "punct" => cli.punct = Some(punct_list(arg)),
        "split" => cli.split_minutes = Some(arg.map_or(30, |a| nonneg(getopt::atoi(a)).unwrap_or(0))),
        "path" => cli.path_override = arg.map(PathBuf::from),
        "phonout" => cli.phonout = arg.map(PathBuf::from),
        "pho" => cli.pho = true,
        "ipa" => set_ipa(cli, arg),
        "sep" => set_sep(cli, arg),
        "tie" => set_tie(cli, arg),
        "load" => cli.load_voice = true,
        _ => cli.ssml_break = arg.and_then(|a| positive(getopt::atoi(a))),
    }
}

/// An empty `--punct=` list announces nothing; `U+0000` never occurs in text.
fn punct_list(arg: Option<&str>) -> Vec<char> {
    match arg {
        None => Vec::new(),
        Some("") => vec!['\0'],
        Some(chars) => chars.chars().collect(),
    }
}

/// The deprecated numeric forms: 1 = `_` between phonemes, 2 = a U+0361 tie,
/// 3 = a ZWJ tie.  Anything else is plain IPA.
fn set_ipa(cli: &mut Cli, arg: Option<&str>) {
    cli.ipa = true;
    match arg.map_or(0, getopt::atoi) {
        1 => cli.sep = Some('_'),
        2 => set_tie_char(cli, '\u{361}'),
        3 => set_tie_char(cli, '\u{200d}'),
        _ => {}
    }
}

fn set_tie_char(cli: &mut Cli, c: char) {
    cli.sep = Some(c);
    cli.tie_mode = true;
}

fn set_sep(cli: &mut Cli, arg: Option<&str>) {
    cli.sep = arg.map_or(Some(' '), parse_sep_value);
    cli.x_mnem = true;
}

fn set_tie(cli: &mut Cli, arg: Option<&str>) {
    cli.sep = arg.map_or(Some('\u{361}'), parse_tie_value);
    cli.tie_mode = true;
    cli.x_mnem = true;
}

/// `--tie[=<c>]`: no value means U+0361 (combining double inverted breve) and
/// `z` means U+200D (zero-width joiner), as upstream documents.
fn parse_tie_value(s: &str) -> Option<char> {
    if s.is_empty() {
        return Some('\u{361}');
    }
    if s == "z" {
        return Some('\u{200d}');
    }
    s.chars().next()
}

fn parse_sep_value(s: &str) -> Option<char> {
    if s == "z" {
        return Some('\u{200c}');
    }
    let mut it = s.chars();
    let c = it.next()?;
    Some(c)
}

const HELP_TEXT: &str = "
espeak-ng [options] [\"<words>\"]

-f <text file>   Text file to speak
--stdin    Read text input from stdin at once till to the end of a stream.

If neither -f nor --stdin are provided, then <words> from arguments are spoken,
or text is spoken from stdin, read separately one line by line at a time.

-a <integer>
\t   Amplitude, 0 to 200, default is 100
-d <device>
\t   Use the specified device to speak the audio on. If not specified, the
\t   default audio device is used.
-g <integer>
\t   Word gap. Pause between words, units of 10mS at the default speed
-k <integer>
\t   Indicate capital letters with: 1=sound, 2=the word \"capitals\",
\t   higher values indicate a pitch increase (try -k20).
-l <integer>
\t   Line length. If not zero (which is the default), consider
\t   lines less than this length as end-of-clause
-p <integer>
\t   Pitch adjustment, 0 to 99, default is 50
-P <integer>
\t   Pitch range adjustment, 0 to 99, default is 50
-s <integer>
\t   Speed in approximate words per minute. The default is 175
-v <voice name>
\t   Use voice file of this name from espeak-ng-data/voices
-w <wave file name>
\t   Write speech to this WAV file, rather than speaking it directly
-b\t   Input text encoding, 1=UTF8, 2=8 bit, 4=16 bit\x20
-m\t   Interpret SSML markup, and ignore other < > tags
--ssml-break=<percentage>
\t   Set SSML break time multiplier, default is 100
-q\t   Quiet, don't produce any speech (may be useful with -x)
-x\t   Write phoneme mnemonics to stdout
-X\t   Write phonemes mnemonics and translation trace to stdout
-z\t   No final sentence pause at the end of the text
-D\t   Enable deterministic random mode
--compile=<voice name>
\t   Compile pronunciation rules and dictionary from the current
\t   directory. <voice name> specifies the language
--compile-debug=<voice name>
\t   Compile pronunciation rules and dictionary from the current
\t   directory, including line numbers for use with -X.
\t   <voice name> specifies the language
--compile-intonations
\t   Compile the intonation data
--compile-phonemes=<phsource-dir>
\t   Compile the phoneme data using <phsource-dir> or the default phsource directory
--ipa      Write phonemes to stdout using International Phonetic Alphabet
--path=\"<path>\"
\t   Specifies the directory containing the espeak-ng-data directory
--pho      Write mbrola phoneme data (.pho) to stdout or to the file in --phonout
--phonout=\"<filename>\"
\t   Write phoneme output from -x -X --ipa and --pho to this file
--punct=\"<characters>\"
\t   Speak the names of punctuation characters during speaking.  If
\t   =<characters> is omitted, all punctuation is spoken.
--sep=<character>
\t   Separate phonemes (from -x --ipa) with <character>.
\t   Default is space, z means ZWJN character.
--split=<minutes>
\t   Starts a new WAV file every <minutes>.  Used with -w
--stdout   Write speech output to stdout
--tie=<character>
\t   Use a tie character within multi-letter phoneme names.
\t   Default is U+361, z means ZWJ character.
--version  Shows version number and date, and location of espeak-ng-data
--voices=<language>
\t   List the available voices for the specified language.
\t   If <language> is omitted, then list all voices.
--load     Load voice from a file in current directory by name.
-h, --help Show this help.
";
