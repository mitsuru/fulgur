# FAQ

## CSS Loading

### Q: Should I use the `--css` flag or `<link rel="stylesheet">`?

**Prefer `<link rel="stylesheet">` in your HTML.** It is the recommended way to
attach stylesheets in fulgur.

```html
<head>
  <link rel="stylesheet" href="./style.css">
</head>
```

### Q: Why? Aren't the two paths equivalent?

In principle they should be, but the `--css` flag currently has known
limitations that `<link>` does not:

- **GCPM `@page` rules (running headers/footers) may not be applied.**
  Stylesheets injected via `--css` can fail to register margin-box content,
  meaning your `position: running(...)` elements never appear in the output.

- **Inline SVG `<text>` rendering can be affected.** When a stylesheet is
  loaded twice — once via `<link>` and once via `--css` — text inside inline
  `<svg>` elements may fail to render.

These differences come from how fulgur's CSS pipeline currently treats
flag-injected stylesheets versus stylesheets resolved from the document.
Loading CSS through `<link>` avoids both issues entirely and is the path that
all bundled examples use.

### Q: When is `--css` still useful?

- **Ad-hoc styling** when you don't want to (or can't) modify the HTML —
  for example, applying a print stylesheet to a third-party document.
- **Scripted batch jobs** where the HTML template is fixed and styling
  varies per invocation.

For these cases, `--css` works for normal block/inline styling. Avoid relying
on it for GCPM features or inline SVG until the pipeline is unified.

### Q: How do I migrate an existing `--css`-based example?

1. Add `<link rel="stylesheet" href="./style.css">` inside the `<head>` of
   your HTML, with a path relative to the HTML file.
2. Drop the `--css style.css` argument from your CLI invocation or build
   script.
3. Re-render and verify the output. If anything that was missing now appears,
   you were hitting one of the limitations above.
