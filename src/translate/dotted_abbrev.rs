//! C `CheckDottedAbbrev` (translateword.c): a one-letter word followed by `. ` and more
//! one-letter words is a run of dotted initials, which C copies without the dots into the first
//! word and spells (`a . b` is `ab`).  C tries it only when no multi-word list entry matched the
//! first letter.

use super::Token;

/// C copies the letters into `word_buf[80]` and gives up on a run that does not fit.
const MAX_LETTER_BYTES: usize = 79;
const MAX_LOOKAHEAD: usize = 2 * MAX_LETTER_BYTES + 4;

/// The initials of a run, and the index of the last token the run takes in.
pub(super) struct Run {
    pub letters: String,
    pub last: usize,
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
                blank = matches!(tokens.get(ix + 1), Some(Token::Word(_)));
                continue;
            }
            Token::ClauseBoundary(_) => break,
            _ => {
                if !blank {
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

/// Whether the word at `from` starts a run of at least two dotted initials.
pub(super) fn check(tokens: &[Token], from: usize) -> Option<Run> {
    let words = clause_words(tokens, from);
    let mut letters = String::new();
    let mut count = 0;
    let mut last = from;
    let mut at = 0;
    while let Some(&(ix, word)) = words.get(at).filter(|(_, w)| is_letter(w)) {
        let dot = words.get(at + 1).is_some_and(|&(_, d)| d == ".");
        let ok = match (dot, words.get(at + 2).is_some(), count > 0) {
            (true, true, _) => 1,
            (true, false, _) | (false, _, false) => 0,
            (false, _, true) => 2,
        };
        if ok == 0 {
            break;
        }
        if letters.len() + word.len() > MAX_LETTER_BYTES {
            return None;
        }
        letters.push_str(word);
        count += 1;
        last = ix;
        if ok == 2 {
            break;
        }
        at += 2;
    }
    (count > 1).then_some(Run { letters, last })
}
