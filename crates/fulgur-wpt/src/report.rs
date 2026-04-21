//! wptreport.json emitter. Minimal schema compatible with upstream
//! wpt.fyi submission (a subset — we omit subtests, screenshots, logs).

use crate::expectations::Expectation;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct WptReport {
    pub results: Vec<TestResult>,
    pub run_info: RunInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub test: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub subtests: Vec<serde_json::Value>,
    pub duration: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RunInfo {
    pub product: String,
    pub revision: String,
}

impl WptReport {
    pub fn new(run_info: RunInfo) -> Self {
        Self {
            results: Vec::new(),
            run_info,
        }
    }

    pub fn push(
        &mut self,
        test: impl Into<String>,
        observed: Expectation,
        message: Option<String>,
        duration: Duration,
    ) {
        let status = match observed {
            Expectation::Pass => "PASS",
            Expectation::Fail => "FAIL",
            Expectation::Skip => "SKIP",
        };
        // `Duration::as_millis` returns u128; we clamp to u64 for JSON
        // compatibility. A single reftest taking >584 million years is
        // beyond our concern.
        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        self.results.push(TestResult {
            test: test.into(),
            status,
            message,
            subtests: Vec::new(),
            duration: duration_ms,
        });
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_three_statuses_and_maps_to_strings() {
        let mut r = WptReport::new(RunInfo {
            product: "fulgur".into(),
            revision: "abc123".into(),
        });
        r.push("a.html", Expectation::Pass, None, Duration::from_millis(10));
        r.push(
            "b.html",
            Expectation::Fail,
            Some("diff 5".into()),
            Duration::from_millis(20),
        );
        r.push("c.html", Expectation::Skip, None, Duration::ZERO);
        assert_eq!(r.results.len(), 3);
        assert_eq!(r.results[0].status, "PASS");
        assert_eq!(r.results[1].status, "FAIL");
        assert_eq!(r.results[1].message.as_deref(), Some("diff 5"));
        assert_eq!(r.results[2].status, "SKIP");
    }

    #[test]
    fn skips_none_message_in_serialization() {
        let mut r = WptReport::new(RunInfo::default());
        r.push("a.html", Expectation::Pass, None, Duration::from_millis(5));
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("\"message\""), "unexpected message field: {s}");
    }

    #[test]
    fn serializes_minimal_valid_schema() {
        let mut r = WptReport::new(RunInfo {
            product: "fulgur".into(),
            revision: "abc123".into(),
        });
        r.push(
            "css/css-page/basic.html",
            Expectation::Pass,
            None,
            Duration::from_millis(15),
        );
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(v["run_info"]["product"], "fulgur");
        assert_eq!(v["run_info"]["revision"], "abc123");
        assert_eq!(v["results"][0]["test"], "css/css-page/basic.html");
        assert_eq!(v["results"][0]["status"], "PASS");
        assert_eq!(v["results"][0]["duration"], 15);
        assert!(v["results"][0]["subtests"].is_array());
    }

    #[test]
    fn write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("nested").join("wptreport.json");
        let r = WptReport::new(RunInfo::default());
        r.write(&out).unwrap();
        assert!(out.exists());
        let contents = std::fs::read_to_string(&out).unwrap();
        let v: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert!(v["results"].is_array());
    }
}
