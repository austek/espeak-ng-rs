//! `SetSpeed` from `setlengths.c` (without libsonic).

use super::voice::WVoice;

const RATE_MIN: i32 = 80;
const RATE_MAX: i32 = 450;

static SPEED_LOOKUP: [u8; 280] = [
    255, 255, 255, 255, 255, 253, 249, 245, 242, 238, 235, 232, 228, 225, 222, 218, 216, 213, 210,
    207, 204, 201, 198, 196, 193, 191, 188, 186, 183, 181, 179, 176, 174, 172, 169, 168, 165, 163,
    161, 159, 158, 155, 153, 152, 150, 148, 146, 145, 143, 141, 139, 137, 136, 135, 133, 131, 130,
    129, 127, 126, 124, 123, 122, 120, 119, 118, 117, 115, 114, 113, 112, 111, 110, 109, 107, 106,
    105, 104, 103, 102, 101, 100, 99, 98, 97, 96, 95, 94, 93, 92, 91, 90, 89, 89, 88, 87, 86, 85,
    84, 83, 82, 82, 81, 80, 80, 79, 78, 77, 76, 76, 75, 75, 74, 73, 72, 71, 71, 70, 69, 69, 68, 67,
    67, 66, 66, 65, 64, 64, 63, 62, 62, 61, 61, 60, 59, 59, 58, 58, 57, 57, 56, 56, 55, 54, 54, 53,
    53, 52, 52, 52, 51, 50, 50, 49, 49, 48, 48, 47, 47, 46, 46, 46, 45, 45, 44, 44, 44, 43, 43, 42,
    41, 40, 40, 40, 39, 39, 39, 38, 38, 38, 37, 37, 37, 36, 36, 35, 35, 35, 35, 34, 34, 34, 33, 33,
    33, 32, 32, 31, 31, 31, 30, 30, 30, 29, 29, 29, 29, 28, 28, 27, 27, 27, 27, 26, 26, 26, 26, 25,
    25, 25, 24, 24, 24, 24, 23, 23, 23, 23, 22, 22, 22, 21, 21, 21, 21, 20, 20, 20, 20, 19, 19, 19,
    18, 18, 17, 17, 17, 16, 16, 16, 16, 16, 16, 15, 15, 15, 15, 14, 14, 14, 13, 13, 13, 12, 12, 12,
    12, 11, 11, 11, 11, 10, 10, 10, 9, 9, 9, 8, 8, 8,
];

static PAUSE_FACTOR_350: [u8; 25] = [
    22, 22, 22, 22, 22, 22, 22, 21, 21, 21, 21, 20, 20, 19, 19, 18, 17, 16, 15, 15, 15, 15, 15, 15, 15,
];

static WAV_FACTOR_350: [u8; 101] = [
    120, 121, 120, 119, 119, 118, 118, 117, 116, 116, 115, 114, 113, 112, 112, 111, 111, 110, 109,
    108, 107, 106, 106, 104, 103, 103, 102, 102, 102, 101, 101, 99, 98, 98, 97, 96, 96, 95, 94, 93,
    91, 90, 91, 90, 89, 88, 86, 85, 86, 85, 85, 84, 82, 81, 80, 79, 77, 78, 78, 76, 77, 75, 75, 74,
    73, 71, 72, 70, 69, 69, 69, 67, 65, 64, 63, 63, 63, 61, 61, 59, 59, 59, 58, 56, 57, 58, 56, 54,
    53, 52, 52, 53, 52, 52, 50, 48, 47, 47, 45, 46, 45,
];

/// C's `SPEED_FACTORS` plus the `len_speeds` table `CalcLengths` reads.
#[derive(Clone, Copy, Debug)]
pub struct Speed {
    pub pause_factor: i32,
    pub clause_pause_factor: i32,
    pub min_pause: i32,
    pub wav_factor: i32,
    pub lenmod_factor: i32,
    pub lenmod2_factor: i32,
    pub min_sample_len: i32,
    pub len_speeds: [i32; 3],
}

impl Default for Speed {
    fn default() -> Self {
        Speed {
            pause_factor: 0,
            clause_pause_factor: 0,
            min_pause: 5,
            wav_factor: 0,
            lenmod_factor: 110,
            lenmod2_factor: 100,
            min_sample_len: RATE_MAX,
            len_speeds: [130, 121, 118],
        }
    }
}

fn speed_multiplier(wpm: i32) -> (i32, i32) {
    let wpm = wpm.min(RATE_MAX);
    let wpm2 = wpm.clamp(RATE_MIN, 359);
    let mut x = SPEED_LOOKUP[(wpm2 - RATE_MIN) as usize] as i32;
    if wpm >= 380 {
        x = 7;
    }
    if wpm >= 400 {
        x = 6;
    }
    (x, wpm)
}

impl Speed {
    /// `SetSpeed(control)` for a rate of `wpm` words per minute.
    pub fn set(&mut self, control: i32, wpm: i32, voice: &WVoice) {
        self.min_sample_len = RATE_MAX;
        self.lenmod_factor = 110;
        self.lenmod2_factor = 100;
        self.min_pause = 5;
        let wpm = if voice.speed_percent > 0 { wpm * voice.speed_percent / 100 } else { wpm };
        let (x, wpm) = speed_multiplier(wpm);
        if control & 1 != 0 {
            self.set_factors(x, voice);
        }
        if control & 2 != 0 {
            self.set_mods(voice.speedf1, wpm, x);
        }
    }

    fn set_factors(&mut self, x: i32, voice: &WVoice) {
        self.len_speeds = [x * voice.speedf1 / 256, x * voice.speedf2 / 256, x * voice.speedf3 / 256];
        if x <= 7 {
            self.len_speeds = [x, x - 1, x - 1];
        }
    }

    fn set_mods(&mut self, speedf1: i32, wpm: i32, x: i32) {
        if wpm > 350 {
            self.lenmod_factor = 85 - (wpm - 350) / 3;
            self.lenmod2_factor = 60 - (wpm - 350) / 8;
        } else if wpm > 250 {
            self.lenmod_factor = 110 - (wpm - 250) / 4;
            self.lenmod2_factor = 110 - (wpm - 250) / 2;
        }
        let s1 = x * speedf1 / 256;
        self.wav_factor = if wpm >= 170 { 110 + (150 * s1) / 128 } else { 128 + (128 * s1) / 130 };
        if wpm >= 350 {
            self.wav_factor = WAV_FACTOR_350[(wpm - 350) as usize] as i32;
        }
        if wpm >= 390 {
            self.min_sample_len = RATE_MAX - (wpm - 400) / 2;
            if wpm > 440 {
                self.min_sample_len = 420 - (wpm - 440);
            }
        }
        self.pause_factor = (256 * s1) / 115;
        self.set_pause_factor(wpm);
    }

    fn set_pause_factor(&mut self, wpm: i32) {
        if wpm > 430 {
            self.pause_factor = 12;
        } else if wpm > 400 {
            self.pause_factor = 13;
        } else if wpm > 374 {
            self.pause_factor = 14;
        } else if wpm > 350 {
            self.pause_factor = PAUSE_FACTOR_350[(wpm - 350) as usize] as i32;
        }
        self.clause_pause_factor = self.pause_factor.max(16);
    }
}
