//! C-faithful SSML clause reading (`readclause.c`, `ssml.c`). `ClauseReader` ports `ReadClause`
//! and reports, per clause, the 32-bit terminator that ended it.

pub mod attrs;
pub mod ssml_tag;
pub mod voice_stack;

pub(crate) mod ctype;
mod decoder;
mod entity;
mod iface;
mod punct;
mod reader;
mod ssml_tags;
mod terminator;

pub use ssml_tags::SsmlTags;
pub use iface::{CharMap, DefaultLang, NoTags, ReaderFlags, ReaderLang, ReaderOptions, TagProcessor};
pub use reader::ClauseReader;
pub use terminator::Terminator;

/// What `ReadClause` leaves for a bracket whose name it has already announced
/// (`0xe000 + '('`); the translator turns it into the announced-bracket pause.
pub const ANNOUNCED_BRACKET: char = '\u{e028}';

/// One clause as C `ReadClause` leaves it in `source[]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceClause {
    pub text: String,
    /// C `clause_start_char`: `count_characters` before the clause was read, at least 0.
    pub start_char: i32,
    /// `count_characters` after the clause was read.
    pub end_char: i32,
    /// Character index of each byte position after a written character, `-1` inside a
    /// multibyte character.
    pub charix: Vec<i32>,
    pub terminator: Terminator,
    pub tone_override: Option<u8>,
    pub voice_change: Option<String>,
    /// C `Eof()` once the clause is read.
    pub eof: bool,
    pub upper_count: u32,
    pub lower_count: u32,
}
