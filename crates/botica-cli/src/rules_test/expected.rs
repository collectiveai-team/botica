//! The `expected.json` of a judgment rule: the band of abide that each fixture must get, and the
//! range of its probability.

use serde::Deserialize;

/// How abide reads the probability that a rule is broken: at or over its `act` threshold, at or
/// over its `flag` threshold, or under both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Act,
    Flag,
    Clear,
}

impl Band {
    pub fn word(self) -> &'static str {
        match self {
            Self::Act => "act",
            Self::Flag => "flag",
            Self::Clear => "clear",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub verdicts: Vec<ExpectedVerdict>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedVerdict {
    /// The fixture, from `fixtures/`: `valid/<file>` or `invalid/<file>`.
    pub file: String,
    pub band: Band,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// What abide said of a fixture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Judged {
    pub band: Band,
    pub probability: f64,
}

pub fn parse(text: &str) -> Result<Expected, String> {
    serde_json::from_str(text).map_err(|error| format!("expected.json: {error}"))
}

/// What is wrong with `expected` for the fixtures `files` (each `valid/<file>` or
/// `invalid/<file>`): each fixture has one verdict, a valid one expects `clear` and an invalid one
/// `act` or `flag`, and a range is within 0 and 1.
pub fn problems(expected: &Expected, files: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for (at, verdict) in expected.verdicts.iter().enumerate() {
        let file = &verdict.file;
        if expected.verdicts[..at]
            .iter()
            .any(|other| other.file == *file)
        {
            found.push(format!("`{file}` has more than one verdict"));
        }
        if !files.contains(file) {
            found.push(format!("`{file}` is not a fixture of the rule"));
        }
        let valid = file.starts_with("valid/");
        match (valid, verdict.band) {
            (true, Band::Clear) | (false, Band::Act | Band::Flag) => {}
            (true, band) => found.push(format!(
                "`{file}` is valid and expects `{}`: a valid fixture expects `clear`",
                band.word()
            )),
            (false, band) => found.push(format!(
                "`{file}` is invalid and expects `{}`: an invalid fixture expects `act` or `flag`",
                band.word()
            )),
        }
        let (min, max) = (verdict.min.unwrap_or(0.0), verdict.max.unwrap_or(1.0));
        if !(0.0..=1.0).contains(&min) || !(0.0..=1.0).contains(&max) || min > max {
            found.push(format!(
                "`{file}`: `min` and `max` are probabilities, from 0 to 1, and `min` is not over `max`"
            ));
        }
    }
    for file in files {
        if !expected
            .verdicts
            .iter()
            .any(|verdict| verdict.file == *file)
        {
            found.push(format!("`{file}` has no verdict in expected.json"));
        }
    }
    found
}

/// Why what abide said of a fixture is not what `expected` asks, or `None` when it is.
pub fn mismatch(expected: &ExpectedVerdict, judged: Option<Judged>) -> Option<String> {
    let Some(judged) = judged else {
        return Some("abide gave no verdict".to_owned());
    };
    let mut wanted = format!("expected `{}`", expected.band.word());
    if let Some(min) = expected.min {
        wanted.push_str(&format!(", at least {min}"));
    }
    if let Some(max) = expected.max {
        wanted.push_str(&format!(", at most {max}"));
    }
    let in_range = expected.min.is_none_or(|min| judged.probability >= min)
        && expected.max.is_none_or(|max| judged.probability <= max);
    (judged.band != expected.band || !in_range).then_some(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        vec!["valid/seam.py".to_owned(), "invalid/private.py".to_owned()]
    }

    fn expected(verdicts: &str) -> Expected {
        parse(&format!("{{\"verdicts\": [{verdicts}]}}")).expect("expected.json")
    }

    const GOOD: &str = r#"{"file": "valid/seam.py", "band": "clear", "max": 0.2},
        {"file": "invalid/private.py", "band": "act", "min": 0.9}"#;

    #[test]
    fn an_expected_json_with_a_verdict_for_each_fixture_has_no_problem() {
        assert!(problems(&expected(GOOD), &files()).is_empty());
        let flag = GOOD.replace("\"act\"", "\"flag\"");
        assert!(problems(&expected(&flag), &files()).is_empty());
    }

    #[test]
    fn each_fixture_has_one_verdict_and_each_verdict_names_a_fixture() {
        let found = problems(
            &expected(&format!(
                r#"{GOOD}, {{"file": "valid/seam.py", "band": "clear"}},
                {{"file": "valid/other.py", "band": "clear"}}"#
            )),
            &files(),
        );
        assert_eq!(
            found,
            [
                "`valid/seam.py` has more than one verdict",
                "`valid/other.py` is not a fixture of the rule",
            ]
        );

        let found = problems(
            &expected(r#"{"file": "valid/seam.py", "band": "clear"}"#),
            &files(),
        );
        assert_eq!(
            found,
            ["`invalid/private.py` has no verdict in expected.json"]
        );
    }

    #[test]
    fn a_valid_fixture_expects_clear_and_an_invalid_one_act_or_flag() {
        let found = problems(
            &expected(
                r#"{"file": "valid/seam.py", "band": "flag"},
                {"file": "invalid/private.py", "band": "clear"}"#,
            ),
            &files(),
        );

        assert_eq!(
            found,
            [
                "`valid/seam.py` is valid and expects `flag`: a valid fixture expects `clear`",
                "`invalid/private.py` is invalid and expects `clear`: an invalid fixture expects \
                 `act` or `flag`",
            ]
        );
    }

    #[test]
    fn a_range_is_of_probabilities_and_in_order() {
        for range in [
            r#""min": -0.1"#,
            r#""max": 1.5"#,
            r#""min": 0.8, "max": 0.7"#,
        ] {
            let found = problems(
                &expected(&format!(
                    r#"{{"file": "valid/seam.py", "band": "clear"}},
                    {{"file": "invalid/private.py", "band": "act", {range}}}"#
                )),
                &files(),
            );

            assert_eq!(
                found,
                ["`invalid/private.py`: `min` and `max` are probabilities, from 0 to 1, and `min` \
                  is not over `max`"],
                "{range}"
            );
        }
    }

    #[test]
    fn an_unknown_field_or_band_is_an_error_of_the_file() {
        for text in [
            r#"{"verdicts": [{"file": "valid/seam.py", "band": "clear", "maximum": 1}]}"#,
            r#"{"verdicts": [{"file": "valid/seam.py", "band": "maybe"}]}"#,
            r#"{"diagnostics": []}"#,
        ] {
            let error = parse(text).expect_err("not an expected.json of a judgment rule");

            assert!(error.starts_with("expected.json: "), "{error}");
        }
    }

    fn verdict(band: Band, min: Option<f64>, max: Option<f64>) -> ExpectedVerdict {
        ExpectedVerdict {
            file: "invalid/private.py".to_owned(),
            band,
            min,
            max,
        }
    }

    fn judged(band: Band, probability: f64) -> Option<Judged> {
        Some(Judged { band, probability })
    }

    #[test]
    fn a_verdict_matches_with_its_band_and_a_probability_within_the_range() {
        let expected = verdict(Band::Act, Some(0.9), Some(0.95));

        assert_eq!(mismatch(&expected, judged(Band::Act, 0.9)), None);
        assert_eq!(mismatch(&expected, judged(Band::Act, 0.95)), None);
        for other in [
            judged(Band::Flag, 0.92),
            judged(Band::Act, 0.89),
            judged(Band::Act, 0.96),
        ] {
            assert_eq!(
                mismatch(&expected, other),
                Some("expected `act`, at least 0.9, at most 0.95".to_owned()),
                "{other:?}"
            );
        }
        assert_eq!(
            mismatch(&verdict(Band::Clear, None, None), judged(Band::Clear, 0.4)),
            None
        );
        assert_eq!(
            mismatch(&verdict(Band::Clear, None, None), judged(Band::Flag, 0.6)),
            Some("expected `clear`".to_owned())
        );
    }

    #[test]
    fn a_fixture_without_a_verdict_of_abide_does_not_match() {
        assert_eq!(
            mismatch(&verdict(Band::Clear, None, None), None),
            Some("abide gave no verdict".to_owned())
        );
    }
}
