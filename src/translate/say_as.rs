//! `<say-as>` as C `TranslateClause` reads it: the `Y` embedded command switches how the
//! characters after it are cut into words (`option_sayas2`), and how each word is spoken
//! (`option_sayas`).

use super::{tokenize_opts, NumberGrammar, NumberToken, Token};

const CHARS_GROUP: i32 = 0x10;
const KEY: i32 = 0x24;
const DIGITS: i32 = 0x40;
const DIGITS1: i32 = 0xc1;
const SINGLE_CHARS: u8 = 4;
/// `Token::Spelled` mode of a one-character key name that C first looks up as a word, so a
/// replacement entry (`+` is "plus") is read under its `_`-prefixed key name.
pub(crate) const KEY_CHAR: u8 = 0x24;
const MAX_BROKEN_DIGITS: usize = 32;

/// The say-as mode a clause starts in and the digit count `count_sayas_digits` carries.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SayAs {
    pub mode: i32,
    digits_seen: usize,
}

impl SayAs {
    /// `count_sayas_digits`.
    pub(crate) fn digits(&self) -> usize {
        self.digits_seen
    }
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
    let letter = rest.get(at..)?.chars().next().filter(|&l| super::is_command_letter(l))?;
    Some(at + letter.len_utf8())
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
        m if m & 0xf0 == CHARS_GROUP => spelled_words(text, (m & 0xf) as u8, true, grammar),
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

fn spelled_words(text: &str, mode: u8, pause: bool, grammar: &NumberGrammar) -> Vec<Token> {
    whitespace_words(text, |w| {
        let words = spelled_digit_groups(w, grammar).into_iter();
        let spelled = words.enumerate().map(|(ix, text)| Token::Spelled { text, mode, pause, continues: ix > 0 });
        spelled.flat_map(|t| [Token::Space, t]).skip(1).collect()
    })
}

/// C `TranslateWord3`: a one-character word of a key name is spelled; a longer one is looked
/// up, and read by the rules, under its `_`-prefixed name.
fn key_words(text: &str, grammar: &NumberGrammar) -> Vec<Token> {
    whitespace_words(text, |w| match w.chars().count() {
        1 => vec![Token::Spelled { text: w.to_lowercase(), mode: KEY_CHAR, pause: false, continues: false }],
        _ => tokenize_opts(&w.to_lowercase(), grammar).into_iter().map(key_token).collect(),
    })
}

fn key_token(token: Token) -> Token {
    let spelled = |text: String| Token::Spelled { text, mode: SINGLE_CHARS, pause: false, continues: false };
    match token {
        Token::Word(w) if w.chars().count() == 1 => spelled(w),
        Token::Word(w) => Token::Word(format!("_{w}")),
        Token::Number(NumberToken::Cardinal(d)) if d.len() == 1 => spelled(d),
        Token::Number(NumberToken::Cardinal(d)) => Token::Word(format!("_{d}")),
        Token::Punctuation(c) | Token::Operator(c) if super::is_spoken_symbol(c) => {
            Token::Spelled { text: c.to_string(), mode: KEY_CHAR, pause: false, continues: false }
        }
        other => other,
    }
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
pub(crate) fn individual_digit_tokens(tokens: Vec<Token>, mode: i32, grammar: &NumberGrammar) -> Vec<Token> {
    let mut mode = mode;
    tokens
        .into_iter()
        .flat_map(|token| match token {
            Token::Embedded(cmd) if cmd.letter == 'Y' => {
                mode = cmd.value.max(0);
                vec![Token::Embedded(cmd)]
            }
            Token::Number(NumberToken::Cardinal(d) | NumberToken::Grouped(d)) if mode == DIGITS1 => {
                broken_digit_words(&d, grammar)
            }
            other => vec![other],
        })
        .collect()
}

/// C `TranslateClause` cuts a run of 5 to 32 digits into the words `break_numbers` names
/// before any number is read, so each group is spoken digit by digit as a word of its own.
fn broken_digit_words(digits: &str, grammar: &NumberGrammar) -> Vec<Token> {
    match broken_digit_groups(digits, grammar) {
        Some(groups) => {
            let words = groups.into_iter().map(Token::Digits);
            words.flat_map(|t| [Token::Space, t]).skip(1).collect()
        }
        None => vec![Token::Digits(digits.to_string())],
    }
}

/// The words `TranslateClause` cuts `digits` into, each with its padding zeros and the separator
/// written after it; `None` when the run is too short or too long to be cut.
fn broken_digit_groups(digits: &str, grammar: &NumberGrammar) -> Option<Vec<String>> {
    if !(5..=MAX_BROKEN_DIGITS).contains(&digits.len()) {
        return None;
    }
    let groups = super::digit_groups(digits, grammar);
    let pad = !super::reads_digit_by_digit(digits, grammar);
    let last = groups.len() - 1;
    let mut start = 0;
    let words = groups.iter().enumerate().map(|(ix, group)| {
        let padding = if pad && ix > 0 { zero_padding(digits.len() - start, grammar.break_numbers) } else { "" };
        let sep = grammar.group_separator.filter(|_| ix < last).map(String::from).unwrap_or_default();
        start += group.len();
        format!("{padding}{group}{sep}")
    });
    Some(words.collect())
}

/// C cuts a word that starts with a run of digits the same way whatever the say-as mode is.
fn spelled_digit_groups(word: &str, grammar: &NumberGrammar) -> Vec<String> {
    let run = word.chars().take_while(char::is_ascii_digit).count();
    let (digits, rest) = word.split_at(run);
    match broken_digit_groups(digits, grammar) {
        Some(mut groups) => {
            if let Some(last) = groups.last_mut() {
                last.push_str(rest);
            }
            groups
        }
        None => vec![word.to_string()],
    }
}

/// The zeros C writes before a group that `remaining` digits start, when the group after it
/// would be one or two digits long.
fn zero_padding(remaining: usize, break_numbers: u32) -> &'static str {
    let breaks_at = |back: usize| remaining.checked_sub(back).is_some_and(|n| break_numbers & (1 << n) != 0);
    match (breaks_at(1), breaks_at(2)) {
        (true, true) => "000",
        (true, false) => "00",
        (false, true) => "0",
        (false, false) => "",
    }
}
