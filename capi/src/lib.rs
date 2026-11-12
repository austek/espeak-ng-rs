//! C ABI for the pure Rust eSpeak NG port.
//!
//! Exports the `libespeak-ng` symbols declared in `include/espeak-ng/*.h`.
//! Pointer handling is confined to `ptr`, `voicetab` and `delivery`.

mod api;
mod blocks;
mod delivery;
mod ops;
mod phonemes;
mod ptr;
mod settings;
mod state;
mod types;
mod voicetab;
