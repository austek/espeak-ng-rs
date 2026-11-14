//! The SSML voice stack from `ssml.c`: `GetVoiceAttributes` and `VoiceFromStack`.
//!
//! C passes `n_ssml_stack` to `GetVoiceAttributes` by value, so the `n++` that
//! pushes a frame and the `n--` that pops one change a copy. The frame is
//! written at `frames[n]`, but the real depth stays at its initial 1. This port
//! keeps that behaviour (see [`VoiceStack::depth`]).

use super::attrs::{attr_copy, attr_lookup, attr_number, get_ssml_attribute};
use crate::voices::{find_voice, select_voice_by_name, VoiceEntry, VoiceQuery};

pub const N_SSML_STACK: usize = 20;
pub const ENGENDER_UNKNOWN: i32 = 0;
pub const ENGENDER_MALE: i32 = 1;
pub const ENGENDER_FEMALE: i32 = 2;
pub const ENGENDER_NEUTRAL: i32 = 3;
/// `CLAUSE_TYPE_VOICE_CHANGE`.
pub const VOICE_CHANGE: u32 = 0x0002_0000;
/// `SSML_VOICE`, `SSML_SENTENCE`, `SSML_CLOSE`.
pub const TAG_VOICE: u32 = 2;
pub const TAG_CLOSE: u32 = 0x20;
const LANGUAGE_LEN: usize = 20;
const VOICE_NAME_LEN: usize = 40;

const GENDERS: [(&str, i32); 3] =
    [("male", ENGENDER_MALE), ("female", ENGENDER_FEMALE), ("neutral", ENGENDER_NEUTRAL)];

/// C `SSML_STACK`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SsmlFrame {
    pub tag_type: u32,
    pub variant: i32,
    pub gender: i32,
    pub age: i32,
    pub voice_name: String,
    pub language: String,
}

/// The parts of the base `espeak_VOICE` the stack reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaseVoice {
    pub identifier: Option<String>,
    pub name: String,
    /// Language names without their priority byte; the first is the main one.
    pub languages: Vec<String>,
    pub priority: u8,
    pub gender: i32,
    pub age: i32,
    pub variant: i32,
}

impl BaseVoice {
    /// `SetVoiceStack`: frame 0 holds the base voice. C `strcpy`s `v->languages`,
    /// which is the priority byte followed by the first language name.
    pub fn frame0(&self) -> SsmlFrame {
        let first = self.languages.first().cloned().unwrap_or_default();
        SsmlFrame {
            tag_type: 0,
            variant: self.variant,
            gender: self.gender,
            age: self.age,
            voice_name: self.name.chars().take(VOICE_NAME_LEN - 1).collect(),
            language: format!("{}{first}", char::from(self.priority)),
        }
    }
}

/// What `VoiceFromStack` hands to `SelectVoice`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceSelect {
    pub name: String,
    pub identifier: String,
    pub language: String,
    pub age: i32,
    pub gender: i32,
    pub variant: i32,
}

/// The voice list behind `SelectVoiceByName` and `SelectVoice`.
pub trait VoiceCatalog {
    /// Identifier of the voice whose name or file matches `name`.
    fn by_name(&self, name: &str) -> Option<String>;
    /// Identifier (possibly with a `+variant` suffix) that best matches `sel`.
    fn select(&self, sel: &VoiceSelect) -> Option<String>;
}

/// [`VoiceCatalog`] over the installed voice list. `select` scores by language
/// and gender only; C's `+variant` expansion for a non-zero `variant` is not ported.
pub struct EntryCatalog<'a>(pub &'a [VoiceEntry]);

impl VoiceCatalog for EntryCatalog<'_> {
    fn by_name(&self, name: &str) -> Option<String> {
        select_voice_by_name(self.0, name).map(|v| v.file.clone())
    }

    fn select(&self, sel: &VoiceSelect) -> Option<String> {
        let language: String = sel.language.chars().filter(|c| *c > ' ').collect();
        if language.is_empty() {
            let base = sel.name.split('+').next().unwrap_or("");
            return self.by_name(base).or_else(|| self.by_name("en"));
        }
        let gender = match sel.gender {
            ENGENDER_MALE => Some('M'),
            ENGENDER_FEMALE => Some('F'),
            _ => None,
        };
        let query = VoiceQuery { language: Some(language), name: None, gender };
        find_voice(self.0, &query).map(|v| v.file.clone()).or_else(|| self.by_name("en"))
    }
}

/// The SSML stack plus the voice-id state `GetVoiceAttributes` mutates.
#[derive(Clone, Debug)]
pub struct VoiceStack {
    pub frames: Vec<SsmlFrame>,
    /// `*n_ssml_stack`: never grows, because C increments a copy.
    pub depth: usize,
    pub current_voice_id: String,
    pub base: BaseVoice,
    pub base_variant: String,
    /// C's `static char identifier[40]` inside `VoiceFromStack`.
    identifier: String,
}

impl VoiceStack {
    pub fn new(base: BaseVoice, base_variant: &str) -> VoiceStack {
        let mut frames = vec![SsmlFrame::default(); N_SSML_STACK];
        frames[0] = base.frame0();
        VoiceStack {
            frames,
            depth: 1,
            current_voice_id: String::new(),
            base,
            base_variant: base_variant.to_string(),
            identifier: String::new(),
        }
    }

    /// `ssml_stack[*n_ssml_stack - 1].tag_type`.
    pub fn top_tag(&self) -> u32 {
        self.frames[self.depth - 1].tag_type
    }

    /// `GetVoiceAttributes`: [`VOICE_CHANGE`] when the resolved voice differs
    /// from `current_voice_id`, else 0. `buf`/`px` are the tag buffer and the
    /// index after the tag name.
    pub fn get_voice_attributes(
        &mut self,
        cat: &dyn VoiceCatalog,
        buf: &[char],
        px: usize,
        tag_type: u32,
    ) -> u32 {
        let mut n = self.depth;
        if tag_type & TAG_CLOSE != 0 {
            n = if n > 1 { n - 1 } else { n };
        } else {
            match self.frame_from_tag(buf, px, tag_type) {
                None => return 0,
                Some(frame) if n < N_SSML_STACK => {
                    self.frames[n] = frame;
                    n += 1;
                }
                Some(_) => return 0,
            }
        }
        let id = self.voice_from_stack(cat, n);
        if id == self.current_voice_id {
            return 0;
        }
        self.current_voice_id = id;
        VOICE_CHANGE
    }

    fn frame_from_tag(&self, buf: &[char], px: usize, tag_type: u32) -> Option<SsmlFrame> {
        let lang = get_ssml_attribute(buf, px, "xml:lang");
        if tag_type != TAG_VOICE && lang.is_none() {
            return None;
        }
        let attr = |name: &str| (tag_type == TAG_VOICE).then(|| get_ssml_attribute(buf, px, name)).flatten();
        let variant = attr_number(attr("variant").as_ref(), 1, false);
        Some(SsmlFrame {
            tag_type,
            variant: if variant > 0 { variant - 1 } else { variant },
            gender: attr_lookup(attr("gender").as_ref(), &GENDERS, ENGENDER_UNKNOWN),
            age: attr_number(attr("age").as_ref(), 0, false),
            voice_name: attr_copy(attr("name").as_ref(), VOICE_NAME_LEN),
            language: attr_copy(lang.as_ref(), LANGUAGE_LEN),
        })
    }

    /// `VoiceFromStack` over the first `n` frames.
    pub fn voice_from_stack(&mut self, cat: &dyn VoiceCatalog, n: usize) -> String {
        let f0 = self.frames[0].clone();
        let mut sel = VoiceSelect {
            name: f0.voice_name,
            identifier: String::new(),
            language: f0.language,
            age: f0.age,
            gender: f0.gender,
            variant: f0.variant,
        };
        for ix in 0..n {
            let frame = self.frames[ix].clone();
            self.apply_frame(cat, &frame, &mut sel);
        }
        sel.identifier = self.identifier.clone();
        let Some(id) = cat.select(&sel) else { return "default".to_string() };
        self.keep_base_variant(id, &sel)
    }

    fn apply_frame(&mut self, cat: &dyn VoiceCatalog, sp: &SsmlFrame, sel: &mut VoiceSelect) {
        let mut name_specified = false;
        if !sp.voice_name.is_empty() {
            if let Some(id) = cat.by_name(&sp.voice_name) {
                name_specified = true;
                sel.name = sp.voice_name.clone();
                self.identifier = id.chars().take(VOICE_NAME_LEN - 1).collect();
                sel.language.clear();
                sel.gender = ENGENDER_UNKNOWN;
                sel.age = 0;
                sel.variant = 0;
            }
        }
        if !sp.language.is_empty() {
            sel.language = self.base_language_for(&sp.language);
            if !name_specified {
                sel.name.clear();
                self.identifier.clear();
            }
        }
        if sp.gender != ENGENDER_UNKNOWN {
            sel.gender = sp.gender;
        }
        if sp.age != 0 {
            sel.age = sp.age;
        }
        if sp.variant != 0 {
            sel.variant = sp.variant;
        }
    }

    fn base_language_for(&self, language: &str) -> String {
        match self.base.languages.iter().any(|l| l == language) {
            true => self.base.languages[0].clone(),
            false => language.to_string(),
        }
    }

    fn keep_base_variant(&self, id: String, sel: &VoiceSelect) -> String {
        let unspecified = sel.gender == ENGENDER_UNKNOWN || sel.gender == self.base.gender;
        if !id.contains('+') && unspecified && !self.base_variant.is_empty() {
            let joined = format!("{id}+{}", self.base_variant);
            return joined.chars().take(VOICE_NAME_LEN - 1).collect();
        }
        id
    }
}
