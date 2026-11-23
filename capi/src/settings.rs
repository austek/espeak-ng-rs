//! Plain-data session settings and the pure mapping from them to an engine.

use std::path::PathBuf;
use std::sync::OnceLock;

use espeak_ng::synthesize::cgen::SynthGlobals;
use espeak_ng::voices::VoiceEntry;
use espeak_ng::{EspeakNg, Parameter};

use crate::types::{N_SPEECH_PARAM, PARAM_DEFAULTS};

/// C's `phoneme_list` and wave generator are process-wide statics that survive
/// `espeak_Initialize` and `espeak_Terminate`, so every engine built for a call shares them.
pub fn process_globals() -> SynthGlobals {
    static GLOBALS: OnceLock<SynthGlobals> = OnceLock::new();
    GLOBALS.get_or_init(SynthGlobals::default).clone()
}

pub const PARAM_RATE: usize = 1;
pub const PARAM_VOLUME: usize = 2;
pub const PARAM_PITCH: usize = 3;
pub const PARAM_RANGE: usize = 4;
pub const PARAM_PUNCTUATION: usize = 5;
pub const PARAM_CAPITALS: usize = 6;
pub const PARAM_WORDGAP: usize = 7;
pub const PARAM_SSML_BREAK: usize = 10;

#[derive(Clone)]
pub struct Settings {
    pub data_dir: PathBuf,
    pub voice: String,
    pub voice_file: Option<VoiceEntry>,
    pub params: [i32; N_SPEECH_PARAM],
    pub punct_list: Vec<char>,
    /// C's `option_phoneme_events`: bit 0 reports phoneme events, bit 1 names them in IPA.
    pub phoneme_events: u32,
}

pub const PHONEME_EVENTS: u32 = 1;
pub const PHONEME_IPA: u32 = 2;

impl Settings {
    pub fn new(data_dir: PathBuf) -> Self {
        Settings {
            data_dir,
            voice: "en".to_string(),
            voice_file: None,
            params: PARAM_DEFAULTS,
            punct_list: Vec::new(),
            phoneme_events: 0,
        }
    }

    /// `espeak_ng_SetPhonemeEvents`: `ipa` only counts when `enable` is set.
    pub fn set_phoneme_events(&mut self, enable: bool, ipa: bool) {
        self.phoneme_events = if enable { PHONEME_EVENTS | (u32::from(ipa) * PHONEME_IPA) } else { 0 };
    }

    /// `espeak_Initialize`'s `options`: either bit turns the events on.
    pub fn set_initialize_options(&mut self, options: i32) {
        self.phoneme_events = options as u32 & (PHONEME_EVENTS | PHONEME_IPA);
    }

    /// C's `SetParameter`: absolute values are stored unclamped; a relative
    /// value for the first five parameters is a percentage of the default.
    /// Returns false for the parameters C reports as `EINVAL`.
    pub fn set_param(&mut self, index: usize, value: i32, relative: bool) -> bool {
        let new_value = match (relative, index < 5) {
            (true, true) => {
                let d = PARAM_DEFAULTS[index];
                d.saturating_add(d.saturating_mul(value) / 100)
            }
            _ => value,
        };
        self.params[index] = new_value;
        matches!(index, PARAM_RATE..=PARAM_RANGE | PARAM_WORDGAP | 9 | PARAM_SSML_BREAK)
    }

    pub fn build_engine(&self) -> espeak_ng::Result<EspeakNg> {
        let mut engine = EspeakNg::with_data_dir(&self.voice, &self.data_dir)?;
        engine.set_globals(process_globals());
        let p = &self.params;
        engine.set_parameter(Parameter::Rate, p[PARAM_RATE]);
        engine.set_parameter(Parameter::Volume, p[PARAM_VOLUME]);
        engine.set_parameter(Parameter::Pitch, p[PARAM_PITCH]);
        engine.set_parameter(Parameter::Range, p[PARAM_RANGE]);
        engine.set_parameter(Parameter::Capitals, p[PARAM_CAPITALS]);
        engine.set_parameter(Parameter::WordGap, p[PARAM_WORDGAP]);
        engine.set_parameter(Parameter::SsmlBreak, p[PARAM_SSML_BREAK]);
        engine.set_punctuation_list(self.punct_chars());
        engine.set_voice_pitch(self.voice_file.as_ref().and_then(|v| v.pitch));
        engine.set_voice_file(self.voice_file.clone());
        engine.set_phoneme_events(self.phoneme_events & PHONEME_EVENTS != 0, self.phoneme_events & PHONEME_IPA != 0);
        Ok(engine)
    }

    fn punct_chars(&self) -> Option<Vec<char>> {
        match self.params[PARAM_PUNCTUATION] {
            1 => Some(Vec::new()),
            2 if !self.punct_list.is_empty() => Some(self.punct_list.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_rate_is_percent_of_default() {
        let mut s = Settings::new(PathBuf::from("."));
        assert!(s.set_param(PARAM_RATE, 7, true));
        assert_eq!(s.params[PARAM_RATE], 187);
    }

    #[test]
    fn ipa_alone_enables_events_only_through_initialize() {
        let mut s = Settings::new(PathBuf::from("."));
        s.set_initialize_options(2);
        assert_eq!(s.phoneme_events, PHONEME_IPA);
        s.set_phoneme_events(false, true);
        assert_eq!(s.phoneme_events, 0);
        s.set_phoneme_events(true, true);
        assert_eq!(s.phoneme_events, PHONEME_EVENTS | PHONEME_IPA);
    }

    #[test]
    fn punctuation_reports_einval_but_stores() {
        let mut s = Settings::new(PathBuf::from("."));
        assert!(!s.set_param(PARAM_PUNCTUATION, 1, false));
        assert_eq!(s.params[PARAM_PUNCTUATION], 1);
    }
}
