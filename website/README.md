# fulgur.dev website

The MkDocs Material source for [https://fulgur.dev](https://fulgur.dev).

## Local development

Prerequisites: [`mise`](https://mise.jdx.dev/) installed (it will fetch
Python 3.12 and `uv` automatically).

```bash
# Install Python dependencies
mise run docs:install

# Run dev server at http://127.0.0.1:8000
mise run docs:serve

# Build the static site to website/site/
mise run docs:build
```

## Structure

```text
website/
├── docs/                     # All Markdown sources (en + ja colocated)
│   ├── index.en.md           # English (default, served at /)
│   └── index.ja.md           # 日本語 (served at /ja/)
├── overrides/                # Material theme overrides
├── mkdocs.yml                # Site configuration
└── pyproject.toml + uv.lock  # Python dependency lockfile
```

Translations follow the [`mkdocs-static-i18n`](https://github.com/ultrabug/mkdocs-static-i18n)
suffix structure: each page lives as a pair of `<page>.en.md` and
`<page>.ja.md` in the same directory. Adding a new page means creating
both files side-by-side so translation status is visible at a glance.

## Deploying

Deployment is handled by GitHub Actions; see beads issue `fulgur-bxw`.
