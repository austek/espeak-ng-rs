//! C-faithful SSML clause reading (`readclause.c`, `ssml.c`). `ClauseReader` ports `ReadClause`
//! and reports, per clause, the 32-bit terminator that ended it.

pub mod attrs;
pub mod ssml_tag;
pub mod voice_stack;

mod ctype;
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

/// One clause as C `ReadClause` leaves it in `source[]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceClause {
    pub text: String,
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
