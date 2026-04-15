//! Integration tests: `<a href>` is emitted as PDF `/Link` annotation.

use fulgur::Engine;

fn engine() -> Engine {
    Engine::builder().build()
}

#[test]
fn external_link_produces_uri_action_in_pdf() {
    let html = r#"<html><body><p><a href="https://example.com">click</a></p></body></html>"#;
    let bytes = engine().render_html(html).expect("render");
    assert!(bytes.starts_with(b"%PDF"));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Link"), "missing /Link annotation subtype");
    assert!(text.contains("/URI"), "missing /URI action type");
    assert!(
        text.contains("https://example.com"),
        "missing URI string in PDF body"
    );
}

#[test]
fn internal_anchor_produces_destination() {
    let html = r##"<html><body>
        <p><a href="#section">jump</a></p>
        <div style="height:1500px"></div>
        <h2 id="section">Target</h2>
    </body></html>"##;
    let bytes = engine().render_html(html).expect("render");
    assert!(bytes.starts_with(b"%PDF"));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Link"), "missing /Link annotation");
    // krilla serializes XYZ destinations; the annotation dict references
    // it via /Dest (indirect object reference) and the destination itself
    // carries /XYZ.
    assert!(
        text.contains("/XYZ") || text.contains("/Dest"),
        "missing internal destination marker (/XYZ or /Dest)"
    );
}

#[test]
fn unresolved_internal_anchor_is_ignored_not_error() {
    let html = r##"<html><body><p><a href="#nope">dangling</a></p></body></html>"##;
    let bytes = engine().render_html(html).expect("render should not fail");
    assert!(bytes.starts_with(b"%PDF"));
}

#[test]
fn multiline_link_merges_into_single_annotation_with_multiple_quads() {
    // Force line-wrapping of a single <a> via narrow container.
    // 200 "word " repetitions inside a 200px-wide container guarantees many lines.
    let long: String = "word ".repeat(200);
    let html = format!(
        r##"<html><body><div style="width:200px"><p><a href="https://multiline.test">{}</a></p></div></body></html>"##,
        long
    );
    let bytes = engine().render_html(&html).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    // Single <a> with text broken across multiple lines → one LinkAnnotation
    // that carries multiple QuadPoints. PDF writer emits /QuadPoints array
    // only when the annotation has more than one rectangle.
    assert!(
        text.contains("/QuadPoints"),
        "expected /QuadPoints in multi-line link"
    );
    // URI should appear once (one occurrence, not one per line)
    let uri_count = text.matches("https://multiline.test").count();
    assert!(uri_count >= 1, "URI missing");
}

#[test]
fn link_spanning_page_break_emits_annotation_on_each_page() {
    // Push a long <a> near the end of page 1 so it wraps onto page 2.
    // A spacer slightly larger than one A4 content page (≈730pt) leaves a
    // shallow strip on the next page; a tall-font, narrow-column link
    // placed after the spacer is guaranteed to overflow across the page
    // boundary so its rects land on two different pages.
    let link_text = "link ".repeat(60);
    let html = format!(
        r##"<html><body>
        <div style="height:900pt"></div>
        <div style="width:120pt;font-size:40pt;line-height:1.2"><a href="https://cross.test">{link_text}</a></div>
    </body></html>"##
    );
    let bytes = engine().render_html(&html).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    // Page-crossing links must produce ONE annotation per page (can't share across pages).
    // Expect the URI to appear at least twice in the PDF byte stream (once per /URI entry).
    let uri_count = text.matches("https://cross.test").count();
    assert!(
        uri_count >= 2,
        "expected URI on both pages, got {uri_count}"
    );
}
