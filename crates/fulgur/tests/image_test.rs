use fulgur::asset::AssetBundle;
use fulgur::config::{Margin, PageSize};
use fulgur::engine::Engine;

// Minimal 1x1 red PNG (69 bytes) with valid CRCs
const MINIMAL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn build_engine_with_image(name: &str) -> Engine {
    let mut assets = AssetBundle::new();
    assets.add_image(name, MINIMAL_PNG.to_vec());
    Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .assets(assets)
        .build()
}

fn build_engine_no_assets() -> Engine {
    Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .build()
}

#[test]
fn test_img_renders_to_pdf() {
    let engine = build_engine_with_image("logo.png");
    let html = r#"<html><body><div><img src="logo.png" style="display:block;width:100px;height:100px"></div></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));

    // Verify image data is embedded: PDF with image should be larger than without
    let pdf_no_img = build_engine_no_assets().render_html(html).unwrap();
    assert!(
        pdf.len() > pdf_no_img.len(),
        "PDF with image ({} bytes) should be larger than without ({} bytes)",
        pdf.len(),
        pdf_no_img.len()
    );
}

#[test]
fn test_img_with_dot_slash_prefix() {
    let engine = build_engine_with_image("logo.png");
    let html = r#"<html><body><div><img src="./logo.png" style="display:block;width:50px;height:50px"></div></body></html>"#;
    let pdf = engine.render_html(html).unwrap();

    // Verify ./logo.png resolves to same image as logo.png
    let pdf_no_img = build_engine_no_assets().render_html(html).unwrap();
    assert!(
        pdf.len() > pdf_no_img.len(),
        "PDF with ./logo.png ({} bytes) should be larger than without ({} bytes)",
        pdf.len(),
        pdf_no_img.len()
    );
}

#[test]
fn test_img_missing_image_no_error() {
    // AssetBundle exists but image not registered — tests get_image returning None
    let mut assets = AssetBundle::new();
    assets.add_image("other.png", MINIMAL_PNG.to_vec());
    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .assets(assets)
        .build();

    let html =
        r#"<html><body><img src="missing.png" style="width:50px;height:50px"></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn test_img_no_assets_no_error() {
    let engine = build_engine_no_assets();
    let html = r#"<html><body><img src="logo.png" style="width:50px;height:50px"></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}
