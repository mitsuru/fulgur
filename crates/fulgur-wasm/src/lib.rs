//! WebAssembly bindings for fulgur.
//!
//! Scope of this crate (B-1): expose a single `render_html` entry point
//! that takes an HTML string and returns the rendered PDF as a byte
//! array. Fonts, CSS resources, and images are out of scope here —
//! callers must keep the input HTML to constructs that need no font
//! (background colours, borders, sized boxes). Subsequent steps (B-2,
//! B-3) will add an `AssetBundle` bridge that accepts JS-side
//! `Uint8Array` payloads for fonts/CSS/images, plus richer rendering
//! options.
//!
//! Browser-class targets (`wasm32-unknown-unknown`) only. WASI requires
//! a different `getrandom` backend selection (see
//! `crates/fulgur/Cargo.toml`).
//!
//! Tracking: fulgur-iym (strategic v0.7.0), fulgur-id9x (this step).

use fulgur::Engine;
use wasm_bindgen::prelude::*;

/// Render the given HTML string to a PDF byte array.
///
/// `wasm-bindgen` translates the returned `Vec<u8>` into a JavaScript
/// `Uint8Array`; callers can wrap it directly in a `Blob` and produce
/// a download URL.
#[wasm_bindgen]
pub fn render_html(html: &str) -> Result<Vec<u8>, JsError> {
    Engine::builder()
        .build()
        .render_html(html)
        .map_err(|e| JsError::new(&format!("{e}")))
}
