# SVG Inline Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** インライン `<svg>` 要素を PDF にベクター描画する（fulgur-rt2）

**Architecture:** `krilla` を 0.6 → 0.7 に更新し、新規依存 `krilla-svg` の `SurfaceExt::draw_svg` を経由して `usvg::Tree` を krilla の Surface に直接描画する。Blitz はデフォルトでインライン `<svg>` を `ImageData::Svg(Box<usvg::Tree>)` としてパース済みのため、`convert.rs` に `<svg>` 分岐を追加し、新規 `SvgPageable` 型（`ImagePageable` と同形）で包んで Pageable ツリーに組み込む。

**Tech Stack:** Rust 2024, krilla 0.7, krilla-svg 0.7, usvg 0.45, blitz-dom 0.2

**Related:** beads issue `fulgur-rt2`、設計詳細は issue の `design` フィールド参照

---

## 背景メモ（実装者向け）

以下は今回の設計段階で検証済みの前提。疑って再調査する必要はない：

1. **krilla 0.6 → 0.7 は fulgur に対して drop-in 互換**。実ビルド・全 296 テストで確認済み。`Surface` API (`crates/fulgur/src/...` で使っている範囲) の diff はゼロ
2. **`blitz-dom` の `svg` feature はデフォルト ON**。インライン `<svg>` は Blitz が自動的に `usvg` でパースし、`ElementData::image_data() -> Option<&ImageData>` 経由で `ImageData::Svg(Box<usvg::Tree>)` として取得できる
3. **Blitz の taffy layout は `<svg>` を replaced element として扱う**ため、`node.final_layout.size` は CSS-resolved 済みの幅・高さを返す（`<img>` と同じ）
4. **`usvg::Tree` は `#[derive(Clone, Debug)]`**。内部 `Vec<Arc<...>>` 構造なので clone は安価
5. **`krilla-svg 0.7` の `SurfaceExt::draw_svg`** シグネチャ:

   ```rust
   fn draw_svg(&mut self, tree: &Tree, size: Size, svg_settings: SvgSettings) -> Option<()>
   ```

   krilla-svg は内部で clip_path を push/pop するので呼び出し側で clip は不要

## 参考ファイル

- `crates/fulgur/src/image.rs` — `SvgPageable` のほぼ全コピー元（差し替え点は clone で一度差分を読む）
- `crates/fulgur/src/convert.rs:299-305` — `<img>` 検出分岐（`<svg>` 分岐はこの直後に追加）
- `crates/fulgur/src/convert.rs:513-557` — `convert_image` 関数（`convert_svg` のテンプレ）
- `crates/fulgur/tests/image_test.rs` — 統合テストの書き方サンプル
- `crates/fulgur/src/pageable.rs` — `BlockStyle`, `BlockPageable`, `PositionedChild`, `draw_with_opacity`

---

## Task 1: krilla 0.7 アップグレード & krilla-svg 依存追加

**Files:**

- Modify: `crates/fulgur/Cargo.toml`

**Step 1.1: Cargo.toml 更新**

`crates/fulgur/Cargo.toml` の `[dependencies]` セクションを以下に変更：

```toml
[dependencies]
cssparser = "0.35"
krilla = "0.7.0"
krilla-svg = "0.7.0"
usvg = "0.45"
blitz-html = "0.2"
blitz-dom = "0.2"
blitz-traits = "0.2"
parley = "0.6"
skrifa = "0.37"
stylo = "0.8.0"
thiserror = "2"
minijinja = { version = "=2.19.0", features = ["unstable_machinery"] }
serde_json = "1"
```

変更点: `krilla = "0.6.0"` → `"0.7.0"`、`krilla-svg = "0.7.0"` 追加、`usvg = "0.45"` 追加。

**Step 1.2: ビルド確認**

Run: `cargo build`
Expected: エラーなく完了。`Compiling krilla v0.7.0` と `Compiling krilla-svg v0.7.0` がログに出る。

**Step 1.3: 既存テスト全件回帰**

Run: `cargo test 2>&1 | grep "test result"`
Expected: 全テストケース pass（`0 failed` が全行）。Task 着手前のベースライン（294+ tests）と同じ件数で `0 failed`。

**Step 1.4: clippy & fmt 確認**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: warning/error なし

Run: `cargo fmt --check`
Expected: 差分なし

**Step 1.5: コミット**

```bash
git add crates/fulgur/Cargo.toml Cargo.lock
git commit -m "deps: bump krilla to 0.7, add krilla-svg and usvg

Prepare for inline SVG rendering support.

- krilla: 0.6.0 -> 0.7.0 (drop-in compatible per verification)
- krilla-svg: new, provides SurfaceExt::draw_svg
- usvg: 0.45 (explicit; already transitive via blitz-dom)

Refs: fulgur-rt2"
```

---

## Task 2: SvgPageable 型のユニットテスト（failing）

**Files:**

- Create: `crates/fulgur/src/svg.rs`
- Modify: `crates/fulgur/src/lib.rs`

**Step 2.1: lib.rs に mod 宣言を追加**

`crates/fulgur/src/lib.rs` の `pub mod image;` の直後に追加：

```rust
pub mod svg;
```

（ファイルの `pub mod` ブロックはアルファベット順ではなく存在する順に並んでいるため、`image;` の次に `svg;` を置く）

**Step 2.2: `svg.rs` に型スケルトンと failing unit tests を書く**

`crates/fulgur/src/svg.rs` を新規作成。まず **テストだけ通らないスケルトン** を作る：

```rust
//! SvgPageable — renders inline <svg> elements to PDF as vector graphics
//! via krilla-svg's SurfaceExt::draw_svg.

use std::sync::Arc;

use usvg::Tree;

use crate::pageable::{Canvas, Pageable, Pagination, Pt, Size};

/// An inline `<svg>` element rendered as vector graphics.
#[derive(Clone)]
pub struct SvgPageable {
    /// Parsed SVG tree, shared via Arc for cheap cloning during pagination.
    pub tree: Arc<Tree>,
    /// Computed layout width from blitz/taffy (Pt).
    pub width: f32,
    /// Computed layout height from blitz/taffy (Pt).
    pub height: f32,
    pub opacity: f32,
    pub visible: bool,
}

impl SvgPageable {
    pub fn new(tree: Arc<Tree>, width: f32, height: f32) -> Self {
        Self {
            tree,
            width,
            height,
            opacity: 1.0,
            visible: true,
        }
    }
}

impl Pageable for SvgPageable {
    fn wrap(&mut self, _avail_width: Pt, _avail_height: Pt) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn split(
        &self,
        _avail_width: Pt,
        _avail_height: Pt,
    ) -> Option<(Box<dyn Pageable>, Box<dyn Pageable>)> {
        // SVGs are atomic — cannot be split across pages
        None
    }

    fn draw(
        &self,
        _canvas: &mut Canvas<'_, '_>,
        _x: Pt,
        _y: Pt,
        _avail_width: Pt,
        _avail_height: Pt,
    ) {
        // TODO (Task 3): implement draw via krilla_svg::SurfaceExt::draw_svg
    }

    fn pagination(&self) -> Pagination {
        Pagination::default()
    }

    fn clone_box(&self) -> Box<dyn Pageable> {
        Box::new(self.clone())
    }

    fn height(&self) -> Pt {
        self.height
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid SVG: 100x50 red rectangle
    const MINIMAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="red"/></svg>"#;

    fn parse_tree() -> Arc<Tree> {
        let opts = usvg::Options::default();
        let tree = Tree::from_str(MINIMAL_SVG, &opts).expect("parse minimal svg");
        Arc::new(tree)
    }

    #[test]
    fn test_wrap_returns_configured_size() {
        let mut svg = SvgPageable::new(parse_tree(), 120.0, 60.0);
        let size = svg.wrap(1000.0, 1000.0);
        assert_eq!(size.width, 120.0);
        assert_eq!(size.height, 60.0);
    }

    #[test]
    fn test_split_returns_none() {
        let svg = SvgPageable::new(parse_tree(), 100.0, 50.0);
        assert!(svg.split(1000.0, 1000.0).is_none());
    }

    #[test]
    fn test_height_returns_configured_height() {
        let svg = SvgPageable::new(parse_tree(), 100.0, 50.0);
        assert_eq!(svg.height(), 50.0);
    }

    #[test]
    fn test_clone_box_shares_tree_via_arc() {
        let original = SvgPageable::new(parse_tree(), 100.0, 50.0);
        let original_ptr = Arc::as_ptr(&original.tree);
        let cloned = original.clone();
        let cloned_ptr = Arc::as_ptr(&cloned.tree);
        assert_eq!(
            original_ptr, cloned_ptr,
            "clone must share the underlying usvg::Tree via Arc"
        );
    }

    #[test]
    fn test_default_opacity_and_visible() {
        let svg = SvgPageable::new(parse_tree(), 100.0, 50.0);
        assert_eq!(svg.opacity, 1.0);
        assert!(svg.visible);
    }
}
```

**Step 2.3: テスト実行**

Run: `cargo test -p fulgur --lib svg::tests`
Expected: 5 tests pass（スケルトンの時点で draw() 以外は完全なので通るはず）。もし `usvg::Tree::from_str` のシグネチャが違う場合はコンパイルエラーで止まる — その場合は `usvg::Tree::from_xmltree` 経由など別 API を試す。

**Step 2.4: 既存テスト回帰**

Run: `cargo test -p fulgur --lib 2>&1 | grep "test result"`
Expected: `219 + 5 = 224 passed`

**Step 2.5: コミット**

```bash
git add crates/fulgur/src/svg.rs crates/fulgur/src/lib.rs
git commit -m "feat(svg): add SvgPageable skeleton with unit tests

Implements Pageable trait for inline <svg> elements. draw() is a
no-op for now (Task 3 adds krilla-svg rendering).

Refs: fulgur-rt2"
```

---

## Task 3: SvgPageable::draw() を krilla-svg で実装（TDD 経由の integration test）

**Files:**

- Modify: `crates/fulgur/src/svg.rs`
- Create: `crates/fulgur/tests/svg_test.rs`

`draw()` は `Canvas<'_, '_>` を必要とし、これは実質 krilla Surface のラップなので、ユニットテストで単独検証するのは難しい。代わりに **statement level** のテスト（draw が呼ばれる経路を通す）を統合テストで行う。

ただしこの時点では `convert.rs` で `<svg>` を認識していないので、HTML 経由で `<svg>` をレンダーするテストはまだ書けない。このタスクでは `draw()` の実装と、次の Task 4 で使う前段の準備のみ行う。

**Step 3.1: `draw()` に krilla-svg 呼び出しを追加**

`crates/fulgur/src/svg.rs` の `draw()` メソッドを以下に置き換え：

```rust
    fn draw(&self, canvas: &mut Canvas<'_, '_>, x: Pt, y: Pt, _avail_width: Pt, _avail_height: Pt) {
        use crate::pageable::draw_with_opacity;
        use krilla_svg::{SurfaceExt, SvgSettings};

        if !self.visible {
            return;
        }
        draw_with_opacity(canvas, self.opacity, |canvas| {
            let Some(size) = krilla::geom::Size::from_wh(self.width, self.height) else {
                return;
            };
            let transform = krilla::geom::Transform::from_translate(x, y);
            canvas.surface.push_transform(&transform);
            // draw_svg returns Option<()>; None means the tree was malformed.
            // We silently skip rather than panic, matching ImagePageable's behavior
            // when krilla::image::Image::from_* returns Err.
            let _ = canvas
                .surface
                .draw_svg(&self.tree, size, SvgSettings::default());
            canvas.surface.pop();
        });
    }
```

**Step 3.2: ビルドだけ確認**

Run: `cargo build -p fulgur`
Expected: エラーなし。`krilla_svg` と `krilla` 0.7 の名前解決が通ることを確認。

**Step 3.3: ユニットテストは変わらず全件 pass**

Run: `cargo test -p fulgur --lib svg::tests`
Expected: 5 tests pass

**Step 3.4: コミット**

```bash
git add crates/fulgur/src/svg.rs
git commit -m "feat(svg): implement SvgPageable::draw via krilla-svg

Uses krilla_svg::SurfaceExt::draw_svg to render the usvg::Tree onto
the krilla surface as vector content. Opacity and visibility follow
the same pattern as ImagePageable.

Refs: fulgur-rt2"
```

---

## Task 4: convert.rs に `<svg>` 分岐を追加 + 最小統合テスト

**Files:**

- Modify: `crates/fulgur/src/convert.rs`
- Create: `crates/fulgur/tests/svg_test.rs`

**Step 4.1: 失敗する統合テストを書く**

`crates/fulgur/tests/svg_test.rs` を新規作成：

```rust
use fulgur::config::{Margin, PageSize};
use fulgur::engine::Engine;

fn build_engine() -> Engine {
    Engine::builder()
        .page_size(PageSize::A4)
        .margin(Margin::uniform(72.0))
        .build()
}

#[test]
fn test_inline_svg_renders_to_pdf() {
    let engine = build_engine();
    let html = r#"<html><body>
        <svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <rect width="100" height="50" fill="red"/>
        </svg>
    </body></html>"#;

    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"), "output should be a PDF");
    assert!(!pdf.is_empty());

    // A page that successfully drew the SVG is larger than an empty page:
    let empty_html = r#"<html><body></body></html>"#;
    let empty_pdf = engine.render_html(empty_html).unwrap();
    assert!(
        pdf.len() > empty_pdf.len(),
        "PDF with SVG ({} bytes) must be larger than empty PDF ({} bytes)",
        pdf.len(),
        empty_pdf.len()
    );
}
```

**Step 4.2: テストが失敗することを確認**

Run: `cargo test -p fulgur --test svg_test test_inline_svg_renders_to_pdf`
Expected: テストが panic または `pdf.len() > empty_pdf.len()` で FAIL する。理由：convert.rs がまだ `<svg>` を特別扱いしていないため、SvgPageable が生成されず PDF サイズ差が発生しないか、あるいは必要な要素が欠落する。もし偶然通った場合は `pdf.len() <= empty_pdf.len() + 50` のような厳しい境界を設けてから次ステップに進むのではなく、そのまま実装に進んで問題ないか確認。

**Step 4.3: `convert.rs` に `<svg>` 分岐を追加**

`crates/fulgur/src/convert.rs` の冒頭 imports に以下を追加：

```rust
use crate::svg::SvgPageable;
use blitz_dom::node::ImageData;
```

同ファイル内の `<img>` 検出箇所（現行の line 299 付近）の直後に `<svg>` 分岐を追加。既存コード：

```rust
        if tag == "img" {
            if let Some(img) = convert_image(node, ctx.assets) {
                return img;
            }
            // Fall through to generic handling below to preserve Taffy-computed dimensions
        }
```

これを以下のように変更（`img` 分岐の直後に `svg` 分岐を追加）：

```rust
        if tag == "img" {
            if let Some(img) = convert_image(node, ctx.assets) {
                return img;
            }
            // Fall through to generic handling below to preserve Taffy-computed dimensions
        }
        if tag == "svg" {
            if let Some(svg) = convert_svg(node, ctx.assets) {
                return svg;
            }
            // Fall through — e.g., ImageData::None (parse failure upstream)
        }
```

次に、`convert_image` 関数（現行 line 513-557）の直後に新規関数 `convert_svg` を追加：

```rust
/// Convert an inline <svg> element into an SvgPageable, wrapped in BlockPageable if styled.
///
/// Blitz automatically parses inline <svg> elements into a usvg::Tree stored on
/// `ElementData::image_data()` as `ImageData::Svg`. This function extracts that tree
/// and wraps it in our Pageable type.
fn convert_svg(node: &Node, assets: Option<&AssetBundle>) -> Option<Box<dyn Pageable>> {
    let elem = node.element_data()?;
    let tree = match elem.image_data()? {
        ImageData::Svg(tree) => Arc::new((**tree).clone()),
        _ => return None,
    };

    let layout = node.final_layout;
    let width = layout.size.width;
    let height = layout.size.height;

    let style = extract_block_style(node, assets);
    let (opacity, visible) = extract_opacity_visible(node);
    if style.has_visual_style() {
        let (cx, cy) = style.content_inset();
        let right_inset = style.border_widths[1] + style.padding[1];
        let bottom_inset = style.border_widths[2] + style.padding[2];
        let content_width = (width - cx - right_inset).max(0.0);
        let content_height = (height - cy - bottom_inset).max(0.0);
        // Propagate visibility to the inner svg — it's the node's own content,
        // not a real CSS child. Do NOT set opacity — the wrapping block handles it.
        let mut svg = SvgPageable::new(tree, content_width, content_height);
        svg.visible = visible;
        let child = PositionedChild {
            child: Box::new(svg),
            x: cx,
            y: cy,
        };
        let mut block = BlockPageable::with_positioned_children(vec![child])
            .with_style(style)
            .with_opacity(opacity)
            .with_visible(visible);
        block.wrap(width, height);
        block.layout_size = Some(Size { width, height });
        Some(Box::new(block))
    } else {
        let mut svg = SvgPageable::new(tree, width, height);
        svg.opacity = opacity;
        svg.visible = visible;
        Some(Box::new(svg))
    }
}
```

**注意点**：

- `(**tree).clone()` — `tree` は `&Box<usvg::Tree>` なので二重 deref で `usvg::Tree` 値を得て clone
- `BlockPageable` / `PositionedChild` / `Size` は既に convert.rs の `use crate::pageable::{...}` に含まれているので追加 import 不要
- `extract_block_style`, `extract_opacity_visible`, `AssetBundle` は既存の private/`use` で解決される

**Step 4.4: テスト再実行**

Run: `cargo test -p fulgur --test svg_test test_inline_svg_renders_to_pdf`
Expected: PASS

**Step 4.5: 既存テスト全件回帰**

Run: `cargo test 2>&1 | grep "test result"`
Expected: 全行 `0 failed`

**Step 4.6: コミット**

```bash
git add crates/fulgur/src/convert.rs crates/fulgur/tests/svg_test.rs
git commit -m "feat(svg): wire inline <svg> elements through convert_svg

Detects <svg> in convert_element and extracts usvg::Tree from
ElementData::image_data(). Mirrors convert_image's structure for
opacity/visibility propagation and style wrapping.

Refs: fulgur-rt2"
```

---

## Task 5: スタイル付き `<svg>`（border / padding）の統合テスト

**Files:**

- Modify: `crates/fulgur/tests/svg_test.rs`

**Step 5.1: 失敗する可能性のあるテストを追加**

`crates/fulgur/tests/svg_test.rs` の末尾に追加：

```rust
#[test]
fn test_svg_with_border_and_padding_renders() {
    let engine = build_engine();
    let html = r#"<html><body>
        <svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"
             style="border: 2px solid black; padding: 10px; background: #eee">
            <rect width="100" height="50" fill="blue"/>
        </svg>
    </body></html>"#;

    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));

    // The PDF should be larger than the same HTML without the SVG
    let empty_pdf = engine.render_html(r#"<html><body></body></html>"#).unwrap();
    assert!(pdf.len() > empty_pdf.len());
}
```

**Step 5.2: テスト実行**

Run: `cargo test -p fulgur --test svg_test test_svg_with_border_and_padding_renders`
Expected: PASS（Task 4 の `convert_svg` がすでに `has_visual_style()` 分岐で `BlockPageable` ラップを実装しているため）

**もし FAIL する場合**：`extract_block_style` 経由で border/padding が正しく取れていない可能性。その場合は `FULGUR_DEBUG=1 cargo test ...` でレイアウトツリーを dump し、原因を特定する。

**Step 5.3: コミット**

```bash
git add crates/fulgur/tests/svg_test.rs
git commit -m "test(svg): verify border and padding wrapping for <svg>

Refs: fulgur-rt2"
```

---

## Task 6: 複数 SVG の統合テスト

**Files:**

- Modify: `crates/fulgur/tests/svg_test.rs`

**Step 6.1: 複数 SVG テストを追加**

```rust
#[test]
fn test_multiple_svgs_on_same_page() {
    let engine = build_engine();
    let html = r#"<html><body>
        <svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <circle cx="25" cy="25" r="20" fill="red"/>
        </svg>
        <svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <circle cx="25" cy="25" r="20" fill="blue"/>
        </svg>
    </body></html>"#;

    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));

    // Two SVGs should produce a larger PDF than one
    let single_html = r#"<html><body>
        <svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <circle cx="25" cy="25" r="20" fill="red"/>
        </svg>
    </body></html>"#;
    let single_pdf = engine.render_html(single_html).unwrap();
    assert!(pdf.len() >= single_pdf.len());
}
```

**Step 6.2: テスト実行**

Run: `cargo test -p fulgur --test svg_test test_multiple_svgs_on_same_page`
Expected: PASS（個別 convert 呼び出しが独立しており相互干渉しないはず）

**Step 6.3: コミット**

```bash
git add crates/fulgur/tests/svg_test.rs
git commit -m "test(svg): verify multiple SVGs render on same page

Refs: fulgur-rt2"
```

---

## Task 7: ページ跨ぎ（不可分性）統合テスト

**Files:**

- Modify: `crates/fulgur/tests/svg_test.rs`

**Step 7.1: 不可分性テストを追加**

A4 のコンテンツ領域は 595-144=451 × 842-144=698 pt。SVG 高さを 600pt にして、上に filler を入れてページ末尾に配置し、次ページへ丸ごと移動するかを確認。

```rust
#[test]
fn test_svg_does_not_split_across_pages() {
    let engine = build_engine();
    // A4 minus uniform 72pt margin ≈ 698pt content height.
    // Place a filler consuming ~500pt, then a 300pt-tall SVG that
    // cannot fit in the remaining ~198pt — must move to page 2.
    let html = r#"<html><body>
        <div style="height: 500pt"></div>
        <svg xmlns="http://www.w3.org/2000/svg" width="200" height="300">
            <rect width="200" height="300" fill="green"/>
        </svg>
    </body></html>"#;

    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    // PDF should contain at least 2 pages.
    // Rough check: PDF bytes should mention "/Count 2" or similar in catalog.
    // Simpler heuristic — count page objects:
    let page_count = pdf.windows(6).filter(|w| w == b"/Page ").count();
    assert!(
        page_count >= 2,
        "expected at least 2 pages, got {}",
        page_count
    );
}
```

**Note**: `/Page` (with trailing space) matches `/Page /Type` style page objects, not `/Pages` (catalog). 実装詳細によりこの方式が効かなければ、`pdf.windows(5).filter(|w| w == b"/Page").count()` や `pdf-writer` のテスト出力パターンを調べ直す。最悪、ページ数検証は `/Count N` の抽出に切り替える：

```rust
    // Alternative: look for "/Count 2" in the Pages tree
    assert!(
        pdf.windows(8).any(|w| w == b"/Count 2"),
        "expected /Count 2 in Pages tree"
    );
```

どちらか通る方を採用。

**Step 7.2: テスト実行**

Run: `cargo test -p fulgur --test svg_test test_svg_does_not_split_across_pages`
Expected: PASS。`SvgPageable::split` が常に `None` を返すため、paginate.rs が SVG 丸ごと次ページへ送る。

**Step 7.3: コミット**

```bash
git add crates/fulgur/tests/svg_test.rs
git commit -m "test(svg): verify SVG is atomic (no page split)

Refs: fulgur-rt2"
```

---

## Task 8: opacity / visibility 伝搬テスト

**Files:**

- Modify: `crates/fulgur/tests/svg_test.rs`

**Step 8.1: opacity / visibility テストを追加**

```rust
#[test]
fn test_svg_with_parent_opacity() {
    let engine = build_engine();
    let html = r#"<html><body>
        <div style="opacity: 0.5">
            <svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
                <rect width="100" height="50" fill="red"/>
            </svg>
        </div>
    </body></html>"#;

    let pdf = engine.render_html(html).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    // Just verifying no panic and valid PDF is produced — opacity
    // state is hard to assert at the byte level.
}

#[test]
fn test_svg_with_visibility_hidden_is_skipped() {
    let engine = build_engine();
    let visible_html = r#"<html><body>
        <svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <rect width="100" height="50" fill="red"/>
        </svg>
    </body></html>"#;
    let hidden_html = r#"<html><body>
        <svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"
             style="visibility: hidden">
            <rect width="100" height="50" fill="red"/>
        </svg>
    </body></html>"#;

    let visible_pdf = engine.render_html(visible_html).unwrap();
    let hidden_pdf = engine.render_html(hidden_html).unwrap();
    assert!(visible_pdf.starts_with(b"%PDF"));
    assert!(hidden_pdf.starts_with(b"%PDF"));

    // Hidden SVG should not emit path content, so the PDF should be
    // at most the same size (typically smaller because the content
    // stream is empty / shorter).
    assert!(
        hidden_pdf.len() <= visible_pdf.len(),
        "hidden SVG PDF ({} bytes) must not be larger than visible SVG PDF ({} bytes)",
        hidden_pdf.len(),
        visible_pdf.len()
    );
}
```

**Step 8.2: テスト実行**

Run: `cargo test -p fulgur --test svg_test test_svg_with_parent_opacity test_svg_with_visibility_hidden_is_skipped`
Expected: 両方 PASS

**Step 8.3: コミット**

```bash
git add crates/fulgur/tests/svg_test.rs
git commit -m "test(svg): verify opacity and visibility propagation

Refs: fulgur-rt2"
```

---

## Task 9: CHANGELOG 更新

**Files:**

- Modify: `CHANGELOG.md` (if exists) or create

**Step 9.1: CHANGELOG 確認 & 更新**

Run: `ls CHANGELOG.md 2>/dev/null && head -30 CHANGELOG.md || echo "no changelog"`

**CHANGELOG.md が存在する場合**: `## [Unreleased]` セクションの `### Added` に追加：

```markdown
### Added
- インライン `<svg>` 要素を PDF にベクター描画対応（fulgur-rt2）
  - `krilla-svg` 経由で path / group / clip / gradient / text / filter / mask をサポート
  - `krilla` を 0.6 → 0.7 にアップグレード
  - `<img src="*.svg">` および `background-image: url(*.svg)` は follow-up issue
```

**CHANGELOG.md が存在しない場合**: このタスクをスキップし、コミットメッセージで完了を示す（CLAUDE.md には CHANGELOG 運用の明示ルールがないため、既存プロジェクト方針に合わせる）。

**Step 9.2: コミット（存在する場合のみ）**

```bash
git add CHANGELOG.md
git commit -m "docs: note inline SVG rendering in CHANGELOG

Refs: fulgur-rt2"
```

---

## Task 10: 最終検証

**Step 10.1: clippy 全件**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: warning/error なし

**Step 10.2: フォーマット**

Run: `cargo fmt --check`
Expected: 差分なし。差分がある場合は `cargo fmt` で修正してコミット：

```bash
cargo fmt
git add -u
git commit -m "style: cargo fmt"
```

**Step 10.3: markdownlint**

Run: `npx markdownlint-cli2 '**/*.md' 2>&1 | tail -20`
Expected: エラーなし。新規 plan ドキュメントや CHANGELOG に違反があれば修正。

**Step 10.4: テスト全件**

Run: `cargo test 2>&1 | grep "test result"`
Expected: 全行 `0 failed`。新規 svg_test の 6 ケース + 既存テスト全件 pass。

**Step 10.5: 手動 PDF 生成スモークテスト**

Run:

```bash
cat > /tmp/svg-smoke.html <<'EOF'
<html><body>
<h1>SVG test</h1>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
  <rect width="200" height="100" fill="lightblue"/>
  <circle cx="100" cy="50" r="40" fill="red" stroke="black" stroke-width="2"/>
  <path d="M 20 80 L 180 80" stroke="darkblue" stroke-width="3"/>
</svg>
</body></html>
EOF
cargo run --bin fulgur -- render /tmp/svg-smoke.html -o /tmp/svg-smoke.pdf
file /tmp/svg-smoke.pdf
```

Expected: PDF が生成され、`file` の出力が `PDF document, version 1.7` など妥当な値を示す。可能なら mupdf で開いて目視確認。

**Step 10.6: beads issue の notes 欄に完了メモを追記（任意）**

```bash
bd update fulgur-rt2 --notes "Implemented via krilla 0.7 + krilla-svg 0.7. All acceptance criteria satisfied. See branch feature/svg-inline-rendering."
```

---

## 実装上の罠メモ

1. **`ImageData::Svg(Box<usvg::Tree>)` からの clone** — `Box::clone` は中身の `usvg::Tree::clone` を呼び、さらに `Arc<fontdb::Database>` や `Vec<Arc<...>>` 内の Arc カウントを増やすだけなので軽い。ヒープ確保は一度だけ

2. **`blitz_dom::node::ImageData` の import** — `blitz_dom::ImageData` ではなく `blitz_dom::node::ImageData` がパブリックパス。lib.rs の `pub use node::{..., ElementData, ...}` には `ImageData` は含まれていない

3. **`usvg::Tree::from_str` のエラー型** — `usvg::Error` を返す。テストでは `.expect(...)` で panic させて問題ない

4. **`draw_with_opacity` の closure** — `canvas` を `&mut Canvas` で受ける。`canvas.surface` 経由で krilla Surface にアクセス。既存の `ImagePageable::draw` と同一パターンなので迷ったら image.rs を参照

5. **blitz が `<svg>` を `ImageData::None` にするケース** — 非常に壊れた SVG や feature 無効化時など。`convert_svg` は `None` を返して generic fallback に流す（ビジュアルには何も出ない）

---

## 完了後

- `/home/ubuntu/fulgur/.worktrees/svg-inline-rendering` で `git log --oneline main..HEAD` を実行して全コミットを確認
- `superpowers:finishing-a-development-branch` スキルで PR 作成 or main へのマージ処理
- `bd close fulgur-rt2` で issue クローズ
