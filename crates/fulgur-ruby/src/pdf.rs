//! `Fulgur::Pdf` — `Engine#render_html` が返す結果オブジェクト。
//!
//! バイト列を保持し、`to_s` (ASCII-8BIT String), `bytesize`, `to_base64`,
//! `to_data_uri`, `inspect`, `write_to_path`, `write_to_io` を提供する。

use base64::Engine as _;
use magnus::{Error, Module, RModule, RString, Ruby, Value, method, prelude::*};

/// `write_to_io` でチャンク単位に分割するサイズ (64 KiB)。
const CHUNK_SIZE: usize = 64 * 1024;

#[magnus::wrap(class = "Fulgur::Pdf", free_immediately, size)]
pub struct RbPdf {
    pub(crate) bytes: Vec<u8>,
}

impl RbPdf {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    fn bytesize(&self) -> usize {
        self.bytes.len()
    }

    fn to_s(&self) -> RString {
        // `RString::from_slice` は ASCII-8BIT (binary) encoding の Ruby String を返す。
        RString::from_slice(&self.bytes)
    }

    fn to_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.bytes)
    }

    fn to_data_uri(&self) -> String {
        format!("data:application/pdf;base64,{}", self.to_base64())
    }

    fn inspect(&self) -> String {
        format!("#<Fulgur::Pdf bytesize={}>", self.bytes.len())
    }

    /// PDF バイト列をファイルパスに書き込む。
    ///
    /// `std::fs::write` を利用し、エラーは `fulgur::Error::Io` 経由で
    /// `map_fulgur_error` に委譲する (NotFound なら `Errno::ENOENT`, それ以外は
    /// `Fulgur::RenderError`)。
    fn write_to_path(&self, path: String) -> Result<(), Error> {
        let ruby = Ruby::get().expect("ruby vm");
        std::fs::write(&path, &self.bytes).map_err(|e| {
            let fulgur_err = fulgur::Error::Io(e);
            crate::error::map_fulgur_error(&ruby, fulgur_err)
        })
    }

    /// PDF バイト列を Ruby IO オブジェクトに書き込む。
    ///
    /// - `binmode` を呼んでエンコーディング変換を防ぐ (`StringIO` / `File` /
    ///   `Tempfile` / `$stdout` などすべてで定義されている)。
    /// - 64 KiB チャンクに分割して `write` を呼ぶ。ピークメモリは `N + CHUNK` に収まる。
    fn write_to_io(&self, io: Value) -> Result<(), Error> {
        let _: Value = io.funcall("binmode", ())?;

        for chunk in self.bytes.chunks(CHUNK_SIZE) {
            let rb_bytes = RString::from_slice(chunk);
            let _: Value = io.funcall("write", (rb_bytes,))?;
        }
        Ok(())
    }
}

pub fn define(_ruby: &Ruby, fulgur: &RModule) -> Result<(), Error> {
    let class = fulgur.define_class("Pdf", magnus::class::object())?;
    class.define_method("bytesize", method!(RbPdf::bytesize, 0))?;
    class.define_method("to_s", method!(RbPdf::to_s, 0))?;
    class.define_method("to_base64", method!(RbPdf::to_base64, 0))?;
    class.define_method("to_data_uri", method!(RbPdf::to_data_uri, 0))?;
    class.define_method("inspect", method!(RbPdf::inspect, 0))?;
    class.define_method("write_to_path", method!(RbPdf::write_to_path, 1))?;
    class.define_method("write_to_io", method!(RbPdf::write_to_io, 1))?;
    Ok(())
}
