/// Expands numeric literals into the English words used to measure a line.
pub(super) fn expand_numbers(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            let char = text[i..].chars().next().expect("valid character boundary");
            out.push(char);
            i += char.len_utf8();
            continue;
        }

        let start = i;

        // Read the initial digits.
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }

        // Preserve decimal literals unchanged rather than reading each side
        // of the decimal point as a separate number.
        if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
            i += 1;

            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }

            let literal = &text[start..i];
            out.push_str(literal);
            continue;
        }

        // Read comma-separated groups, such as 10,000 or 1,000,000.
        while i < bytes.len() && bytes[i] == b',' {
            let group_start = i + 1;
            let mut group_end = group_start;

            while group_end < bytes.len() && bytes[group_end].is_ascii_digit() {
                group_end += 1;
            }

            let group_len = group_end - group_start;

            if group_len != 3 {
                break;
            }

            i = group_end;
        }

        // Read a time such as 9:30.
        if i + 1 < bytes.len() && bytes[i] == b':' && bytes[i + 1].is_ascii_digit() {
            i += 1;

            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        let number_end = i;

        // Read ordinal suffixes such as st, nd, rd, and th.
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }

        let literal = &text[start..number_end];
        let suffix = &text[number_end..i];
        let replacement = expand_literal(literal, suffix);

        match replacement {
            Some(words) => {
                if !out.is_empty() && !out.chars().last().is_some_and(char::is_whitespace) {
                    out.push(' ');
                }
                out.push_str(&words);
                if i < bytes.len() {
                    let next = text[i..].chars().next().expect("valid character boundary");
                    if next.is_alphanumeric() {
                        out.push(' ');
                    }
                }
            }
            None => out.push_str(&text[start..i]),
        }
    }

    out
}

fn expand_literal(literal: &str, suffix: &str) -> Option<String> {
    if let Some((hour, minute)) = literal.split_once(':') {
        if minute.len() != 2 || hour.is_empty() || hour.len() > 2 {
            return None;
        }

        let hour: u64 = hour.parse().ok()?;
        let minute: u64 = minute.parse().ok()?;
        if hour > 23 || minute > 59 {
            return None;
        }

        let mut words = cardinal(hour)?;
        words.push_str(&minute_words(minute)?);
        return Some(words);
    }

    if !suffix.is_empty() {
        let value: u64 = literal.replace(',', "").parse().ok()?;
        if !matches!(suffix, "st" | "nd" | "rd" | "th") {
            return None;
        }
        return ordinal(value);
    }

    let digits = literal.replace(',', "");
    let value: u64 = digits.parse().ok()?;

    // Years are conventionally read in pairs, e.g. 1999 -> "nineteen
    // ninety-nine", rather than "one thousand nine hundred ninety-nine".
    if digits.len() == 4 && (1000..=2099).contains(&value) {
        return year(value);
    }

    cardinal(value)
}

fn cardinal(n: u64) -> Option<String> {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];

    match n {
        0..=19 => Some(ONES[n as usize].to_string()),
        20..=99 => {
            let tens = TENS[(n / 10) as usize];
            let ones = n % 10;
            Some(if ones == 0 {
                tens.to_string()
            } else {
                format!("{tens}-{}", ONES[ones as usize])
            })
        }
        100..=999 => {
            let rest = n % 100;
            Some(if rest == 0 {
                format!("{} hundred", ONES[(n / 100) as usize])
            } else {
                format!("{} hundred {}", ONES[(n / 100) as usize], cardinal(rest)?)
            })
        }
        1_000..=999_999 => {
            let thousands = n / 1_000;
            let rest = n % 1_000;
            Some(if rest == 0 {
                format!("{} thousand", cardinal(thousands)?)
            } else {
                format!("{} thousand {}", cardinal(thousands)?, cardinal(rest)?)
            })
        }
        _ => None,
    }
}

fn year(n: u64) -> Option<String> {
    if (1000..=1999).contains(&n) {
        let first = cardinal(n / 100)?;
        let rest = n % 100;

        return Some(match rest {
            0 => format!("{first} hundred"),
            1..=9 => format!("{first} oh {}", cardinal(rest)?),
            _ => format!("{first} {}", cardinal(rest)?),
        });
    }

    if (2000..=2009).contains(&n) {
        let rest = n % 2000;
        return Some(if rest == 0 {
            "two thousand".to_string()
        } else {
            format!("two thousand {}", cardinal(rest)?)
        });
    }

    let rest = n % 100;
    Some(if rest == 0 {
        format!("{} hundred", cardinal(n / 100)?)
    } else {
        format!("{} {}", cardinal(n / 100)?, cardinal(rest)?)
    })
}

fn minute_words(minute: u64) -> Option<String> {
    if minute < 10 {
        Some(format!(" oh {}", cardinal(minute)?))
    } else {
        Some(format!(" {}", cardinal(minute)?))
    }
}

fn ordinal(n: u64) -> Option<String> {
    let words = cardinal(n)?;
    let (head, last) = words
        .rsplit_once([' ', '-'])
        .map_or(("", words.as_str()), |(h, l)| (h, l));

    let last_ordinal = match last {
        "one" => "first",
        "two" => "second",
        "three" => "third",
        "five" => "fifth",
        "eight" => "eighth",
        "nine" => "ninth",
        "twelve" => "twelfth",
        "twenty" => "twentieth",
        "thirty" => "thirtieth",
        "forty" => "fortieth",
        "fifty" => "fiftieth",
        "sixty" => "sixtieth",
        "seventy" => "seventieth",
        "eighty" => "eightieth",
        "ninety" => "ninetieth",
        "zero" => return None,
        other if other.ends_with("teen") => return Some(format!("{head} {other}th").trim().to_string()),
        other if other.ends_with("ty") => {
            return Some(
                format!("{head} {}tieth", other.trim_end_matches("ty"))
                    .trim()
                    .to_string(),
            );
        }
        other => return Some(format!("{head} {other}th").trim().to_string()),
    };

    Some(if head.is_empty() {
        last_ordinal.to_string()
    } else {
        format!("{head}-{last_ordinal}")
    })
}

#[cfg(test)]
mod tests {
    use super::expand_numbers;

    #[test]
    fn expands_grouped_numbers_years_ordinals_and_times() {
        assert_eq!(expand_numbers("I paid 10,000 dollars."), "I paid ten thousand dollars.");
        assert_eq!(
            expand_numbers("I was born in 1999."),
            "I was born in nineteen ninety-nine."
        );
        assert_eq!(expand_numbers("She finished 1st."), "She finished first.");
        assert_eq!(expand_numbers("Meet me at 9:30."), "Meet me at nine thirty.");
    }

    #[test]
    fn leaves_unsupported_numeric_literals_unchanged() {
        assert_eq!(expand_numbers("The value is 1,000,000."), "The value is 1,000,000.");
        assert_eq!(expand_numbers("The ratio is 1.5."), "The ratio is 1.5.");
    }
}
