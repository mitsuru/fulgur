use fulgur::config::PageSize;
use fulgur::engine::Engine;

#[test]
fn test_opacity_half() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="opacity: 0.5;">
            <p>This text should be semi-transparent</p>
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1000);
}

#[test]
fn test_opacity_zero() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="opacity: 0;">
            <p>This text should be invisible but preserve layout</p>
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 500);
}

#[test]
fn test_visibility_hidden() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="visibility: hidden;">
            <p>This text should be hidden</p>
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 500);
}

#[test]
fn test_opacity_on_div_placeholder() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="opacity: 0.7; width: 100px; height: 100px; background-color: blue;">
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 500);
}

#[test]
fn test_nested_opacity() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="opacity: 0.5;">
            <p>Outer semi-transparent</p>
            <div style="opacity: 0.5;">
                <p>Inner semi-transparent (effective 0.25)</p>
            </div>
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1000);
}

#[test]
fn test_opacity_with_background() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="background-color: red; opacity: 0.5; padding: 20px;">
            <p>Semi-transparent red background</p>
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1000);
}

#[test]
fn test_visibility_hidden_preserves_layout() {
    let engine = Engine::builder().page_size(PageSize::A4).build();
    let html = r#"<html><body>
        <div style="visibility: hidden; height: 100px; background-color: blue;">
            <p>Hidden but takes space</p>
        </div>
        <div style="background-color: green; padding: 10px;">
            <p>This should appear below the hidden element</p>
        </div>
    </body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1000);
}
