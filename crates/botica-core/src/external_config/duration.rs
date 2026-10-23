//! Time spans in the forms that package managers accept, so that `7 days`, `1 week`, `P7D`, `1w`
//! and `10080` minutes compare as the same span.

use toml::Value;

/// What a bare number counts in a key of a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Seconds,
    Minutes,
    Days,
}

const MINUTE: f64 = 60.0;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;
const WEEK: f64 = 7.0 * DAY;
// Cargo accepts months, and a month is at least 28 days.
const MONTH: f64 = 28.0 * DAY;

/// The seconds of `value`: a number, or text that is a number, counted in `unit`; an ISO 8601
/// duration (`P7D`, `PT168H`, `P1W`); or numbers each followed by a unit (`7 days`, `1 week`,
/// `1w`, `3d 12h`). `None` for anything else, such as a date or `false`.
pub fn seconds(value: &Value, unit: Option<Unit>) -> Option<f64> {
    let bare = |number: f64| {
        unit.map(|unit| {
            number
                * match unit {
                    Unit::Seconds => 1.0,
                    Unit::Minutes => MINUTE,
                    Unit::Days => DAY,
                }
        })
    };
    match value {
        Value::Integer(number) => bare(*number as f64),
        Value::Float(number) => bare(*number),
        Value::String(text) => {
            let text = text.trim();
            match text.parse::<f64>() {
                Ok(number) => bare(number),
                Err(_) => iso(text).or_else(|| spans(text)),
            }
        }
        _ => None,
    }
}

fn iso(text: &str) -> Option<f64> {
    let rest = text.strip_prefix(['P', 'p'])?;
    let (date, time) = match rest.split_once(['T', 't']) {
        Some((date, time)) => (date, Some(time)),
        None => (rest, None),
    };
    let mut total = parts(date, |letter| match letter {
        'W' => Some(WEEK),
        'D' => Some(DAY),
        _ => None,
    })?;
    if let Some(time) = time {
        total += parts(time, |letter| match letter {
            'H' => Some(HOUR),
            'M' => Some(MINUTE),
            'S' => Some(1.0),
            _ => None,
        })?;
    }
    Some(total)
}

/// The sum of `<number><letter>` pairs, each letter weighed by `weight`.
fn parts(text: &str, weight: impl Fn(char) -> Option<f64>) -> Option<f64> {
    let mut total = 0.0;
    let mut number = String::new();
    for letter in text.chars() {
        if letter.is_ascii_digit() || letter == '.' {
            number.push(letter);
        } else {
            total += number.parse::<f64>().ok()? * weight(letter.to_ascii_uppercase())?;
            number.clear();
        }
    }
    number.is_empty().then_some(total)
}

fn spans(text: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut rest = text.trim();
    let mut any = false;
    while !rest.is_empty() {
        let digits = rest
            .find(|letter: char| !(letter.is_ascii_digit() || letter == '.'))
            .unwrap_or(rest.len());
        let number: f64 = rest[..digits].parse().ok()?;
        rest = rest[digits..].trim_start();
        let letters = rest
            .find(|letter: char| !letter.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        total += number * unit_of(&rest[..letters].to_ascii_lowercase())?;
        rest = rest[letters..].trim_start_matches([' ', ',']);
        any = true;
    }
    any.then_some(total)
}

fn unit_of(word: &str) -> Option<f64> {
    Some(match word {
        "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
        "m" | "min" | "mins" | "minute" | "minutes" => MINUTE,
        "h" | "hr" | "hrs" | "hour" | "hours" => HOUR,
        "d" | "day" | "days" => DAY,
        "w" | "wk" | "wks" | "week" | "weeks" => WEEK,
        "month" | "months" => MONTH,
        _ => return None,
    })
}
