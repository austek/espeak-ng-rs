// tests/ssml_tag_port.rs
//
// `ProcessSsmlTag` port (ssml.c, C master f0fb11a8): a tag in, the text and
// embedded commands it writes, its clause terminator and the stack state out.
// Terminators and strings follow ssml.c; the voice-stack cases were also run
// through the C binary (`-q -m -v en --ipa`) and match its line structure.

use espeak_ng::translate::readclause::attrs::{
    attr_copy, attr_eq, attr_lookup, attr_number, get_ssml_attribute, prosody_value,
};
use espeak_ng::translate::readclause::ssml_tag::{
    clause, parse_ssml_reference, process_ssml_tag, SsmlEnv, SsmlState, N_SPEECH_PARAM,
};
use espeak_ng::translate::readclause::voice_stack::{BaseVoice, VoiceCatalog, VoiceSelect};

const VOICE_CHANGE: u32 = 0x20000;

struct Stub;

impl VoiceCatalog for Stub {
    fn by_name(&self, name: &str) -> Option<String> {
        matches!(name, "fr" | "de" | "en").then(|| name.to_string())
    }

    fn select(&self, sel: &VoiceSelect) -> Option<String> {
        let lang: String = sel.language.chars().filter(|c| *c > ' ').take(2).collect();
        Some(if lang.is_empty() { sel.name.clone() } else { lang })
    }
}

fn defaults() -> [i32; N_SPEECH_PARAM] {
    let mut d = [0; N_SPEECH_PARAM];
    d[1] = 175;
    d[2] = 100;
    d[3] = 50;
    d[4] = 50;
    d[10] = 100;
    d
}

fn state() -> SsmlState {
    let base = BaseVoice {
        identifier: Some("en".into()),
        name: "en".into(),
        languages: vec!["en".into()],
        priority: 5,
        gender: 1,
        ..BaseVoice::default()
    };
    let mut st = SsmlState::new(defaults(), base, "");
    st.voice.current_voice_id = "en".into();
    st
}

fn pause_factors(_wpm: i32) -> (i32, i32) {
    (12, 16)
}

fn no_sound(_: &str) -> Option<i32> {
    None
}

fn sound_3(name: &str) -> Option<i32> {
    (name == "a.wav").then_some(3)
}

fn tag_with(st: &mut SsmlState, tone: bool, load: &dyn Fn(&str) -> Option<i32>, tag: &str) -> (u32, String) {
    let env = SsmlEnv {
        voices: &Stub,
        tone_language: tone,
        n_outbuf: 500,
        pause_factors: &pause_factors,
        load_sound: load,
        uri_callback: None,
    };
    let chars: Vec<char> = tag.chars().collect();
    let mut out = String::new();
    let t = process_ssml_tag(st, &env, &chars, &mut out);
    (t, out)
}

fn tag(st: &mut SsmlState, text: &str) -> (u32, String) {
    tag_with(st, false, &no_sound, text)
}

#[test]
fn sentence_paragraph_and_html_kinds() {
    let mut st = state();
    assert_eq!(tag(&mut st, "s"), (0x80046, " ".into()));
    assert_eq!(tag(&mut st, "/s"), (0x80028, " ".into()));
    assert_eq!(tag(&mut st, "p"), (0x80046, " ".into()));
    assert_eq!(tag(&mut st, "/p"), (0x80046, " ".into()));
    assert_eq!(tag(&mut st, "h1"), (0x80046, " ".into()));
    assert_eq!(tag(&mut st, "/h2"), (0x80046, " ".into()));
    assert_eq!(tag(&mut st, "hr/"), (0x80046, " ".into()));
    assert_eq!(tag(&mut st, "li"), (0x4001e, " ".into()));
    assert_eq!(tag(&mut st, "/li"), (0x4001e, " ".into()));
    assert_eq!(tag(&mut st, "br/"), (0x4001e, " ".into()));
    assert_eq!(tag(&mut st, "/speak"), (0x80028, " ".into()));
}

#[test]
fn break_strength_and_time() {
    let mut st = state();
    assert_eq!(tag(&mut st, "break strength=\"weak\"/"), (0, " \u{1}2B".into()));
    assert_eq!(tag(&mut st, "break strength=\"x-weak\"/"), (0, " \u{1}1B".into()));
    assert_eq!(tag(&mut st, "break/"), (clause::NONE + 21, " ".into()));
    assert_eq!(tag(&mut st, "break strength=\"strong\"/"), (clause::NONE + 40, " ".into()));
    assert_eq!(tag(&mut st, "break strength=\"x-strong\"/"), (clause::NONE + 80, " ".into()));
    assert_eq!(tag(&mut st, "break strength=\"none\"/"), (0, " \u{1}0B".into()));
    assert_eq!(tag(&mut st, "break strength=bogus/"), (0, " \u{1}2B".into()));
}

#[test]
fn break_time_scales_by_pause_factors() {
    let mut st = state();
    assert_eq!(tag(&mut st, "break time=\"500ms\"/"), (clause::NONE + 800, " ".into()));
    assert_eq!(tag(&mut st, "break time=\"100ms\"/"), (clause::NONE + 213, " ".into()));
    assert_eq!(tag(&mut st, "break time=\"2s\"/"), (clause::NONE + 3200, " ".into()));
    st.speech_parameters[10] = 50;
    assert_eq!(tag(&mut st, "break time=\"500ms\"/"), (clause::NONE + 400, " ".into()));
}

#[test]
fn long_break_sets_the_pause_long_flag() {
    let mut st = state();
    let (t, _) = tag(&mut st, "break time=\"200s\"/");
    assert_eq!(t, clause::NONE | clause::PAUSE_LONG | 0xfff);
    let (t, _) = tag(&mut st, "break time=\"20s\"/");
    assert_eq!(t, clause::NONE | clause::PAUSE_LONG | 1000);
    let (t, _) = tag(&mut st, "break time=\"8s\"/");
    assert_eq!(t, clause::NONE | clause::PAUSE_LONG | 400);
}

#[test]
fn weak_break_with_time_still_ends_the_clause() {
    let mut st = state();
    let (t, out) = tag(&mut st, "break strength=\"weak\" time=\"500ms\"/");
    assert_eq!((t, out.as_str()), (clause::NONE + 800, " \u{1}2B"));
}

#[test]
fn break_time_stores_the_current_rate_as_the_base_rate() {
    let mut st = state();
    tag(&mut st, "prosody rate=\"slow\"");
    tag(&mut st, "break time=\"1s\"/");
    assert_eq!(st.param_stack[0].parameter[1], 140);
}

#[test]
fn say_as_modes() {
    let mut st = state();
    let say = |st: &mut SsmlState, t: &str| tag(st, t);
    assert_eq!(say(&mut st, "say-as interpret-as=\"characters\"").1, " \u{1}18Y");
    assert_eq!(say(&mut st, "say-as interpret-as=\"tts:char\"").1, " \u{1}20Y");
    assert_eq!(say(&mut st, "say-as interpret-as=\"telephone\"").1, " \u{1}193Y");
    assert_eq!(say(&mut st, "say-as interpret-as=\"tts:digits\"").1, " \u{1}193Y");
    assert_eq!(
        say(&mut st, "say-as interpret-as=\"tts:digits\" detail=\"2\"").1,
        " \u{1}66Y"
    );
    assert_eq!(say(&mut st, "say-as interpret-as=\"characters\" format=\"glyphs\"").1, " \u{1}19Y");
    assert_eq!(say(&mut st, "say-as interpret-as=\"date\"").1, " \u{1}-1Y");
    assert_eq!(st.sayas_mode, -1);
    assert_eq!(say(&mut st, "/say-as"), (0, " \u{1}Y".into()));
    assert_eq!(st.sayas_mode, 0);
}

#[test]
fn say_as_key_replaces_key_names_on_close() {
    let env = SsmlEnv {
        voices: &Stub,
        tone_language: false,
        n_outbuf: 500,
        pause_factors: &pause_factors,
        load_sound: &no_sound,
        uri_callback: None,
    };
    let mut st = state();
    let mut out = String::new();
    let open: Vec<char> = "say-as interpret-as=\"tts:key\"".chars().collect();
    process_ssml_tag(&mut st, &env, &open, &mut out);
    out.push_str("underscore");
    let close: Vec<char> = "/say-as".chars().collect();
    process_ssml_tag(&mut st, &env, &close, &mut out);
    assert_eq!(out, " \u{1}36Y\u{e05f}\u{1}Y");
}

#[test]
fn prosody_writes_embedded_commands_and_close_restores() {
    let mut st = state();
    assert_eq!(tag(&mut st, "prosody rate=\"slow\""), (0, " \u{1}140S".into()));
    assert_eq!(tag(&mut st, "/prosody"), (0, " \u{1}175S".into()));
    assert_eq!(tag(&mut st, "prosody volume=\"loud\""), (0, " \u{1}150A".into()));
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}100A");
    assert_eq!(tag(&mut st, "prosody pitch=\"+10%\"").1, " \u{1}55P");
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}50P");
    assert_eq!(tag(&mut st, "prosody pitch=\"+2st\"").1, " \u{1}56P");
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}50P");
    assert_eq!(tag(&mut st, "prosody pitch=\"80\"").1, " \u{1}80P");
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}50P");
    assert_eq!(tag(&mut st, "prosody rate=\"2\"").1, " \u{1}350S");
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}175S");
}

#[test]
fn prosody_emits_parameters_in_index_order_and_nests() {
    let mut st = state();
    let (_, out) = tag(&mut st, "prosody range=\"x-high\" rate=\"fast\" pitch=\"high\"");
    assert_eq!(out, " \u{1}218S\u{1}55P\u{1}90R");
    let (_, inner) = tag(&mut st, "prosody rate=\"+10%\"");
    assert_eq!(inner, " \u{1}239S");
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}218S");
    assert_eq!(tag(&mut st, "/prosody").1, " \u{1}175S\u{1}50P\u{1}50R");
}

#[test]
fn unmatched_close_changes_nothing() {
    let mut st = state();
    assert_eq!(tag(&mut st, "/prosody"), (0, " ".into()));
    assert_eq!(st.n_param_stack, 1);
}

#[test]
fn emphasis_sets_volume_and_level_or_range_for_tone_languages() {
    let mut st = state();
    assert_eq!(tag(&mut st, "emphasis"), (0, " \u{1}3F".into()));
    assert_eq!(tag(&mut st, "/emphasis").1, " \u{1}0F");
    assert_eq!(tag(&mut st, "emphasis level=\"strong\"").1, " \u{1}120A\u{1}4F");
    assert_eq!(tag(&mut st, "/emphasis").1, " \u{1}100A\u{1}0F");
    assert_eq!(tag(&mut st, "emphasis level=\"reduced\"").1, " \u{1}75A\u{1}2F");
    assert_eq!(tag(&mut st, "/emphasis").1, " \u{1}100A\u{1}0F");
    let mut st = state();
    let (_, out) = tag_with(&mut st, true, &no_sound, "emphasis");
    assert_eq!(out, " \u{1}110A\u{1}70R");
}

#[test]
fn phoneme_is_written_only_for_the_espeak_alphabet() {
    let mut st = state();
    assert_eq!(
        tag(&mut st, "phoneme alphabet=\"espeak\" ph=\"h@l'oU\""),
        (0, " [[h@l'oU]]".into())
    );
    assert_eq!(tag(&mut st, "phoneme alphabet=\"ipa\" ph=\"x\""), (0, " ".into()));
}

#[test]
fn style_sets_punctuation_and_capitals_options() {
    let mut st = state();
    assert_eq!(tag(&mut st, "tts:style field=\"punctuation\" mode=\"all\""), (0, " ".into()));
    assert_eq!(st.option_punctuation, 1);
    tag(&mut st, "tts:style field=\"capital_letters\" mode=\"spelling\"");
    assert_eq!(st.option_capitals, 2);
    tag(&mut st, "/tts:style");
    tag(&mut st, "/tts:style");
    assert_eq!((st.option_punctuation, st.option_capitals), (-1, 0));
}

#[test]
fn mark_adds_names_by_byte_offset() {
    let mut st = state();
    assert_eq!(tag(&mut st, "mark name=\"m1\"/"), (0, " \u{1}0M".into()));
    assert_eq!(tag(&mut st, "mark name=\"m2\"/"), (0, " \u{1}3M".into()));
    assert_eq!(st.names.get(3), Some("m2"));
    st.skip_marker = "m3".into();
    assert_eq!(tag(&mut st, "mark name=\"m3\"/"), (clause::NONE, " ".into()));
    assert!(st.clear_skipping_text);
}

#[test]
fn audio_plays_a_loaded_file_and_silences_the_text() {
    let mut st = state();
    let (t, out) = tag_with(&mut st, false, &sound_3, "audio src=\"a.wav\"");
    assert_eq!((t, out.as_str()), (clause::NONE, " \u{1}3I"));
    assert!(st.audio_text);
    assert_eq!(st.speech_parameters[0], 1);
    let (t, _) = tag_with(&mut st, false, &sound_3, "/audio");
    assert_eq!(t, clause::NONE);
    assert!(!st.audio_text);
    assert_eq!(st.speech_parameters[0], 0);
}

#[test]
fn audio_with_a_missing_file_keeps_the_text() {
    let mut st = state();
    let (t, out) = tag(&mut st, "audio src=\"nofile.wav\"");
    assert_eq!((t, out.as_str()), (clause::NONE, " "));
    assert_eq!(st.speech_parameters[0], 0);
    assert_eq!(tag(&mut st, "/audio").0, clause::NONE);
}

#[test]
fn audio_with_a_uri_callback_emits_the_name_offset() {
    let accept = |_: &str, _: Option<&str>| 0;
    let env = SsmlEnv {
        voices: &Stub,
        tone_language: false,
        n_outbuf: 500,
        pause_factors: &pause_factors,
        load_sound: &no_sound,
        uri_callback: Some(&accept),
    };
    let mut st = state();
    st.names.add("earlier");
    let mut out = String::new();
    let t: Vec<char> = "audio src=\"http://x/a.wav\"".chars().collect();
    assert_eq!(process_ssml_tag(&mut st, &env, &t, &mut out), clause::NONE);
    assert_eq!(out, " \u{1}8U");
    assert_eq!(st.speech_parameters[0], 1);
}

#[test]
fn self_closing_audio_pops_itself() {
    let mut st = state();
    let (t, _) = tag_with(&mut st, false, &sound_3, "audio src=\"a.wav\"/");
    assert_eq!(t, clause::NONE);
    assert!(!st.audio_text);
    assert_eq!(st.speech_parameters[0], 0);
}

#[test]
fn sub_and_ignored_elements_set_ignore_text() {
    let mut st = state();
    assert_eq!(tag(&mut st, "sub alias=\"world wide web\""), (0, " world wide web".into()));
    assert!(st.ignore_text);
    tag(&mut st, "/sub");
    assert!(!st.ignore_text);
    tag(&mut st, "script");
    assert!(st.ignore_text);
    tag(&mut st, "/metadata");
    assert!(!st.ignore_text);
}

#[test]
fn html_inline_tags_write_no_space_and_comments_write_one() {
    let mut st = state();
    assert_eq!(tag(&mut st, "b"), (0, String::new()));
    assert_eq!(tag(&mut st, "/em"), (0, String::new()));
    assert_eq!(tag(&mut st, "!-- c --"), (0, " ".into()));
    assert_eq!(tag(&mut st, "unknown x=\"y\""), (0, " ".into()));
}

#[test]
fn self_closing_tags_in_the_ignore_table_do_nothing() {
    let mut st = state();
    assert_eq!(tag(&mut st, "voice xml:lang=\"fr\"/"), (0, " ".into()));
    assert_eq!(tag(&mut st, "prosody rate=\"slow\"/"), (0, " ".into()));
    assert_eq!(tag(&mut st, "say-as interpret-as=\"characters\"/"), (0, " ".into()));
    assert_eq!(tag(&mut st, "s/").0, 0x80046);
    assert_eq!(st.voice.current_voice_id, "en");
}

#[test]
fn voice_with_the_base_language_is_no_change() {
    let mut st = state();
    assert_eq!(tag(&mut st, "voice xml:lang=\"en\""), (0, " ".into()));
    assert_eq!(tag(&mut st, "/voice"), (0, " ".into()));
}

#[test]
fn voice_change_and_return_to_base() {
    let mut st = state();
    assert_eq!(tag(&mut st, "voice xml:lang=\"fr\""), (clause::VOICE, " ".into()));
    assert_eq!(st.voice.current_voice_id, "fr");
    assert_eq!(tag(&mut st, "/voice"), (VOICE_CHANGE, " ".into()));
    assert_eq!(st.voice.current_voice_id, "en");
}

#[test]
fn nested_voice_overwrites_instead_of_stacking() {
    let mut st = state();
    assert_eq!(tag(&mut st, "voice xml:lang=\"de\"").0, clause::VOICE);
    assert_eq!(tag(&mut st, "voice name=\"fr\"").0, clause::VOICE);
    assert_eq!(st.voice.current_voice_id, "fr");
    assert_eq!(tag(&mut st, "/voice").0, VOICE_CHANGE);
    assert_eq!(st.voice.current_voice_id, "en");
    assert_eq!(st.voice.depth, 1);
}

#[test]
fn speak_with_a_language_changes_voice_and_close_reverts() {
    let mut st = state();
    assert_eq!(tag(&mut st, "speak xml:lang=\"fr\"").0, clause::VOICE);
    assert_eq!(tag(&mut st, "/speak").0, clause::PERIOD + VOICE_CHANGE);
}

#[test]
fn sentence_language_never_reverts() {
    let mut st = state();
    assert_eq!(tag(&mut st, "s xml:lang=\"de\"").0, 0x80046 + VOICE_CHANGE);
    assert_eq!(st.voice.current_voice_id, "de");
    assert_eq!(tag(&mut st, "/s").0, clause::PERIOD);
    assert_eq!(tag(&mut st, "/p").0, clause::PARAGRAPH);
    assert_eq!(st.voice.current_voice_id, "de");
}

#[test]
fn paragraph_without_a_language_has_no_voice_change() {
    let mut st = state();
    assert_eq!(tag(&mut st, "p").0, clause::PARAGRAPH);
    assert_eq!(tag(&mut st, "s lang=\"de\"").0, clause::PARAGRAPH);
}

#[test]
fn only_the_exact_xml_lang_attribute_is_read() {
    let buf: Vec<char> = "voice lang=\"fr\" xml:lang=\"de\"".chars().collect();
    let v = get_ssml_attribute(&buf, 5, "xml:lang").expect("attribute");
    assert_eq!(attr_copy(Some(&v), 20), "de");
    let buf: Vec<char> = "voice lang=\"fr\"".chars().collect();
    assert!(get_ssml_attribute(&buf, 5, "xml:lang").is_none());
    let buf: Vec<char> = "voice name=\"x\"".chars().collect();
    assert!(get_ssml_attribute(&buf, 5, "lang").is_none());
}

#[test]
fn attribute_values_quoted_unquoted_and_empty() {
    let buf: Vec<char> = "a b = 'x y' c=z/ d=/ e=\"\"".chars().collect();
    let get = |n: &str| get_ssml_attribute(&buf, 1, n);
    assert_eq!(attr_copy(get("b").as_ref(), 40), "x y");
    assert_eq!(attr_copy(get("c").as_ref(), 40), "z");
    assert_eq!(attr_copy(get("d").as_ref(), 40), "");
    assert_eq!(attr_copy(get("e").as_ref(), 40), "");
    assert!(get("zz").is_none());
    let weak: Vec<char> = "break strength=weak".chars().collect();
    assert!(!attr_eq(get_ssml_attribute(&weak, 5, "strength").as_ref(), "weak"));
}

#[test]
fn attribute_numbers_and_lookup() {
    let buf: Vec<char> = "x t=\"2s\" u=\"500ms\" n=\"abc\" m=\"7\"".chars().collect();
    let get = |n: &str| get_ssml_attribute(&buf, 1, n);
    assert_eq!(attr_number(get("t").as_ref(), 0, true), 2000);
    assert_eq!(attr_number(get("t").as_ref(), 0, false), 2);
    assert_eq!(attr_number(get("u").as_ref(), 0, true), 500);
    assert_eq!(attr_number(get("n").as_ref(), 9, true), 9);
    assert_eq!(attr_number(None, 4, false), 4);
    assert_eq!(attr_lookup(get("m").as_ref(), &[("7", 1)], -1), 1);
    assert_eq!(attr_lookup(get("n").as_ref(), &[("7", 1)], -1), -1);
}

#[test]
fn prosody_value_forms() {
    let buf: Vec<char> = "x a=\"+10%\" b=\"-3st\" c=\"1.5\" d=\"+20\" e=\"junk\" f=\"50%\"".chars().collect();
    let v = |n: &str| prosody_value(false, &get_ssml_attribute(&buf, 1, n).expect("attr"));
    assert_eq!(v("a"), (2, 110));
    assert_eq!(v("b"), (2, 84));
    assert_eq!(v("c"), (0, 1));
    assert_eq!(v("d"), (1, 20));
    assert_eq!(v("e"), (2, 100));
    assert_eq!(v("f"), (2, 50));
    let r = get_ssml_attribute(&buf, 1, "c").expect("attr");
    assert_eq!(prosody_value(true, &r), (2, 150));
}

#[test]
fn xml_references() {
    let mut c2 = 0;
    assert_eq!(parse_ssml_reference("lt", &mut c2), Some(0xe03c));
    assert_eq!(c2, ' ' as u32);
    let mut c2 = 'x' as u32;
    assert_eq!(parse_ssml_reference("amp", &mut c2), Some('&' as u32));
    assert_eq!(c2, 'x' as u32);
    assert_eq!(parse_ssml_reference("quot", &mut 0), Some('"' as u32));
    assert_eq!(parse_ssml_reference("#x41", &mut 0), Some(0x41));
    assert_eq!(parse_ssml_reference("#65", &mut 0), Some(65));
    assert_eq!(parse_ssml_reference("#12abc", &mut 0), Some(12));
    assert_eq!(parse_ssml_reference("#x", &mut 0), None);
    assert_eq!(parse_ssml_reference("bogus", &mut 0), None);
}
