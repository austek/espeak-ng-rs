//! The `Replace:` lines C prints to the phoneme trace stream (`espeakPHONEMES_TRACE`) when a
//! word's list entry is replacement text.
//!
//! The word lookup sits too deep to hand a sink down, so it notes the line here and the
//! clause translation takes the lines of each token as it goes.

use std::cell::{Cell, RefCell};

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static LINES: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// Runs `f` with the lines recorded, so the clause codes it makes carry them.
fn tracing<T>(f: impl FnOnce() -> T) -> T {
    let was = ACTIVE.replace(true);
    let out = f();
    ACTIVE.set(was);
    out
}

impl super::Translator {
    /// The `Replace:` lines C prints for each clause of `text` while it translates the clause,
    /// when the phoneme trace has bit 3 set.
    pub fn replace_trace_lines(&self, text: &str) -> crate::error::Result<Vec<Vec<String>>> {
        let codes = tracing(|| self.translate_to_clause_codes(text))?;
        let mut clauses = vec![Vec::new()];
        for code in &codes {
            if let Some(super::CodeMarker::Replaced(line)) = &code.marker {
                clauses.last_mut().into_iter().for_each(|lines| lines.push(line.clone()));
            } else if code.is_boundary && code.code == 0 && code.clause_char.is_some() {
                clauses.push(Vec::new());
            }
        }
        Ok(clauses)
    }
}

pub(crate) fn active() -> bool {
    ACTIVE.get()
}

/// Notes C's `Replace: <word>  <replacement>`; both texts keep the space C leaves after them.
pub(crate) fn note(word: &str, replacement: &str) {
    if active() {
        LINES.with_borrow_mut(|lines| lines.push((word.to_string(), replacement.to_string())));
    }
}

/// The entry matched `rest` after its first word too: C's word text spans them all.
pub(crate) fn extend_first_word(rest: &str) {
    LINES.with_borrow_mut(|lines| lines.first_mut().into_iter().for_each(|(word, _)| word.push_str(&format!("{rest} "))));
}

/// The lines noted since the last call.
pub(crate) fn take() -> Vec<String> {
    let lines = LINES.with_borrow_mut(std::mem::take);
    lines.into_iter().map(|(word, replacement)| format!("Replace: {word}  {replacement}")).collect()
}
