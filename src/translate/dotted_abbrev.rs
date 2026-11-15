//! C `CheckDottedAbbrev` (translateword.c): a one-letter word followed by `. ` and more
//! one-letter words is a run of dotted initials, which C copies without the dots into the first
//! word and spells (`a . b` is `ab`).  C tries it only when no multi-word list entry matched the
//! first letter.

use super::Token;

/// C copies the letters into `word_buf[80]` and gives up on a run that does not fit.
const MAX_LETTER_BYTES: usize = 80;
const MAX_LOOKAHEAD: usize = 2 * MAX_LETTER_BYTES + 4;

/// The initials of a run, and the index of the last token the run takes in.
pub(super) struct Run {
    pub letters: String,
    pub last: usize,
    pub possessive: bool,
}

/// The words of C's clause text from `from` on: words and dots set apart by blanks.  A word
/// that no blank follows is not one the check can use, so it is left out.
pub(super) fn clause_words(tokens: &[Token], from: usize) -> Vec<(usize, &str)> {
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut blank = true;
    for (ix, token) in tokens.iter().enumerate().skip(from) {
        let word = match token {
            Token::Space => {
                blank = true;
                continue;
            }
            Token::Embedded(_) => continue,
            Token::Word(w) if blank && super::is_dotted_letters(w) => {
                let pieces = w.split('.').flat_map(|p| [p, "."]);
                words.extend(pieces.take(2 * w.split('.').count() - 1).map(|p| (ix, p)));
                blank = false;
                continue;
            }
            Token::Word(w) if blank => w.as_str(),
            Token::Punctuation('.') if blank => {
                words.push((ix, "."));
                blank = matches!(tokens.get(ix + 1), Some(Token::Word(_))) || apostrophe_s_after(tokens, ix);
                continue;
            }
            Token::ClauseBoundary(_) => break,
            _ => {
                if !blank && !apostrophe_s_after(tokens, ix.saturating_sub(1)) {
                    words.pop();
                }
                break;
            }
        };
        words.push((ix, word));
        blank = false;
        if words.len() > MAX_LOOKAHEAD {
            break;
        }
    }
    words
}

fn is_letter(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(char::is_alphabetic) && chars.next().is_none()
}

enum Scan {
    Run(Run),
    Short,
    Overflow,
}

/// Whether the word at `from` starts a run of at least two dotted initials.
pub(super) fn check(tokens: &[Token], from: usize) -> Option<Run> {
    match scan(tokens, from) {
        Scan::Run(run) => Some(run),
        Scan::Short | Scan::Overflow => None,
    }
}

/// Whether the run of initials at `from` is too long for C's word buffer, so that C leaves the
/// first letter to the word lookup instead of spelling it.
pub(super) fn overflows(tokens: &[Token], from: usize) -> bool {
    matches!(scan(tokens, from), Scan::Overflow)
}

fn scan(tokens: &[Token], from: usize) -> Scan {
    let words = clause_words(tokens, from);
    let mut letters = String::new();
    let mut count = 0;
    let mut last = from;
    let mut possessive = false;
    let mut at = 0;
    while let Some(&(ix, word)) = words.get(at).filter(|(_, w)| is_letter(w)) {
        let dot_at = words.get(at + 1).filter(|&&(_, d)| d == ".").map(|&(dot_ix, _)| dot_ix);
        let ok = match (dot_at, words.get(at + 2).is_some(), count > 0) {
            (Some(_), true, _) => 1,
            (Some(dot_ix), false, _) if apostrophe_s_after(tokens, dot_ix) => 3,
            (Some(_), false, _) | (None, _, false) => 0,
            (None, _, true) => 2,
        };
        if ok == 0 {
            break;
        }
        if letters.len() + word.len() > MAX_LETTER_BYTES {
            return Scan::Overflow;
        }
        letters.push_str(word);
        count += 1;
        last = if ok == 3 { dot_at.unwrap_or(ix) } else { ix };
        possessive = ok == 3 || (ok == 2 && apostrophe_s_after(tokens, ix));
        if ok >= 2 {
            break;
        }
        at += 2;
    }
    if count > 1 {
        Scan::Run(Run { letters, last, possessive })
    } else {
        Scan::Short
    }
}

/// C drops the final dot of `u.s.a.'s` (CheckDottedAbbrev, translateword.c:1195).
fn apostrophe_s_after(tokens: &[Token], dot: usize) -> bool {
    matches!(
        (tokens.get(dot + 1), tokens.get(dot + 2)),
        (Some(Token::Punctuation('\'')), Some(Token::Word(w))) if w.starts_with('s')
    )
}
