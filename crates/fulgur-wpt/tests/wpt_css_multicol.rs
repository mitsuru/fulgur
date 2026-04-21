//! Phase 3 entry: run all css-multicol reftests via the shared runner.
//! Never panics on regressions — emits artifacts under target/wpt-report/css-multicol/.

use fulgur_wpt::runner::run_phase;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn wpt_css_multicol() {
    let outcome =
        run_phase(&workspace_root(), "css-multicol", 96).expect("runner should not error");
    if let Some(o) = outcome {
        eprintln!("css-multicol report at {}", o.report_dir.display());
    }
}
