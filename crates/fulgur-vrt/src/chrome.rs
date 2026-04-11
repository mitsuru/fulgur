//! Chromium screenshot adapter (skeleton only — full impl tracked in future issue).
//!
//! This module is gated behind the `chrome-golden` feature. Its purpose is to
//! hold the fixed Chromium revision and the future entry point used to
//! regenerate `goldens/chrome/` images. The real implementation (fetch
//! Chromium, launch it via `chromiumoxide`, capture screenshots, shut down
//! cleanly) is tracked in a follow-up issue and intentionally out of scope
//! for the initial VRT crate scaffolding.
//!
//! Running `cargo check -p fulgur-vrt --features chrome-golden` must compile
//! cleanly with this stub.

use image::RgbaImage;
use std::path::PathBuf;

/// Pinned Chromium build revision. Bump this only when regenerating
/// `goldens/chrome/` — the golden update PR must change this constant and
/// the PNGs in the same commit so the on-disk references stay in lockstep
/// with the browser that produced them.
pub const CHROMIUM_REVISION: &str = "1280000";

/// Filesystem location where the fetched Chromium build is cached.
/// Namespaced by `CHROMIUM_REVISION` so a bump does not reuse an older
/// download.
pub fn fetcher_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fulgur-vrt")
        .join(format!("chromium-{CHROMIUM_REVISION}"))
}

/// Capture a Chromium screenshot of the given HTML at the given viewport.
///
/// **Not yet implemented.** See the crate docs for the tracking issue.
/// The signature is fixed here so that Task 6's runner can reference it
/// unconditionally once the real implementation lands.
#[allow(dead_code)]
pub async fn screenshot_html(_html: &str, _viewport: (u32, u32)) -> anyhow::Result<RgbaImage> {
    todo!("chrome golden generation is tracked in a follow-up issue")
}
