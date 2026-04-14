use fulgur::asset::AssetBundle;
use fulgur::config::PageSize;
use fulgur::engine::Engine;

#[test]
fn woff2_font_renders_to_pdf() {
    let mut assets = AssetBundle::new();
    assets
        .add_font_file("tests/fixtures/fonts/NotoSans-Regular.woff2")
        .expect("WOFF2 load must succeed");
    assets.add_css("body { font-family: 'Noto Sans', sans-serif; }");

    let engine = Engine::builder()
        .page_size(PageSize::A4)
        .assets(assets)
        .build();

    let html = r#"<html><body>
        <h1>WOFF2 Font Test</h1>
        <p>Hello — WOFF2 fixture rendering via AssetBundle.</p>
    </body></html>"#;

    let pdf = engine.render_html(html).expect("PDF render must succeed");
    assert!(
        pdf.starts_with(b"%PDF"),
        "output must start with %PDF magic"
    );
    assert!(
        pdf.len() > 1000,
        "PDF should be non-trivial, got {} bytes",
        pdf.len()
    );
}
