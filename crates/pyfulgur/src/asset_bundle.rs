use crate::error::map_fulgur_error;
use fulgur::AssetBundle;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::path::PathBuf;

#[pyclass(name = "AssetBundle", module = "pyfulgur")]
pub struct PyAssetBundle {
    pub(crate) inner: AssetBundle,
}

#[pymethods]
impl PyAssetBundle {
    #[new]
    fn new() -> Self {
        Self {
            inner: AssetBundle::new(),
        }
    }

    fn add_css(&mut self, css: &str) {
        self.inner.add_css(css);
    }

    fn add_css_file(&mut self, path: PathBuf) -> PyResult<()> {
        self.inner.add_css_file(path).map_err(map_fulgur_error)
    }

    fn add_font_file(&mut self, path: PathBuf) -> PyResult<()> {
        self.inner.add_font_file(path).map_err(map_fulgur_error)
    }

    fn add_image(&mut self, name: &str, data: &Bound<'_, PyBytes>) {
        self.inner.add_image(name, data.as_bytes().to_vec());
    }

    fn add_image_file(&mut self, name: &str, path: PathBuf) -> PyResult<()> {
        self.inner
            .add_image_file(name, path)
            .map_err(map_fulgur_error)
    }
}

impl PyAssetBundle {
    /// Engine builder/constructor に渡すために内部の `AssetBundle` を取り出す。
    /// 呼び出し後 inner は空の `AssetBundle::new()` にリセットされる。
    pub(crate) fn take_inner(&mut self) -> AssetBundle {
        std::mem::replace(&mut self.inner, AssetBundle::new())
    }
}
