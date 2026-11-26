//! Format codes as they are written in a locale.
//!
//! Number formats stored in a workbook are always in the canonical (en-US) form, which is what
//! [`format_number`](super::format::format_number) reads. The format code given to TEXT is
//! different: Excel reads it with the regional settings, so in French it is written
//! `"# ##0,00"` and `"jj/mm/aaaa"`, in German `"#.##0,00"` and `"TT.MM.JJJJ"`.
//! [`to_canonical`] rewrites such a code into the canonical form.
//!
//! References, the "Number format codes" page in each language:
//! <https://support.microsoft.com/fr-fr/office/5026bbd6-04bc-48cd-bf33-80f18b4eae68>
//! (and `de-de`, `it-it`), and LibreOffice's table of localized date codes:
//! <https://help.libreoffice.org/latest/en-US/text/shared/01/05020301.html>

use crate::locale::Locale;

/// What a language writes differently from English in a format code.
struct Keywords {
    /// The name of the General format.
    general: &'static str,
    /// Date letters: (localized letter, canonical letter), in lower case.
    letters: &'static [(char, char)],
}

fn keywords(locale_id: &str) -> Option<Keywords> {
    match locale_id.split('-').next().unwrap_or_default() {
        "fr" => Some(Keywords {
            general: "Standard",
            letters: &[('j', 'd'), ('a', 'y')],
        }),
        "de" => Some(Keywords {
            general: "Standard",
            letters: &[('t', 'd'), ('j', 'y')],
        }),
        "it" => Some(Keywords {
            general: "Generale",
            letters: &[('g', 'd'), ('a', 'y')],
        }),
        _ => None,
    }
}

enum Piece {
    /// Copied as it is: quoted text, an escaped character, `_x`, `*x`, a bracket, AM/PM, General.
    Verbatim(String),
    Char(char),
}

fn starts_with_ignore_case(chars: &[char], word: &str) -> bool {
    let mut n = 0;
    for w in word.chars() {
        match chars.get(n) {
            Some(c) if c.to_lowercase().eq(w.to_lowercase()) => n += 1,
            _ => return false,
        }
    }
    true
}

fn split_pieces(code: &str, keywords: Option<&Keywords>) -> Vec<Piece> {
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut pieces = Vec::new();
    let mut i = 0;
    while i < len {
        let c = chars[i];
        let rest = &chars[i..];
        // Length of a piece copied as it is, if one starts here.
        let verbatim = match c {
            '"' => {
                let mut j = i + 1;
                loop {
                    match chars.get(j) {
                        None => break len - i,
                        Some('"') if chars.get(j + 1) == Some(&'"') => j += 2,
                        Some('"') => break j + 1 - i,
                        Some(_) => j += 1,
                    }
                }
            }
            '\\' | '_' | '*' => 2.min(len - i),
            '[' => rest
                .iter()
                .position(|&x| x == ']')
                .map_or(len - i, |p| p + 1),
            _ if starts_with_ignore_case(rest, "AM/PM") => 5,
            _ => 0,
        };
        if verbatim > 0 {
            pieces.push(Piece::Verbatim(rest[..verbatim].iter().collect()));
            i += verbatim;
            continue;
        }
        if let Some(k) = keywords {
            if starts_with_ignore_case(rest, k.general) {
                pieces.push(Piece::Verbatim("General".to_string()));
                i += k.general.chars().count();
                continue;
            }
        }
        if starts_with_ignore_case(rest, "General") {
            pieces.push(Piece::Verbatim("General".to_string()));
            i += 7;
            continue;
        }
        let letter = keywords.and_then(|k| {
            k.letters
                .iter()
                .find(|(local, _)| c.to_ascii_lowercase() == *local)
                .map(|(_, canonical)| *canonical)
        });
        pieces.push(Piece::Char(letter.unwrap_or(c)));
        i += 1;
    }
    pieces
}

fn is_digit_placeholder(piece: Option<&Piece>) -> bool {
    matches!(piece, Some(Piece::Char('0' | '#' | '?')))
}

fn is_date_or_time(section: &[Piece]) -> bool {
    section.iter().any(|piece| match piece {
        Piece::Char(c) => matches!(c.to_ascii_lowercase(), 'd' | 'm' | 'y' | 'h' | 's'),
        Piece::Verbatim(v) => {
            v.eq_ignore_ascii_case("AM/PM")
                || (v.starts_with('[')
                    && v.len() > 2
                    && v[1..v.len() - 1]
                        .chars()
                        .all(|x| matches!(x.to_ascii_lowercase(), 'h' | 'm' | 's')))
        }
    })
}

/// Rewrites a format code written in the locale `locale_id` (the separators of `locale`, the
/// date letters and the name of General of its language) into the canonical (en-US) form.
///
/// English date letters are still understood in every locale, since none of them means
/// something else there. A period or a comma that is not one of the locale's separators is a
/// literal character. A space is the group separator only between two digit placeholders when
/// the locale groups digits with a space (French).
pub(crate) fn to_canonical(code: &str, locale_id: &str, locale: &Locale) -> String {
    let symbols = &locale.numbers.symbols;
    let decimal = symbols.decimal.chars().next().unwrap_or('.');
    let group = symbols.group.chars().next().unwrap_or(',');
    let keywords = keywords(locale_id);
    if decimal == '.' && group == ',' && keywords.is_none() {
        return code.to_string();
    }
    let group_is_space = group.is_whitespace();
    let is_group = |c: char| {
        if group_is_space {
            c.is_whitespace()
        } else {
            c == group
        }
    };

    let pieces = split_pieces(code, keywords.as_ref());
    let mut output = String::with_capacity(code.len() + 4);
    // `split` drops the section separators: one is pushed after each section.
    for section in pieces.split(|p| matches!(p, Piece::Char(';'))) {
        let date = is_date_or_time(section);
        for (i, piece) in section.iter().enumerate() {
            let c = match piece {
                Piece::Verbatim(v) => {
                    output.push_str(v);
                    continue;
                }
                Piece::Char(c) => *c,
            };
            let previous = i.checked_sub(1).and_then(|p| section.get(p));
            let next = section.get(i + 1);
            let mapped = if date {
                // Only fractions of a second have a decimal separator in a date or a time.
                let after_seconds = matches!(previous, Some(Piece::Char('s' | 'S')));
                if c == decimal && after_seconds && matches!(next, Some(Piece::Char('0'))) {
                    Some('.')
                } else {
                    None
                }
            } else if c == decimal {
                Some('.')
            } else if is_group(c)
                && (!group_is_space
                    || (is_digit_placeholder(previous) && is_digit_placeholder(next)))
            {
                Some(',')
            } else {
                None
            };
            match mapped {
                Some(m) => output.push(m),
                None if c == '.' || c == ',' => {
                    output.push('\\');
                    output.push(c);
                }
                None => output.push(c),
            }
        }
        output.push(';');
    }
    // One separator too many was pushed after the last section.
    output.pop();
    output
}
