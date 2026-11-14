//! [`TagProcessor`] over the `ProcessSsmlTag` port.

use super::iface::{ReaderFlags, TagProcessor};
use super::ssml_tag::{process_ssml_tag, SsmlEnv, SsmlState, P_SILENCE};
use super::Terminator;

pub struct SsmlTags<'a> {
    pub state: SsmlState,
    pub env: SsmlEnv<'a>,
}

impl<'a> SsmlTags<'a> {
    pub fn new(state: SsmlState, env: SsmlEnv<'a>) -> Self {
        SsmlTags { state, env }
    }

    fn load(&mut self, flags: &ReaderFlags) {
        let st = &mut self.state;
        st.ignore_text = flags.ignore_text;
        st.audio_text = flags.audio_text;
        st.sayas_mode = flags.sayas_mode as i32;
        st.sayas_start = flags.sayas_start;
        st.clear_skipping_text = flags.clear_skipping_text;
        st.option_punctuation = i32::from(flags.punctuation);
        st.option_capitals = flags.capitals as i32;
        if let Some(id) = st.voice.base.identifier.clone() {
            st.voice.current_voice_id = id;
        }
    }

    fn store(&self, flags: &mut ReaderFlags) {
        let st = &self.state;
        flags.ignore_text = st.ignore_text;
        flags.audio_text = st.audio_text;
        flags.sayas_mode = st.sayas_mode as u32;
        flags.sayas_start = st.sayas_start;
        flags.clear_skipping_text = st.clear_skipping_text;
        flags.punctuation = st.option_punctuation as u8;
        flags.capitals = st.option_capitals as u32;
        flags.silence = st.speech_parameters[P_SILENCE] == 1;
    }
}

impl TagProcessor for SsmlTags<'_> {
    fn process_tag(&mut self, tag: &[char], out: &mut String, flags: &mut ReaderFlags) -> Option<Terminator> {
        self.load(flags);
        let terminator = process_ssml_tag(&mut self.state, &self.env, tag, out);
        self.store(flags);
        (terminator != 0).then(|| Terminator::from_bits(terminator))
    }

    fn current_voice_id(&self) -> String {
        self.state.voice.current_voice_id.clone()
    }
}
