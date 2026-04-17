# pyfulgur

Python bindings for [fulgur](https://github.com/mitsuru/fulgur) — an offline, deterministic HTML/CSS to PDF conversion library written in Rust.

## Status

**Alpha (v0.0.2).** Core `Engine` / `AssetBundle` / `PageSize` / `Margin` / `render_html` API is available. Batch rendering, sandboxing, and template engine wiring are planned for later releases.

## Install

```bash
pip install pyfulgur
```

Pre-built wheels are published for manylinux (x86_64, aarch64), macOS (arm64, x86_64), and Windows (x86_64).

## Quick start

```python
from pyfulgur import AssetBundle, Engine, PageSize

bundle = AssetBundle()
bundle.add_css("body { font-family: sans-serif; }")

engine = Engine(page_size=PageSize.A4, assets=bundle)
pdf_bytes = engine.render_html("<h1>Hello, world!</h1>")

with open("output.pdf", "wb") as f:
    f.write(pdf_bytes)
```

Builder style:

```python
engine = (
    Engine.builder()
    .page_size(PageSize.A4)
    .landscape(False)
    .title("My doc")
    .assets(bundle)
    .build()
)
engine.render_html_to_file("<h1>Hi</h1>", "out.pdf")
```

## API surface

- `Engine(**kwargs)` / `Engine.builder()` → `EngineBuilder`
- `AssetBundle`: `add_css`, `add_css_file`, `add_font_file`, `add_image`, `add_image_file`
- `PageSize`: `A4`, `LETTER`, `A3`, `custom(w_mm, h_mm)`, `.landscape()`
- `Margin`: `Margin(top, right, bottom, left)`, `Margin.uniform(pt)`, `Margin.symmetric(v, h)`, `Margin.uniform_mm(mm)`
- Exceptions: `FileNotFoundError`, `ValueError`, `pyfulgur.RenderError`

## Links

- [fulgur on GitHub](https://github.com/mitsuru/fulgur)
- [fulgur on crates.io](https://crates.io/crates/fulgur)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
