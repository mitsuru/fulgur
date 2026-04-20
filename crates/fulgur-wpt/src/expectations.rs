//! Expectations file parser and PASS/FAIL/SKIP judgement.
//!
//! An expectation file is a plain-text list, one entry per line:
//!
//! ```text
//! # comments start with '#'
//! PASS css/css-page/a.html
//! FAIL css/css-page/b.html  # regression since 2026-04
//! SKIP css/css-page/c.html  # manual-only test
//! ```
//!
//! Blank lines and lines starting with `#` are ignored. An inline `#`
//! starts a trailing comment that is captured but does not affect
//! parsing. Duplicate entries for the same path are rejected.

use anyhow::{Result, bail};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Default)]
pub struct ExpectationFile {
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    expectation: Expectation,
    #[allow(dead_code)] // comment is informational; kept for future reporting
    comment: Option<String>,
}

impl ExpectationFile {
    pub fn parse(src: &str) -> Result<Self> {
        let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
        for (lineno, raw) in src.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (body, comment) = match line.find('#') {
                Some(idx) => (line[..idx].trim(), Some(line[idx + 1..].trim().to_string())),
                None => (line, None),
            };
            let mut parts = body.splitn(2, char::is_whitespace);
            let status = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("").trim();
            if path.is_empty() {
                bail!("line {}: missing path", lineno + 1);
            }
            let expectation = match status {
                "PASS" => Expectation::Pass,
                "FAIL" => Expectation::Fail,
                "SKIP" => Expectation::Skip,
                other => bail!("line {}: unknown status {other}", lineno + 1),
            };
            if entries.contains_key(path) {
                bail!("line {}: duplicate entry for {path}", lineno + 1);
            }
            entries.insert(
                path.to_string(),
                Entry {
                    expectation,
                    comment,
                },
            );
        }
        Ok(Self { entries })
    }

    pub fn load(path: &Path) -> Result<Self> {
        let s = std::fs::read_to_string(path)?;
        Self::parse(&s)
    }

    pub fn get(&self, test_path: &str) -> Option<Expectation> {
        self.entries.get(test_path).map(|e| e.expectation)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    Regression,
    Promotion,
    Skipped,
    UnknownTest,
}

/// Compare the declared expectation with the observed result.
pub fn judge(declared: Option<Expectation>, observed: Option<Expectation>) -> Verdict {
    match (declared, observed) {
        (Some(Expectation::Skip), _) => Verdict::Skipped,
        (Some(d), Some(o)) if d == o => Verdict::Ok,
        (Some(Expectation::Pass), Some(Expectation::Fail)) => Verdict::Regression,
        (Some(Expectation::Fail), Some(Expectation::Pass)) => Verdict::Promotion,
        (None, _) => Verdict::UnknownTest,
        _ => Verdict::Ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_three_statuses() {
        let src = "\
# comment line
PASS css/css-page/a.html
FAIL css/css-page/b.html  # reason
SKIP css/css-page/c.html  # manual

";
        let f = ExpectationFile::parse(src).unwrap();
        assert_eq!(f.get("css/css-page/a.html"), Some(Expectation::Pass));
        assert_eq!(f.get("css/css-page/b.html"), Some(Expectation::Fail));
        assert_eq!(f.get("css/css-page/c.html"), Some(Expectation::Skip));
        assert_eq!(f.len(), 3);
    }

    #[test]
    fn ignores_blank_and_comment_lines() {
        let src = "\n  \n# foo\nPASS x\n";
        let f = ExpectationFile::parse(src).unwrap();
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn rejects_unknown_status() {
        assert!(ExpectationFile::parse("XYZZY a.html\n").is_err());
    }

    #[test]
    fn rejects_duplicate_entry() {
        let src = "PASS a.html\nFAIL a.html\n";
        assert!(ExpectationFile::parse(src).is_err());
    }

    #[test]
    fn judge_pass_pass_is_ok() {
        assert_eq!(
            judge(Some(Expectation::Pass), Some(Expectation::Pass)),
            Verdict::Ok
        );
    }

    #[test]
    fn judge_pass_fail_is_regression() {
        assert_eq!(
            judge(Some(Expectation::Pass), Some(Expectation::Fail)),
            Verdict::Regression
        );
    }

    #[test]
    fn judge_fail_pass_is_promotion() {
        assert_eq!(
            judge(Some(Expectation::Fail), Some(Expectation::Pass)),
            Verdict::Promotion
        );
    }

    #[test]
    fn judge_skip_wins() {
        assert_eq!(
            judge(Some(Expectation::Skip), Some(Expectation::Pass)),
            Verdict::Skipped
        );
    }

    #[test]
    fn judge_unknown_declared() {
        assert_eq!(judge(None, Some(Expectation::Pass)), Verdict::UnknownTest);
    }

    // ---- Extra coverage ----------------------------------------------------

    #[test]
    fn parses_tab_separated_fields() {
        let src = "PASS\tcss/css-page/tabbed.html\n";
        let f = ExpectationFile::parse(src).unwrap();
        assert_eq!(f.get("css/css-page/tabbed.html"), Some(Expectation::Pass));
    }

    #[test]
    fn parses_crlf_line_endings() {
        let src = "PASS a.html\r\nFAIL b.html  # reason\r\n";
        let f = ExpectationFile::parse(src).unwrap();
        assert_eq!(f.get("a.html"), Some(Expectation::Pass));
        assert_eq!(f.get("b.html"), Some(Expectation::Fail));
    }

    #[test]
    fn empty_file_yields_empty() {
        let f = ExpectationFile::parse("").unwrap();
        assert!(f.is_empty());
        assert_eq!(f.len(), 0);
    }

    #[test]
    fn rejects_missing_path() {
        // Bare status with no path.
        assert!(ExpectationFile::parse("PASS\n").is_err());
    }

    // Current contract: "declared PASS, observed SKIP" is not a regression.
    // It falls into the catch-all OK arm. Pinning the behaviour so any
    // future tightening of the rule is deliberate rather than accidental.
    #[test]
    fn judge_pass_declared_skip_observed_is_ok() {
        assert_eq!(
            judge(Some(Expectation::Pass), Some(Expectation::Skip)),
            Verdict::Ok
        );
    }
}
