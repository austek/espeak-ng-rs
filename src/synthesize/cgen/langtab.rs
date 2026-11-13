//! The synthesis settings `SelectTranslator` (tr_languages.c) gives each language.

type Lengths = [i32; 8];

/// What a language changes from `NewTranslator`'s defaults.
#[derive(Clone, Copy, Default)]
pub struct Overrides {
    pub lengths: Option<Lengths>,
    pub amps: Option<Lengths>,
    pub equal_mods: bool,
    pub mods0_is_mods: bool,
    pub lengthen_tonic: Option<i32>,
    pub max_lengthmod: Option<i32>,
    pub long_stop: Option<i32>,
    pub word_gap: Option<i32>,
    pub sonorant_min: Option<i32>,
    pub long_vowel_threshold: Option<i32>,
    pub word_merge: Option<i32>,
    pub intonation_group: Option<usize>,
    pub tone: Option<ToneLang>,
}

/// A language with `langopts.tone_language`, and the sandhi rules its
/// `CalcPitches_Tone` pass applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToneLang {
    Vietnamese,
    Mandarin,
    Hakka,
    Other,
}

const EQUAL_L: Lengths = [230, 230, 230, 230, 0, 0, 230, 230];
const EQUAL_A: Lengths = [19; 8];
const FR_L: Lengths = [190, 170, 190, 200, 0, 0, 190, 240];
const FR_A: Lengths = [18, 16, 18, 18, 18, 18, 18, 18];
const SK_A: Lengths = [17, 16, 20, 20, 20, 22, 22, 21];
const SK_L: Lengths = [190, 190, 210, 210, 0, 0, 210, 210];
const TA_L: Lengths = [200, 200, 210, 210, 0, 0, 230, 230];
const TA2_L: Lengths = [230, 230, 240, 240, 0, 0, 260, 260];
const TA_A: Lengths = [18, 18, 18, 18, 20, 20, 22, 22];
const EO_L: Lengths = [150, 140, 180, 180, 0, 0, 200, 200];
const EO_A: Lengths = [16, 14, 20, 20, 20, 22, 22, 21];
const HR_A: Lengths = [17, 17, 20, 20, 20, 22, 22, 21];
const TR_A: Lengths = [18, 16, 20, 21, 20, 21, 21, 20];
const RU_L: Lengths = [150, 140, 220, 220, 0, 0, 260, 280];
const RU_A: Lengths = [16, 16, 18, 18, 20, 24, 24, 22];
const FI_L: Lengths = [150, 180, 200, 200, 0, 0, 210, 250];
const FI_A: Lengths = [18, 16, 22, 22, 20, 22, 22, 22];

fn la(lengths: Lengths, amps: Lengths) -> Overrides {
    Overrides { lengths: Some(lengths), amps: Some(amps), ..Overrides::default() }
}

fn l(lengths: Lengths) -> Overrides {
    Overrides { lengths: Some(lengths), ..Overrides::default() }
}

fn la0(lengths: Lengths, amps: Lengths) -> Overrides {
    Overrides { mods0_is_mods: true, ..la(lengths, amps) }
}

fn la_equal_mods(lengths: Lengths, amps: Lengths) -> Overrides {
    Overrides { equal_mods: true, ..la(lengths, amps) }
}

fn latin_family(lang: &str) -> Option<Overrides> {
    Some(match lang {
        "es" | "an" | "ca" | "ia" | "pap" => Overrides {
            sonorant_min: Some(120),
            ..la0([160, 145, 155, 150, 0, 0, 200, 245], [16, 14, 15, 16, 20, 20, 22, 22])
        },
        "fr" | "am" => la0(FR_L, FR_A),
        "tt" => la(FR_L, FR_A),
        "it" => Overrides {
            sonorant_min: Some(130),
            ..la0([160, 140, 150, 165, 0, 0, 218, 305], [17, 15, 18, 16, 20, 22, 22, 22])
        },
        "pt" => la0([170, 115, 210, 240, 0, 0, 260, 280], [16, 11, 19, 21, 20, 22, 22, 21]),
        "ro" | "rup" => la([170, 170, 180, 180, 0, 0, 240, 260], [15, 13, 18, 18, 20, 22, 22, 21]),
        "eo" | "io" => la(EO_L, EO_A),
        "eu" => la([200, 200, 200, 200, 0, 0, 210, 230], [16, 16, 18, 18, 18, 18, 18, 18]),
        "sq" => la([150, 150, 180, 180, 0, 0, 300, 300], [16, 12, 16, 16, 20, 20, 21, 19]),
        "gn" => Overrides { mods0_is_mods: true, ..Overrides::default() },
        _ => return None,
    })
}

fn germanic_family(lang: &str) -> Option<Overrides> {
    Some(match lang {
        "af" => l([170, 140, 220, 220, 0, 0, 250, 270]),
        "da" => l([160, 140, 200, 200, 0, 0, 220, 230]),
        "nb" | "no" => l([160, 140, 200, 200, 0, 0, 220, 230]),
        "is" => l([180, 160, 200, 200, 0, 0, 240, 250]),
        "nl" => l([160, 135, 210, 210, 0, 0, 260, 280]),
        "sv" => la([160, 135, 220, 220, 0, 0, 250, 280], [16, 16, 20, 20, 20, 22, 22, 21]),
        "cy" => Overrides {
            sonorant_min: Some(120),
            ..la([170, 220, 180, 180, 0, 0, 250, 270], [17, 15, 18, 18, 0, 0, 22, 20])
        },
        "en" => l([182, 140, 220, 220, 0, 0, 248, 275]),
        "de" => Overrides {
            word_gap: Some(0x8),
            long_vowel_threshold: Some(175 / 2),
            ..la([150, 130, 200, 200, 0, 0, 270, 270], [20, 20, 20, 20, 20, 22, 22, 20])
        },
        _ => return None,
    })
}

fn slavic_family(lang: &str) -> Option<Overrides> {
    Some(match lang {
        "ru" | "uk" => la(RU_L, RU_A),
        "be" => la([160, 140, 200, 140, 0, 0, 240, 160], [12, 10, 8, 8, 0, 0, 16, 17]),
        "sk" | "cs" => la(SK_L, SK_A),
        "pl" => la([160, 190, 175, 175, 0, 0, 200, 210], [17, 13, 19, 19, 20, 22, 22, 21]),
        "hr" | "bs" => la([180, 160, 200, 200, 0, 0, 220, 230], HR_A),
        "sr" => la([160, 150, 200, 200, 0, 0, 250, 260], HR_A),
        "mk" => la([180, 160, 200, 200, 0, 0, 220, 230], HR_A),
        "lv" | "ltg" => la([180, 180, 180, 160, 0, 0, 230, 180], [14, 10, 10, 8, 0, 0, 20, 15]),
        "smj" => Overrides {
            long_stop: Some(130),
            intonation_group: Some(3),
            ..la(FI_L, FI_A)
        },
        _ => return None,
    })
}

fn asian_family(lang: &str) -> Option<Overrides> {
    Some(match lang {
        "bn" | "as" | "bpy" => la0([180, 180, 210, 210, 0, 0, 230, 240], TA_A),
        "hi" | "or" | "pa" | "mr" => la0([190, 190, 210, 210, 0, 0, 230, 250], [17, 14, 20, 19, 20, 22, 22, 21]),
        "gu" | "ne" => la0(EQUAL_L, EQUAL_A),
        "ta" => Overrides { word_merge: Some(1), ..la0(TA_L, TA_A) },
        "kn" | "te" => la0(TA2_L, TA_A),
        "ml" => la0([180, 160, 240, 240, 0, 0, 260, 260], EQUAL_A),
        "ka" => la(TA_L, TA_A),
        "si" => la0(TA_L, TA_A),
        "id" | "ms" => la([160, 200, 180, 180, 0, 0, 220, 240], [16, 18, 18, 18, 20, 22, 22, 21]),
        "vi" => Overrides {
            word_gap: Some(0x21),
            tone: Some(ToneLang::Vietnamese),
            ..la0([150, 150, 180, 180, 210, 230, 230, 240], [16, 16, 16, 16, 22, 22, 22, 22])
        },
        "zh" | "cmn" => Overrides {
            word_gap: Some(0x21),
            tone: Some(ToneLang::Mandarin),
            ..la0([230, 150, 230, 230, 230, 0, 240, 250], [22, 16, 22, 22, 22, 22, 22, 22])
        },
        "yue" => Overrides {
            word_gap: Some(0x21),
            tone: Some(ToneLang::Other),
            ..la0([230, 150, 230, 230, 230, 0, 240, 250], [22, 16, 22, 22, 22, 22, 22, 22])
        },
        "shn" => Overrides { mods0_is_mods: true, tone: Some(ToneLang::Other), ..Overrides::default() },
        "hak" => Overrides { tone: Some(ToneLang::Hakka), ..Overrides::default() },
        _ => return None,
    })
}

fn other_family(lang: &str) -> Option<Overrides> {
    Some(match lang {
        "el" | "grc" => Overrides {
            sonorant_min: Some(130),
            ..la0([155, 180, 210, 210, 0, 0, 270, 300], [15, 12, 20, 20, 20, 22, 22, 21])
        },
        "hu" => la_equal_mods([185, 195, 195, 190, 0, 0, 210, 220], [17, 17, 19, 19, 20, 22, 22, 21]),
        "hy" => l([250, 200, 250, 250, 0, 0, 250, 250]),
        "jbo" => Overrides { max_lengthmod: Some(368), ..l([145, 145, 170, 160, 0, 0, 330, 350]) },
        "kk" => Overrides {
            lengthen_tonic: Some(0),
            ..la_equal_mods([190, 180, 230, 230, 0, 0, 250, 250], TR_A)
        },
        "kl" => la(EQUAL_L, EQUAL_A),
        "ku" => la([180, 180, 190, 180, 0, 0, 230, 240], [18, 18, 20, 20, 20, 22, 22, 21]),
        "mn" => Overrides {
            lengthen_tonic: Some(0),
            word_gap: Some(1),
            ..la_equal_mods([190, 180, 220, 220, 0, 0, 260, 240], [16, 16, 17, 17, 20, 20, 22, 18])
        },
        "om" => la([200, 200, 200, 200, 0, 0, 200, 200], [18, 15, 20, 20, 20, 22, 22, 22]),
        "sw" | "tn" => la0([160, 170, 200, 200, 0, 0, 320, 340], [16, 12, 19, 19, 20, 22, 22, 21]),
        "tr" | "az" => la([190, 180, 200, 230, 0, 0, 240, 250], TR_A),
        "fi" | "et" => Overrides { long_stop: Some(130), ..Overrides::default() },
        _ => return None,
    })
}

/// The overrides `SelectTranslator` applies for the primary language code.
pub fn overrides_for(lang: &str) -> Overrides {
    [latin_family, germanic_family, slavic_family, asian_family, other_family]
        .iter()
        .find_map(|f| f(lang))
        .unwrap_or_default()
}
