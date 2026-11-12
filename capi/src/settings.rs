//! Plain-data session settings and the pure mapping from them to an engine.

use std::path::PathBuf;

use espeak_ng::voices::VoiceEntry;
use espeak_ng::{EspeakNg, Parameter};

use crate::types::{N_SPEECH_PARAM, PARAM_DEFAULTS};

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
}

impl Settings {
    pub fn new(data_dir: PathBuf) -> Self {
        Settings {
            data_dir,
            voice: "en".to_string(),
            voice_file: None,
            params: PARAM_DEFAULTS,
            punct_list: Vec::new(),
        }
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
    fn punctuation_reports_einval_but_stores() {
        let mut s = Settings::new(PathBuf::from("."));
        assert!(!s.set_param(PARAM_PUNCTUATION, 1, false));
        assert_eq!(s.params[PARAM_PUNCTUATION], 1);
    }
}
