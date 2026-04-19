mod support;
use support::content_stream::count_ops;

use fulgur::config::PageSize;
use fulgur::engine::Engine;
use std::path::PathBuf;

fn render_example(name: &str) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join(name);
    let html = std::fs::read_to_string(root.join("index.html")).unwrap();

    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .base_path(root)
        .build();

    engine
        .render_html(&html)
        .expect("render_html should succeed")
}

#[test]
fn table_header_uses_rect_for_uniform_borders() {
    let pdf = render_example("table-header");
    let Some(counts) = count_ops(&pdf) else {
        eprintln!("qpdf not installed — skipping");
        return;
    };

    // After rect-borders: uniform cell borders should be single `re`
    // strokes, not 4 `m+l` per cell. Threshold is generous; tighter
    // locking comes in Task 7.
    assert!(
        counts.re > 50,
        "expected re > 50 (rect-stroked cell borders), got re={} m={} l={}",
        counts.re,
        counts.m,
        counts.l
    );
    assert!(
        counts.m < 400,
        "expected m < 400 (line strokes collapsed into rects), got m={} (re={})",
        counts.m,
        counts.re
    );
}
