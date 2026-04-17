//! `Fulgur::Pdf` — `Engine#render_html` が返す結果オブジェクト。
//!
//! バイト列を保持し、`to_s` (ASCII-8BIT String), `bytesize`, `to_base64`,
//! `to_data_uri`, `inspect` を提供する。

use base64::Engine as _;
use magnus::{Error, Module, RModule, RString, Ruby, method};

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
}

pub fn define(_ruby: &Ruby, fulgur: &RModule) -> Result<(), Error> {
    let class = fulgur.define_class("Pdf", magnus::class::object())?;
    class.define_method("bytesize", method!(RbPdf::bytesize, 0))?;
    class.define_method("to_s", method!(RbPdf::to_s, 0))?;
    class.define_method("to_base64", method!(RbPdf::to_base64, 0))?;
    class.define_method("to_data_uri", method!(RbPdf::to_data_uri, 0))?;
    class.define_method("inspect", method!(RbPdf::inspect, 0))?;
    Ok(())
}
