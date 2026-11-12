//! A GNU `getopt_long` emulation for the command-line front end: option
//! permutation, `--` termination, unique-prefix abbreviations of long options,
//! clustered short options, and glibc's diagnostics.

/// Whether a long option takes an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HasArg {
    /// `--name` only.
    No,
    /// `--name` or `--name=value`; a following word is never consumed.
    Optional,
    /// `--name=value` or `--name value`.
    Required,
}

/// One long option; `abbreviable` is false for options that exist but must be
/// spelled in full (they never take part in prefix matching or ambiguity lists).
#[derive(Debug, Clone, Copy)]
pub struct LongOpt {
    /// Name without the leading `--`.
    pub name: &'static str,
    /// Argument rule.
    pub has_arg: HasArg,
    /// Whether the option takes part in prefix matching.
    pub abbreviable: bool,
}

/// One recognised option occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opt {
    /// The short character, or `None` for a long option.
    pub short: Option<char>,
    /// The full long name (abbreviations are expanded), empty for a short option.
    pub long: String,
    /// The attached or following argument.
    pub arg: Option<String>,
}

/// A parsed command line: the options in order, the non-option words, and the
/// first diagnostic, after which parsing stops (`getopt` returned `'?'`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Recognised options, in command-line order.
    pub opts: Vec<Opt>,
    /// Non-option words, in order (the permuted tail of `argv`).
    pub positionals: Vec<String>,
    /// The diagnostic text without the `argv[0]: ` prefix.
    pub error: Option<String>,
}

/// Parse `args` (without `argv[0]`) against a `getopt` option string such as
/// `"a:b:Dh"` and a long-option table.
pub fn parse(args: &[String], shorts: &str, longs: &[LongOpt]) -> Parsed {
    let mut out = Parsed::default();
    let mut i = 0;
    while i < args.len() && out.error.is_none() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            out.positionals.extend(args[i..].iter().cloned());
            break;
        }
        if let Some(body) = a.strip_prefix("--") {
            i += take_long(body, a, args.get(i), longs, &mut out);
        } else if a.len() > 1 && a.starts_with('-') {
            i += take_shorts(&a[1..], args.get(i), shorts, &mut out);
        } else {
            out.positionals.push(a.clone());
        }
    }
    out
}

fn arg_kind(shorts: &str, c: char) -> Option<bool> {
    let at = shorts.find(c).filter(|_| c != ':')?;
    Some(shorts[at + c.len_utf8()..].starts_with(':'))
}

/// Returns how many following words were consumed.
fn take_shorts(cluster: &str, next: Option<&String>, shorts: &str, out: &mut Parsed) -> usize {
    for (at, c) in cluster.char_indices() {
        let rest = &cluster[at + c.len_utf8()..];
        match arg_kind(shorts, c) {
            None => {
                out.error = Some(format!("invalid option -- '{c}'"));
                return 0;
            }
            Some(false) => out.opts.push(Opt { short: Some(c), long: String::new(), arg: None }),
            Some(true) if !rest.is_empty() => {
                out.opts.push(Opt { short: Some(c), long: String::new(), arg: Some(rest.to_string()) });
                return 0;
            }
            Some(true) => return short_arg_from_next(c, next, out),
        }
    }
    0
}

fn short_arg_from_next(c: char, next: Option<&String>, out: &mut Parsed) -> usize {
    match next {
        Some(n) => {
            out.opts.push(Opt { short: Some(c), long: String::new(), arg: Some(n.clone()) });
            1
        }
        None => {
            out.error = Some(format!("option requires an argument -- '{c}'"));
            0
        }
    }
}

enum Lookup<'a> {
    Found(&'a LongOpt),
    Ambiguous(Vec<&'a LongOpt>),
    Unknown,
}

fn lookup<'a>(name: &str, longs: &'a [LongOpt]) -> Lookup<'a> {
    if let Some(exact) = longs.iter().find(|l| l.name == name) {
        return Lookup::Found(exact);
    }
    let hits: Vec<&LongOpt> =
        longs.iter().filter(|l| l.abbreviable && l.name.starts_with(name)).collect();
    match hits.as_slice() {
        [] => Lookup::Unknown,
        [one] => Lookup::Found(one),
        _ => Lookup::Ambiguous(hits),
    }
}

/// Returns how many following words were consumed.
fn take_long(
    body: &str,
    whole: &str,
    next: Option<&String>,
    longs: &[LongOpt],
    out: &mut Parsed,
) -> usize {
    let (name, value) = match body.split_once('=') {
        Some((n, v)) => (n, Some(v.to_string())),
        None => (body, None),
    };
    match lookup(name, longs) {
        Lookup::Unknown => {
            out.error = Some(format!("unrecognized option '{whole}'"));
            0
        }
        Lookup::Ambiguous(hits) => {
            let list: String = hits.iter().map(|l| format!(" '--{}'", l.name)).collect();
            out.error = Some(format!("option '--{name}' is ambiguous; possibilities:{list}"));
            0
        }
        Lookup::Found(l) => take_found(l, value, next, out),
    }
}

fn take_found(l: &LongOpt, value: Option<String>, next: Option<&String>, out: &mut Parsed) -> usize {
    let push = |out: &mut Parsed, arg| {
        out.opts.push(Opt { short: None, long: l.name.to_string(), arg });
    };
    match (l.has_arg, value) {
        (HasArg::No, Some(_)) => {
            out.error = Some(format!("option '--{}' doesn't allow an argument", l.name));
            0
        }
        (HasArg::Required, None) => match next {
            Some(n) => {
                push(out, Some(n.clone()));
                1
            }
            None => {
                out.error = Some(format!("option '--{}' requires an argument", l.name));
                0
            }
        },
        (_, v) => {
            push(out, v);
            0
        }
    }
}

/// C's `atoi`: optional blanks and sign, then leading digits; anything else is 0.
pub fn atoi(s: &str) -> i32 {
    let t = s.trim_start();
    let (neg, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let n = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |acc, d| (acc * 10 + i64::from(d - b'0')).min(i64::from(i32::MAX)));
    (if neg { -n } else { n }) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONGS: &[LongOpt] = &[
        LongOpt { name: "stdin", has_arg: HasArg::No, abbreviable: true },
        LongOpt { name: "stdout", has_arg: HasArg::No, abbreviable: true },
        LongOpt { name: "path", has_arg: HasArg::Required, abbreviable: true },
        LongOpt { name: "ipa", has_arg: HasArg::Optional, abbreviable: true },
    ];

    fn words(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn atoi_reads_a_leading_integer() {
        assert_eq!([atoi("12x"), atoi(" -7"), atoi("abc"), atoi("")], [12, -7, 0, 0]);
    }

    #[test]
    fn optional_argument_never_takes_the_next_word() {
        let p = parse(&words(&["--ipa", "x"]), "", LONGS);
        assert_eq!((p.opts[0].arg.clone(), p.positionals), (None, words(&["x"])));
    }

    #[test]
    fn exact_name_beats_a_longer_prefix_match() {
        let p = parse(&words(&["--stdin"]), "", LONGS);
        assert_eq!(p.opts[0].long, "stdin");
    }
}
