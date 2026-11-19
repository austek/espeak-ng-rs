//! Safe operations behind the exported functions.

use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use espeak_ng::translate::{data_dir_from_path, default_data_dir, resolve_dict_stem, split_voice_variant};
use espeak_ng::voices::{
    find_catalog_voice, list_catalog, listed_voices, load_voice_file, resolve_voice,
    voice_file_entry, VoiceEntry,
};
use espeak_ng::{EspeakNg, VoiceSpec};

use crate::blocks::{build, buffer_samples, Block, Ev};
use crate::settings::Settings;
use crate::state::CANCEL;
use crate::types::{UriCallback, ENDPAUSE, POS_CHARACTER_VALUE, SSML};

pub const SAMPLE_RATE: i32 = 22050;
pub const VERSION: &str = "1.53.0";

pub fn resolve_data_dir(path: Option<PathBuf>) -> Option<PathBuf> {
    let dir = path
        .and_then(|p| data_dir_from_path(&p))
        .unwrap_or_else(|| PathBuf::from(default_data_dir()));
    dir.join("phontab").exists().then_some(dir)
}

/// C's `espeak_SetVoiceByName` takes a voice name or file, not a language code.
fn known_voice(data: &std::path::Path, name: &str) -> bool {
    let catalog = list_catalog(data);
    espeak_ng::voices::select_voice_by_name(&catalog, name).is_some()
        || espeak_ng::voices::load_mbrola_voice(data, name).is_some()
}

/// `espeak_SetVoiceByName`: true when the voice exists and is now selected.
pub fn select_voice(settings: &mut Settings, name: &str) -> bool {
    let data = settings.data_dir.clone();
    let (base, _) = split_voice_variant(name);
    let entry = voice_file_entry(&data, base);
    let spec = VoiceSpec::by_name(name);
    if entry.is_none() && !known_voice(&data, base) {
        return false;
    }
    let spelled = match (&entry, spec.language.as_deref()) {
        (Some(_), _) => name.to_string(),
        (None, Some(lang)) => match &spec.variant {
            Some(v) => format!("{lang}+{v}"),
            None => lang.to_string(),
        },
        (None, None) => name.to_string(),
    };
    match resolve_voice(&spelled, &data) {
        Some(v) => {
            settings.voice = v;
            settings.voice_file = entry;
            true
        }
        None => false,
    }
}

pub fn select_voice_file(settings: &mut Settings, path: &str) -> bool {
    let Some(v) = load_voice_file(std::path::Path::new(path)) else {
        return false;
    };
    let want = v.dictionary.clone().unwrap_or_else(|| v.language.clone());
    settings.voice = resolve_dict_stem(&settings.data_dir, &want).unwrap_or(want);
    settings.voice_file = Some(v);
    true
}

/// `espeak_ListVoices`: all voices, or those compatible with `language`.
pub fn list(settings: &Settings, language: Option<&str>) -> Vec<VoiceEntry> {
    let data = &settings.data_dir;
    listed_voices(data, list_catalog(data), language)
}

/// `espeak_SetVoiceByProperties` with a single language or name criterion.
pub fn select_by_properties(
    settings: &mut Settings,
    name: Option<&str>,
    language: Option<&str>,
    variant: usize,
) -> bool {
    let picked = match (language.filter(|l| !l.is_empty()), name.filter(|n| !n.is_empty())) {
        (Some(l), _) => list(settings, Some(l)).into_iter().nth(variant),
        (None, Some(n)) => find_catalog_voice(n, &settings.data_dir),
        (None, None) => list(settings, None).into_iter().next(),
    };
    picked.is_some_and(|v| select_voice(settings, &v.name))
}

pub struct Job {
    pub settings: Settings,
    pub text: String,
    pub flags: u32,
    pub position: u32,
    pub position_type: i32,
    pub buflength_ms: i32,
}

/// A `<audio src>` the reader handed to the URI callback.
#[derive(Clone, Debug)]
pub struct UriCall {
    /// The clause the reader was in.
    pub clause: usize,
    pub uri: String,
    pub base: Option<String>,
}

/// What only markup has: a phoneme line per clause and the reader's URI callbacks.
pub struct MarkupReport {
    pub lines: Vec<String>,
    pub uri_calls: Vec<UriCall>,
}

pub struct Rendered {
    /// The buffers of each clause, in order.
    pub clauses: Vec<Vec<Block>>,
    /// The text that was spoken, after `position` skipping.
    pub text: String,
    pub markup: Option<MarkupReport>,
}

/// The phoneme string of each clause of plain text, as C prints it to the trace
/// stream and hands it to the phoneme callback.
pub fn phoneme_lines(settings: &Settings, text: &str, mode: i32) -> Vec<String> {
    crate::phonemes::clauses(settings, text, mode)
        .map(|clauses| clauses.into_iter().map(|c| c.phonemes).collect())
        .unwrap_or_default()
}

/// The `Replace:` lines C prints before each clause's phoneme string when the trace mode has
/// bit 3 set.
pub fn replace_lines(settings: &Settings, text: &str, mode: i32) -> Vec<Vec<String>> {
    if mode & crate::phonemes::PHONEMES_TRACE == 0 {
        return Vec::new();
    }
    crate::phonemes::replace_lines(settings, text, mode).unwrap_or_default()
}

/// `espeak_SetPhonemeTrace`'s mode as the engine's trace format.
pub fn trace_format(mode: i32) -> espeak_ng::PhonemeTrace {
    let glyph = char::from_u32(((mode >> 8) & 0xffff) as u32).filter(|&c| c != '\0');
    let tie = mode & crate::phonemes::PHONEMES_TIE != 0;
    espeak_ng::PhonemeTrace {
        ipa: mode & crate::phonemes::PHONEMES_IPA != 0,
        separator: glyph.filter(|_| !tie),
        tie: glyph.filter(|_| tie),
    }
}

/// `uri_callback(1, uri, xmlbase)`; a URI with an interior NUL is refused.
pub fn call_uri(callback: UriCallback, uri: &str, base: Option<&str>) -> i32 {
    let Ok(uri) = CString::new(uri) else { return 1 };
    let base = base.and_then(|b| CString::new(b).ok());
    let base_ptr = base.as_ref().map_or(std::ptr::null(), |b| b.as_ptr());
    // SAFETY: both pointers are NUL-terminated strings that outlive the call (or null).
    unsafe { callback(1, uri.as_ptr(), base_ptr) }
}

type UriLog = std::sync::Arc<std::sync::Mutex<Vec<(String, Option<String>)>>>;

/// The reader's URI hook: it records each call and answers from `known` (the replies the C
/// callback gave already), accepting any later one until the callback is asked in its turn.
fn recording_hook(known: Vec<i32>, log: UriLog) -> espeak_ng::UriHook {
    Box::new(move |uri, base| {
        let mut calls = log.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let reply = known.get(calls.len()).copied().unwrap_or(0);
        calls.push((uri.to_string(), base.map(str::to_string)));
        reply
    })
}

fn engine_for(job: &Job, known: &[i32], log: &UriLog) -> espeak_ng::Result<EspeakNg> {
    let mut engine = job.settings.build_engine()?;
    engine.set_markup(job.flags & SSML != 0);
    let wants_uri = crate::state::callbacks().uri.is_some();
    engine.set_uri_callback(wants_uri.then(|| recording_hook(known.to_vec(), log.clone())));
    engine.set_no_final_pause(job.flags & ENDPAUSE == 0);
    Ok(engine)
}

fn skipped(job: &Job) -> String {
    match job.position_type {
        POS_CHARACTER_VALUE if job.position > 0 => job.text.chars().skip(job.position as usize).collect(),
        _ => job.text.clone(),
    }
}

/// Render the job.  For markup, `trace` asks for the phoneme line of each clause and `known` are
/// the replies the URI callback gave so far.
pub fn render(job: &Job, known: &[i32], trace: Option<i32>) -> espeak_ng::Result<Rendered> {
    let log = UriLog::default();
    let engine = engine_for(job, known, &log)?;
    let text = skipped(job);
    if job.flags & SSML != 0 {
        return render_markup(job, &engine, text, trace, &log);
    }
    let mut chunks: Vec<Vec<i16>> = Vec::new();
    let mut events = Vec::new();
    let rate = engine.synth_streaming_with_events(&text, |chunk, _final, ev| {
        chunks.push(chunk.to_vec());
        events.extend_from_slice(ev);
        CANCEL.load(Ordering::SeqCst)
    })?;
    let size = buffer_samples(job.buflength_ms, rate);
    let events: Vec<Ev> = events.iter().map(Ev::from).collect();
    Ok(Rendered { clauses: build(&chunks, &events, size), text, markup: None })
}

fn render_markup(job: &Job, engine: &EspeakNg, text: String, trace: Option<i32>, log: &UriLog) -> espeak_ng::Result<Rendered> {
    let done = engine.synth_markup_clauses(&text, trace.map(trace_format))?;
    let size = buffer_samples(job.buflength_ms, engine.sample_rate());
    let events: Vec<Ev> = done.events.iter().map(Ev::from).collect();
    let logged = log.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let uri_calls = logged
        .iter()
        .zip(&done.uri_clauses)
        .map(|((uri, base), &clause)| UriCall { clause, uri: uri.clone(), base: base.clone() })
        .collect();
    let markup = MarkupReport { lines: done.phoneme_lines, uri_calls };
    Ok(Rendered { clauses: build(&done.chunks, &events, size), text, markup: Some(markup) })
}

/// Compile `<lang>_list`/`<lang>_rules` from `source` (default: the data
/// directory) into `<data dir>/<lang>_dict`.
pub fn compile_dict(settings: &Settings, source: Option<&str>, name: Option<&str>) -> Result<(), String> {
    use espeak_ng::dictionary::compile::compile_dictionary;
    use espeak_ng::dictionary::transpose::TransposeConfig;
    use espeak_ng::phoneme::PhonemeData;

    let source = source.map_or_else(|| settings.data_dir.clone(), PathBuf::from);
    let lang = name.map_or_else(|| settings.voice.split('+').next().unwrap_or("en").to_string(), str::to_string);
    let mut phdata = PhonemeData::load(&settings.data_dir).map_err(|e| e.to_string())?;
    let table = espeak_ng::translate::resolve_phoneme_table(&settings.data_dir, &lang);
    if phdata.select_table_by_name(&table).is_err() && phdata.select_table_by_name(&lang).is_err() {
        phdata.select_table_by_name("en").map_err(|e| e.to_string())?;
    }
    let bytes = compile_dictionary(&source, &lang, &phdata, &TransposeConfig::for_lang(&lang))
        .map_err(|e| e.to_string())?;
    std::fs::write(settings.data_dir.join(format!("{lang}_dict")), bytes).map_err(|e| e.to_string())
}

/// C's `espeak_Key` for a multi-character name and `espeak_Char`.
pub fn character_markup(code: u32) -> String {
    format!("<say-as interpret-as=\"tts:char\">&#{code};</say-as>")
}
