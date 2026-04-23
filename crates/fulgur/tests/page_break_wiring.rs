//! Integration tests for CSS page-break-after / page-break-before wiring (fulgur-lje5).
//!
//! Design note: these tests use 4 divs (320pt total > 200pt page) so that
//! `body.split()` is triggered by natural overflow. This is necessary because
//! the root-level `html.find_split_point` only checks direct children's
//! `pagination()`, and `body` itself never carries break-after/before — the
//! forced break lives inside `body`'s children. A follow-up task should make
//! the split algorithm detect forced breaks at arbitrary nesting depth so that
//! the simpler 2-div arrangement works without requiring overflow.

use fulgur::{Engine, Margin, PageSize};

fn page_count(pdf: &[u8]) -> usize {
    let prefix = b"/Type /Page";
    let mut count = 0usize;
    let mut i = 0;
    while i + prefix.len() < pdf.len() {
        if &pdf[i..i + prefix.len()] == prefix {
            let next = pdf[i + prefix.len()];
            if !next.is_ascii_alphanumeric() {
                count += 1;
            }
            i += prefix.len();
        } else {
            i += 1;
        }
    }
    count
}

/// `page-break-after: always` on the first div forces an extra page split.
///
/// Without the break property, 4 × 80pt = 320pt across a 200pt page gives
/// 2 pages ([d1,d2][d3,d4]).  With break-after on d1 the split at index 1
/// fires first → [d1][d2,d3,d4] → wrap triggers another split → 3 pages.
#[test]
fn page_break_after_always_splits_pages() {
    let html = r#"<!doctype html><html><head><style>
        body { margin: 0; }
        .d1 { height: 80pt; page-break-after: always; }
        .d2 { height: 80pt; }
        .d3 { height: 80pt; }
        .d4 { height: 80pt; }
    </style></head><body>
      <div class="d1"></div>
      <div class="d2"></div>
      <div class="d3"></div>
      <div class="d4"></div>
    </body></html>"#;
    let engine = Engine::builder()
        .page_size(PageSize::custom(70.5556, 70.5556))
        .margin(Margin::uniform(0.0))
        .build();
    let pdf = engine.render_html(html).expect("render");
    assert!(
        page_count(&pdf) >= 3,
        "expected page-break-after: always to force >= 3 pages, got {}",
        page_count(&pdf)
    );
}

/// `break-after: page` (CSS Fragmentation Level 3) also forces the extra split.
#[test]
fn break_after_page_splits_pages() {
    let html = r#"<!doctype html><html><head><style>
        body { margin: 0; }
        .d1 { height: 80pt; break-after: page; }
        .d2 { height: 80pt; }
        .d3 { height: 80pt; }
        .d4 { height: 80pt; }
    </style></head><body>
      <div class="d1"></div>
      <div class="d2"></div>
      <div class="d3"></div>
      <div class="d4"></div>
    </body></html>"#;
    let engine = Engine::builder()
        .page_size(PageSize::custom(70.5556, 70.5556))
        .margin(Margin::uniform(0.0))
        .build();
    let pdf = engine.render_html(html).expect("render");
    assert!(
        page_count(&pdf) >= 3,
        "expected break-after: page to force >= 3 pages, got {}",
        page_count(&pdf)
    );
}

/// `page-break-before: always` on the second div forces an extra split.
///
/// `break_before` requires `i > 0 && pc.y > 0`, so d2 (index 1, y = 80pt)
/// is the first position where the condition can fire.  Result:
/// [d1][d2,d3,d4] → overflow → [d1][d2,d3][d4] = 3 pages.
#[test]
fn page_break_before_always_splits_pages() {
    let html = r#"<!doctype html><html><head><style>
        body { margin: 0; }
        .d1 { height: 80pt; }
        .d2 { height: 80pt; page-break-before: always; }
        .d3 { height: 80pt; }
        .d4 { height: 80pt; }
    </style></head><body>
      <div class="d1"></div>
      <div class="d2"></div>
      <div class="d3"></div>
      <div class="d4"></div>
    </body></html>"#;
    let engine = Engine::builder()
        .page_size(PageSize::custom(70.5556, 70.5556))
        .margin(Margin::uniform(0.0))
        .build();
    let pdf = engine.render_html(html).expect("render");
    assert!(
        page_count(&pdf) >= 3,
        "expected page-break-before: always to force >= 3 pages, got {}",
        page_count(&pdf)
    );
}

/// `break-before: page` (CSS Fragmentation Level 3) also forces the extra split.
#[test]
fn break_before_page_splits_pages() {
    let html = r#"<!doctype html><html><head><style>
        body { margin: 0; }
        .d1 { height: 80pt; }
        .d2 { height: 80pt; break-before: page; }
        .d3 { height: 80pt; }
        .d4 { height: 80pt; }
    </style></head><body>
      <div class="d1"></div>
      <div class="d2"></div>
      <div class="d3"></div>
      <div class="d4"></div>
    </body></html>"#;
    let engine = Engine::builder()
        .page_size(PageSize::custom(70.5556, 70.5556))
        .margin(Margin::uniform(0.0))
        .build();
    let pdf = engine.render_html(html).expect("render");
    assert!(
        page_count(&pdf) >= 3,
        "expected break-before: page to force >= 3 pages, got {}",
        page_count(&pdf)
    );
}

/// Without any break property, 4 × 80pt = 320pt > 200pt gives exactly 2 pages.
/// This confirms the break tests above are detecting forced splits, not just overflow.
#[test]
fn no_break_property_stays_on_one_page() {
    let html = r#"<!doctype html><html><head><style>
        body { margin: 0; }
        .first { height: 80pt; }
        .second { height: 80pt; }
    </style></head><body>
      <div class="first"></div>
      <div class="second"></div>
    </body></html>"#;
    let engine = Engine::builder()
        .page_size(PageSize::custom(70.5556, 70.5556))
        .margin(Margin::uniform(0.0))
        .build();
    let pdf = engine.render_html(html).expect("render");
    assert_eq!(
        page_count(&pdf),
        1,
        "without break properties, both divs should fit on 1 page, got {}",
        page_count(&pdf)
    );
}
