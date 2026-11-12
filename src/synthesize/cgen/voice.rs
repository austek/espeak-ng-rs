//! The synthesis-relevant parts of C's `voice_t` and `Translator`.

use crate::synthesize::wavegen::{set_tone_adjust, DEFAULT_TONE_POINTS, N_TONE_ADJUST};

/// C's `voice_t`, as far as the formant generator reads it.
#[derive(Clone)]
pub struct WVoice {
    pub pitch_base: i32,
    pub pitch_range: i32,
    pub formant_factor: i32,
    pub speed_percent: i32,
    pub echo_delay: i32,
    pub echo_amp: i32,
    pub flutter: i32,
    pub n_harmonic_peaks: i32,
    pub peak_shape: i32,
    pub voicing: i32,
    pub consonant_amp: i32,
    pub consonant_ampv: i32,
    pub roughness: i32,
    pub freq: [i32; 9],
    pub freq2: [i32; 9],
    pub height: [i32; 9],
    pub height2: [i32; 9],
    pub width: [i32; 9],
    pub freqadd: [i32; 9],
    /// Breath noise per formant; `breath[0]` is non-zero when any is set.
    pub breath: [i32; 9],
    pub breathw: [i32; 9],
    pub tone_adjust: [u8; N_TONE_ADJUST],
    pub speedf1: i32,
    pub speedf2: i32,
    pub speedf3: i32,
}

impl Default for WVoice {
    /// `VoiceReset`.
    fn default() -> Self {
        const HEIGHTS: [i32; 9] = [130, 128, 120, 116, 100, 100, 128, 128, 128];
        const WIDTHS: [i32; 9] = [140, 128, 128, 160, 171, 171, 128, 128, 128];
        let height = HEIGHTS.map(|h| h * 2);
        let mut width = WIDTHS.map(|w| w * 2);
        width[0] = width[0] * 105 / 100;
        WVoice {
            pitch_base: 0x47000,
            pitch_range: 4104,
            formant_factor: 256,
            speed_percent: 100,
            echo_delay: 0,
            echo_amp: 0,
            flutter: 64,
            n_harmonic_peaks: 5,
            peak_shape: 0,
            voicing: 64,
            consonant_amp: 90,
            consonant_ampv: 100,
            roughness: 2,
            freq: [256; 9],
            freq2: [256; 9],
            height,
            height2: height,
            width,
            freqadd: [0; 9],
            breath: [0; 9],
            breathw: [0, 200, 200, 400, 400, 400, 600, 600, 600],
            tone_adjust: set_tone_adjust(&DEFAULT_TONE_POINTS),
            speedf1: 256,
            speedf2: 238,
            speedf3: 232,
        }
    }
}

impl WVoice {
    /// The `pitch <base> <range>` keyword.
    pub fn set_pitch(&mut self, pitch1: i32, pitch2: i32) {
        self.pitch_base = (pitch1 - 9) << 12;
        self.pitch_range = (pitch2 - pitch1) * 108;
        let factor = (pitch1 - 82) as f64 / 82.0;
        self.formant_factor = ((1.0 + factor / 4.0) * 256.0) as i32;
    }

    /// The `formant <n> <freq> <height> <width> [<add>]` keyword, percentages.
    pub fn set_formant(&mut self, formant: usize, freq: i32, height: i32, width: i32, add: i32) {
        if formant > 8 {
            return;
        }
        if freq >= 0 {
            self.freq[formant] = (freq as f64 * 2.56001) as i32;
            self.freq2[formant] = self.freq[formant];
        }
        if height >= 0 {
            self.height[formant] = (height as f64 * 2.56001) as i32;
            self.height2[formant] = self.height[formant];
        }
        if width >= 0 {
            self.width[formant] = (width as f64 * 2.56001) as i32;
        }
        self.freqadd[formant] = add;
        if formant == 0 {
            self.width[0] = self.width[0] * 105 / 100;
        }
    }
}

/// `length_mods_en`: vowel length by the two following phonemes, not last syllable.
pub static LENGTH_MODS_EN: [u8; 100] = [
    100, 120, 100, 105, 100, 110, 110, 100, 95, 100, 105, 120, 105, 110, 125, 130, 135, 115, 125,
    100, 105, 120, 75, 100, 75, 105, 120, 85, 75, 100, 105, 120, 85, 105, 95, 115, 120, 100, 95,
    100, 110, 120, 95, 105, 100, 115, 120, 100, 100, 100, 105, 120, 100, 105, 95, 115, 120, 110,
    95, 100, 105, 120, 100, 105, 105, 122, 125, 110, 105, 100, 105, 120, 100, 105, 105, 122, 125,
    110, 105, 100, 105, 120, 95, 105, 100, 115, 120, 110, 100, 100, 100, 120, 100, 100, 100, 100,
    100, 100, 100, 100,
];

/// `length_mods_en0`: as above, for the last syllable of a word.
pub static LENGTH_MODS_EN0: [u8; 100] = [
    100, 150, 100, 105, 110, 115, 110, 110, 110, 100, 105, 150, 105, 110, 125, 135, 140, 115, 135,
    100, 105, 150, 90, 105, 90, 122, 135, 100, 90, 100, 105, 150, 100, 105, 100, 122, 135, 100,
    100, 100, 105, 150, 100, 105, 105, 115, 135, 110, 105, 100, 105, 150, 100, 105, 105, 122, 130,
    120, 125, 100, 105, 150, 100, 105, 110, 122, 125, 115, 110, 100, 105, 150, 100, 105, 105, 122,
    135, 120, 105, 100, 105, 150, 100, 105, 105, 115, 135, 110, 105, 100, 100, 100, 100, 100, 100,
    100, 100, 100, 100, 100,
];

/// `length_mods_equal`.
pub static LENGTH_MODS_EQUAL: [u8; 100] = {
    let row = [110, 120, 100, 110, 110, 110, 110, 110, 110, 110];
    let mut t = [0u8; 100];
    let mut i = 0;
    while i < 100 {
        t[i] = row[i % 10];
        i += 1;
    }
    t
};

fn length_mod_tab(value: i32) -> &'static [u8; 100] {
    match value {
        0 | 1 => &LENGTH_MODS_EN,
        2 => &LENGTH_MODS_EN0,
        _ => &LENGTH_MODS_EQUAL,
    }
}

/// The `Translator` fields that `CalcLengths`, `CalcPitches` and `Generate` read.
#[derive(Clone)]
pub struct LangSynth {
    pub stress_lengths: [i32; 8],
    pub stress_amps: [i32; 8],
    pub length_mods: &'static [u8; 100],
    pub length_mods0: &'static [u8; 100],
    pub lengthen_tonic: i32,
    pub max_lengthmod: i32,
    pub long_stop: i32,
    pub word_gap: i32,
    pub sonorant_min: i32,
    pub long_vowel_threshold: i32,
    pub maxamp_eoc: i32,
    pub word_merge: i32,
    pub stress_flags: u32,
    pub intonation_group: usize,
    pub tunes: [usize; 6],
}

impl Default for LangSynth {
    /// `NewTranslator`.
    fn default() -> Self {
        LangSynth {
            stress_lengths: [182, 140, 220, 220, 220, 240, 260, 280],
            stress_amps: [18, 18, 20, 20, 20, 22, 22, 20],
            length_mods: length_mod_tab(201 % 100),
            length_mods0: length_mod_tab(201 / 100),
            lengthen_tonic: 20,
            max_lengthmod: 500,
            long_stop: 100,
            word_gap: 0,
            sonorant_min: 95,
            long_vowel_threshold: 190 / 2,
            maxamp_eoc: 19,
            word_merge: 0,
            stress_flags: 0,
            intonation_group: 0,
            tunes: [0, 1, 2, 3, 0, 0],
        }
    }
}

use super::langtab::{overrides_for, Overrides};

impl LangSynth {
    /// `SelectTranslator`, for the fields this module uses.
    pub fn for_lang(lang: &str) -> Self {
        let mut t = LangSynth::default();
        let primary = lang.split(['-', '_']).next().unwrap_or(lang).to_ascii_lowercase();
        t.apply(&overrides_for(&primary));
        t
    }

    fn apply(&mut self, o: &Overrides) {
        self.stress_lengths = o.lengths.unwrap_or(self.stress_lengths);
        self.stress_amps = o.amps.unwrap_or(self.stress_amps);
        if o.equal_mods {
            self.length_mods = &LENGTH_MODS_EQUAL;
            self.length_mods0 = &LENGTH_MODS_EQUAL;
        }
        if o.mods0_is_mods {
            self.length_mods0 = self.length_mods;
        }
        self.lengthen_tonic = o.lengthen_tonic.unwrap_or(self.lengthen_tonic);
        self.max_lengthmod = o.max_lengthmod.unwrap_or(self.max_lengthmod);
        self.long_stop = o.long_stop.unwrap_or(self.long_stop);
        self.word_gap = o.word_gap.unwrap_or(self.word_gap);
        self.sonorant_min = o.sonorant_min.unwrap_or(self.sonorant_min);
        self.long_vowel_threshold = o.long_vowel_threshold.unwrap_or(self.long_vowel_threshold);
        self.word_merge = o.word_merge.unwrap_or(self.word_merge);
        self.intonation_group = o.intonation_group.unwrap_or(self.intonation_group);
    }
}
