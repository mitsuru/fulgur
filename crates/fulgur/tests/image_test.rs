use fulgur::asset::AssetBundle;
use fulgur::config::{Margin, PageSize};
use fulgur::engine::Engine;

// Minimal 1x1 white PNG (69 bytes)
const MINIMAL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
    0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // 8-bit RGB
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
    0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, // compressed data
    0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, //
    0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn test_img_renders_to_pdf() {
    let mut assets = AssetBundle::new();
    assets.add_image("logo.png", MINIMAL_PNG.to_vec());

    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .assets(assets)
        .build();

    let html = r#"<html><body><img src="logo.png" style="width:100px;height:100px"></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 200);
}

#[test]
fn test_img_with_dot_slash_prefix() {
    let mut assets = AssetBundle::new();
    assets.add_image("logo.png", MINIMAL_PNG.to_vec());

    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .assets(assets)
        .build();

    let html = r#"<html><body><img src="./logo.png" style="width:50px;height:50px"></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn test_img_missing_image_no_error() {
    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .build();

    let html =
        r#"<html><body><img src="missing.png" style="width:50px;height:50px"></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn test_img_no_assets_no_error() {
    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .build();

    let html = r#"<html><body><img src="logo.png" style="width:50px;height:50px"></body></html>"#;
    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}
