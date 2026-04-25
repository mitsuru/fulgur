# fulgur WASM demo

Browser-side `HTML → PDF` rendering powered by `fulgur-wasm`. Renders entirely
in the browser; no network calls.

## Scope (B-1)

- Single entry point: `render_html(html: string) → Uint8Array`
- No fonts, CSS resources, or images. The default sample HTML is a coloured
  `<div>` so nothing depends on a font being available.
- Subsequent steps (B-2, B-3) will add an `AssetBundle` bridge for fonts /
  CSS / images and richer rendering options.

## Build

Requires [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

```bash
wasm-pack build crates/fulgur-wasm --target web --dev \
  --out-dir ../../examples/wasm-demo/pkg
```

This populates `examples/wasm-demo/pkg/` with `fulgur_wasm.js`,
`fulgur_wasm_bg.wasm`, and TypeScript declarations.

## Run

ES modules require a real HTTP origin (file:// will not work). Any static
server is fine; for a quick check:

```bash
cd examples/wasm-demo
python3 -m http.server 8000
# then visit http://localhost:8000/
```

Edit the HTML in the textarea, click "Render PDF", and the browser will
download `output.pdf`.

## Notes

- `--dev` builds produce a ~37 MB `.wasm`. `wasm-pack build … --release` (no
  flag = release) shrinks it but currently still ships every fulgur dependency.
  Aggressive size reduction (`wasm-opt`, dead-code analysis) is part of the
  later B-3 / scope C work, not B-1.
- The first call after page load incurs a one-time WASM compile cost on top
  of the render itself; subsequent calls reuse the instance.

## Tracking

- `fulgur-iym` (strategic v0.7.0) — overall WASM bet
- `fulgur-id9x` (this step) — B-1: bare wasm-bindgen wrapper
- `crates/fulgur/CLAUDE.md` ※memory `project_wasm_resource_bridging.md` —
  scope 1 / 3a / 3b stage design
