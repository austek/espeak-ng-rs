//! `<say-as>` as C `TranslateClause` reads it: the `Y` embedded command switches how the
//! characters after it are cut into words (`option_sayas2`), and how each word is spoken
//! (`option_sayas`).

use super::{tokenize_opts, NumberGrammar, NumberToken, Token};

const CHARS_GROUP: i32 = 0x10;
const KEY: i32 = 0x24;
const DIGITS: i32 = 0x40;
const DIGITS1: i32 = 0xc1;
const SINGLE_CHARS: u8 = 4;

/// The say-as mode a clause starts in and the digit count `count_sayas_digits` carries.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SayAs {
    pub mode: i32,
    digits_seen: usize,
}

enum Piece<'a> {
    Text(&'a str),
    Command(&'a str),
}

/// Length of the embedded command at the start of `rest` (`\x01[+-]<digits><letter>`).
fn command_len(rest: &str) -> Option<usize> {
    let b = rest.as_bytes();
    let sign = usize::from(matches!(b.get(1), Some(b'+' | b'-')));
    let digits = b[1 + sign..].iter().take_while(|c| c.is_ascii_digit()).count();
    let at = 1 + sign + digits;
    b.get(at).filter(|c| c.is_ascii_uppercase()).map(|_| at + 1)
}

fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('\u{1}') {
        let tail = &rest[at..];
        match command_len(tail) {
            Some(len) => {
                out.push(Piece::Text(&rest[..at]));
                out.push(Piece::Command(&tail[..len]));
                rest = &tail[len..];
            }
            None => {
                out.push(Piece::Text(&rest[..at + 1]));
                rest = &tail[1..];
            }
        }
    }
    out.push(Piece::Text(rest));
    out
}

/// Tokens of one clause's text, cut the way the say-as mode in force cuts them.
pub(crate) fn tokenize_clause(text: &str, grammar: &NumberGrammar, state: &mut SayAs) -> Vec<Token> {
    let pieces = pieces(text);
    let sets_mode = pieces.iter().any(|p| matches!(p, Piece::Command(c) if c.ends_with('Y')));
    if state.mode == 0 && !sets_mode {
        return tokenize_opts(text, grammar);
    }
    let mut tokens = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Command(c) => {
                let command = tokenize_opts(c, grammar);
                command.iter().for_each(|t| note_mode(t, state));
                tokens.extend(command);
            }
            Piece::Text(t) => tokens.extend(tokenize_text(t, grammar, state)),
        }
    }
    tokens.dedup_by(|a, b| matches!((a, b), (Token::Space, Token::Space)));
    tokens
}

fn note_mode(token: &Token, state: &mut SayAs) {
    if let Token::Embedded(cmd) = token {
        if cmd.letter == 'Y' {
            *state = SayAs { mode: cmd.value.max(0), digits_seen: 0 };
        }
    }
}

fn tokenize_text(text: &str, grammar: &NumberGrammar, state: &mut SayAs) -> Vec<Token> {
    match state.mode {
        m if m & 0xf0 == CHARS_GROUP => spelled_words(text, (m & 0xf) as u8, true),
        KEY => key_words(text, grammar),
        m if m & 0xf0 == DIGITS => digit_words(&group_digits(text, (m & 0xf) as usize, state), grammar),
        _ => tokenize_opts(text, grammar),
    }
}

/// C skips the word rules in this mode, so letters stay with the digits beside them
/// (`a12` is one word) and only an all-digit word is a number.
fn digit_words(text: &str, grammar: &NumberGrammar) -> Vec<Token> {
    whitespace_words(text, |w| match w.chars().any(char::is_alphabetic) {
        true => vec![Token::Word(w.to_string())],
        false => tokenize_opts(w, grammar),
    })
}

/// The words of `text`, one token list each, with a space before, between and after them
/// where `text` has whitespace.
fn whitespace_words(text: &str, word: impl Fn(&str) -> Vec<Token>) -> Vec<Token> {
    let mut out = Vec::new();
    if text.starts_with(char::is_whitespace) {
        out.push(Token::Space);
    }
    for (i, w) in text.split_whitespace().enumerate() {
        if i > 0 {
            out.push(Token::Space);
        }
        out.extend(word(w));
    }
    if text.ends_with(char::is_whitespace) {
        out.push(Token::Space);
    }
    out
}

fn spelled_words(text: &str, mode: u8, pause: bool) -> Vec<Token> {
    whitespace_words(text, |w| vec![Token::Spelled { text: w.to_string(), mode, pause }])
}

/// C `TranslateWord3`: a one-character key name is spelled, a longer one is read as a word.
fn key_words(text: &str, grammar: &NumberGrammar) -> Vec<Token> {
    whitespace_words(text, |w| {
        let lower = w.to_lowercase();
        match lower.chars().count() {
            1 => vec![Token::Spelled { text: lower, mode: SINGLE_CHARS, pause: false }],
            _ => tokenize_opts(&lower, grammar),
        }
    })
}

/// C splits a digit run into words of `n` digits (`count_sayas_digits`) and ends it at
/// the first character that is not a digit.
fn group_digits(text: &str, n: usize, state: &mut SayAs) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    for c in text.chars() {
        if c.is_ascii_digit() {
            state.digits_seen += 1;
            if state.digits_seen > n {
                out.push(' ');
                state.digits_seen = 1;
            }
        } else {
            state.digits_seen = 0;
            if out.ends_with(|p: char| p.is_ascii_digit()) {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out
}

/// C `TranslateNumber` declines a number while `option_sayas == SAYAS_DIGITS1`, so its digits
/// are spoken one by one.  `mode` is the say-as mode at the start of `tokens`.
pub(crate) fn individual_digit_tokens(tokens: Vec<Token>, mode: i32) -> Vec<Token> {
    let mut mode = mode;
    tokens
        .into_iter()
        .map(|token| match token {
            Token::Embedded(cmd) if cmd.letter == 'Y' => {
                mode = cmd.value.max(0);
                Token::Embedded(cmd)
            }
            Token::Number(NumberToken::Cardinal(d) | NumberToken::Grouped(d)) if mode == DIGITS1 => Token::Digits(d),
            other => other,
        })
        .collect()
}
