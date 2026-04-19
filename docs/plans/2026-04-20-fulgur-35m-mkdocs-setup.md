# fulgur-35m: MkDocs Material Setup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Phase A 公式サイト基盤として `website/` ディレクトリと MkDocs Material 環境を構築し、後続の Phase A/B タスク (cna/bxw/lgr/qef/e9k/ljj/aax) が乗る土台を作る。

**Architecture:** `website/` をリポジトリ直下に新設し、既存 `docs/` (plans/ADR) と完全分離する。Python 依存は `uv` + `pyproject.toml` で管理し、`uv.lock` をチェックインして再現性を保証する。i18n は `mkdocs-static-i18n` の `suffix` 構造を採用、`<page>.en.md` と `<page>.ja.md` を同ディレクトリに同居させて翻訳ペアを可視化する。`mise.toml` の task として `docs:install` / `docs:serve` / `docs:build` を提供し、Rust ビルドと共通の DX に統合する。

**Tech Stack:**

- MkDocs 1.6 + Material theme 9.5
- mkdocs-static-i18n 1.2 (suffix 構造)
- pymdown-extensions 10.7+ (admonition, tabbed, snippets, superfences mermaid 統合)
- uv (Python 依存解決)
- mise.toml task (DX 統合)

**Reference:** beads issue `fulgur-35m` の design / acceptance フィールド参照。

---

## Task 1: .gitignore 更新

**Files:**

- Modify: `.gitignore`

**Step 1: 現在の .gitignore を確認**

```bash
cat .gitignore
```

**Step 2: website/site/ と website/.venv/ を追加**

`.gitignore` の末尾に以下を追記:

```text
website/site
website/.venv
```

**Step 3: 確認**

```bash
cat .gitignore
```

期待: 上記2行が追加されていること。

**Step 4: コミット**

```bash
git add .gitignore
git commit -m "chore(website): ignore mkdocs build output and uv venv (fulgur-35m)"
```

---

## Task 2: website/pyproject.toml 作成

**Files:**

- Create: `website/pyproject.toml`

**Step 1: ディレクトリ作成**

```bash
mkdir -p website
```

**Step 2: pyproject.toml を書き込む**

`website/pyproject.toml`:

```toml
[project]
name = "fulgur-website"
version = "0.0.0"
description = "fulgur.dev documentation site"
requires-python = ">=3.11"
dependencies = [
    "mkdocs-material>=9.5",
    "mkdocs-static-i18n>=1.2",
    "pymdown-extensions>=10.7",
]

[tool.uv]
package = false
```

**Step 3: コミット (uv.lock は次タスクで生成して合算コミット)**

スキップ。次タスクで lock 一緒にコミット。

---

## Task 3: uv で依存解決し uv.lock 生成

**Files:**

- Create: `website/uv.lock` (uv が自動生成)

**Step 1: uv sync を実行**

```bash
cd website && uv sync
```

期待: `.venv/` が生成され、`uv.lock` が作成される。エラーなく完了。

**Step 2: uv.lock の存在確認**

```bash
ls -la website/uv.lock
```

期待: ファイルが存在する。

**Step 3: 主要パッケージがインストールされたか確認**

```bash
cd website && uv run mkdocs --version
```

期待: `mkdocs, version 1.6.x` のような出力。

**Step 4: コミット**

```bash
git add website/pyproject.toml website/uv.lock
git commit -m "chore(website): add pyproject.toml + uv.lock for mkdocs deps (fulgur-35m)"
```

---

## Task 4: ディレクトリ構造とプレースホルダ index.{en,ja}.md

**Files:**

- Create: `website/docs/index.en.md`
- Create: `website/docs/index.ja.md`
- Create: `website/overrides/.gitkeep`

**Step 1: ディレクトリ作成**

```bash
mkdir -p website/docs/assets website/overrides
```

**Step 2: 英語版 index.en.md (プレースホルダ)**

`website/docs/index.en.md`:

```markdown
# Fulgur

A modern, lightweight HTML/CSS to PDF library for Rust.

> This page is a placeholder. The landing content is tracked in beads issue `fulgur-cna`.
```

**Step 3: 日本語版 index.ja.md (プレースホルダ)**

`website/docs/index.ja.md`:

```markdown
# Fulgur

Rust 製の軽量 HTML/CSS to PDF ライブラリ。

> このページはプレースホルダーです。ランディングコンテンツは beads issue `fulgur-cna` で管理しています。
```

**Step 4: overrides/ を空のまま追跡対象にする**

```bash
touch website/overrides/.gitkeep
```

**Step 5: コミット**

```bash
git add --sparse website/docs website/overrides
git commit -m "feat(website): add placeholder index pages and overrides scaffold (fulgur-35m)"
```

---

## Task 5: ロゴ・favicon プレースホルダ SVG

**Files:**

- Create: `website/docs/assets/logo.svg`
- Create: `website/docs/assets/favicon.svg`

**Step 1: logo.svg (紫の稲妻アイコン、暫定)**

`website/docs/assets/logo.svg`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
  <path d="M13 2L3 14h7l-1 8 10-12h-7l1-8z"/>
</svg>
```

**Step 2: favicon.svg (同形シンプル版)**

`website/docs/assets/favicon.svg`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#673AB7">
  <path d="M13 2L3 14h7l-1 8 10-12h-7l1-8z"/>
</svg>
```

**Step 3: コミット**

```bash
git add website/docs/assets
git commit -m "feat(website): add placeholder logo and favicon (fulgur-35m)"
```

注: `currentColor` で塗りを theme palette に追従させる。favicon は単独色でブラウザタブに見える色を指定 (deep purple #673AB7)。

---

## Task 6: mkdocs.yml 設定

**Files:**

- Create: `website/mkdocs.yml`

**Step 1: mkdocs.yml を書き込む**

`website/mkdocs.yml`:

```yaml
site_name: Fulgur
site_description: A modern, lightweight HTML/CSS to PDF library for Rust
site_url: https://fulgur.dev/
repo_url: https://github.com/fulgur-rs/fulgur
repo_name: fulgur-rs/fulgur
edit_uri: edit/main/website/docs/

docs_dir: docs

theme:
  name: material
  custom_dir: overrides
  language: en
  logo: assets/logo.svg
  favicon: assets/favicon.svg
  palette:
    - media: "(prefers-color-scheme: light)"
      scheme: default
      primary: deep purple
      accent: amber
      toggle:
        icon: material/brightness-7
        name: Switch to dark mode
    - media: "(prefers-color-scheme: dark)"
      scheme: slate
      primary: deep purple
      accent: amber
      toggle:
        icon: material/brightness-4
        name: Switch to light mode
  features:
    # navigation.instant は mkdocs-static-i18n の language switcher と
    # 非互換 (strict モード時に warning -> error)
    - navigation.tracking
    - navigation.tabs
    - navigation.sections
    - navigation.top
    - search.suggest
    - search.highlight
    - content.code.copy
    - content.code.annotate
    - content.tabs.link

plugins:
  - search
  - i18n:
      docs_structure: suffix
      languages:
        - locale: en
          default: true
          name: English
          build: true
        - locale: ja
          name: 日本語
          build: true
          nav_translations:
            Home: ホーム

markdown_extensions:
  - admonition
  - attr_list
  - md_in_html
  - footnotes
  - tables
  - toc:
      permalink: true
  - pymdownx.details
  - pymdownx.superfences:
      custom_fences:
        - name: mermaid
          class: mermaid
          format: !!python/name:pymdownx.superfences.fence_code_format
  - pymdownx.tabbed:
      alternate_style: true
  - pymdownx.snippets:
      base_path:
        - .
        - ..
  - pymdownx.highlight:
      anchor_linenums: true
      line_spans: __span
      pygments_lang_class: true
  - pymdownx.inlinehilite
  - pymdownx.tasklist:
      custom_checkbox: true
  - pymdownx.emoji:
      emoji_index: !!python/name:material.extensions.emoji.twemoji
      emoji_generator: !!python/name:material.extensions.emoji.to_svg

nav:
  - Home: index.md
```

**Step 2: build --strict で動作確認**

```bash
cd website && uv run mkdocs build --strict
```

期待: エラー・警告なしで `website/site/` にサイトが生成される。

**Step 3: 出力確認**

```bash
ls website/site/
ls website/site/ja/
```

期待: 英語版が `website/site/` に、日本語版が `website/site/ja/` に出力される。

**Step 4: 失敗時のリカバリ**

`mkdocs build --strict` が失敗した場合の代表ケース:

- `i18n` プラグイン認識失敗 → `pip list | grep i18n` で `mkdocs-static-i18n` の入っているか確認
- `pymdownx.snippets` の `base_path` 解釈失敗 → `..` を外す (現状は使わない)
- `nav_translations` キー不正 → 最新の plugin docs を確認
- `default: true` の解釈エラー → 古いバージョンの場合 `default_language: en` を試す

エラーメッセージを精読してから修正。

**Step 5: コミット**

```bash
git add website/mkdocs.yml
git commit -m "feat(website): add mkdocs.yml with Material theme and i18n (fulgur-35m)"
```

---

## Task 7: mise.toml に Python ツールと docs タスクを追加

**Files:**

- Modify: `mise.toml`

**Step 1: 現在の mise.toml を確認**

```bash
cat mise.toml
```

**Step 2: `[tools]` に Python と uv を追加**

`mise.toml` の `[tools]` セクションを以下に変更:

```toml
[tools]
rust = "latest"
python = "3.12"
uv = "latest"
```

**Step 3: docs:install/serve/build タスクを追加**

`mise.toml` の末尾 (update-examples タスクの後) に追記:

```toml
[tasks."docs:install"]
description = "Install website Python deps via uv"
dir = "website"
run = "uv sync"

[tasks."docs:serve"]
description = "Run MkDocs dev server (http://127.0.0.1:8000)"
dir = "website"
depends = ["docs:install"]
run = "uv run mkdocs serve"

[tasks."docs:build"]
description = "Build static site to website/site/"
dir = "website"
depends = ["docs:install"]
run = "uv run mkdocs build --strict"
```

**Step 4: タスク認識確認**

```bash
mise tasks ls | grep docs
```

期待: `docs:install`, `docs:serve`, `docs:build` が表示される (mise が CLI から見える場合)。

**Step 5: docs:build 実行確認**

```bash
mise run docs:build
```

期待: ビルドが成功し `website/site/` が更新される。

**Step 6: コミット**

```bash
git add mise.toml
git commit -m "chore(mise): add docs:install/serve/build tasks for website (fulgur-35m)"
```

---

## Task 8: website/README.md (ローカル開発手順)

**Files:**

- Create: `website/README.md`

**Step 1: README を書き込む**

`website/README.md`:

````markdown
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
````

**Step 2: コミット**

```bash
git add website/README.md
git commit -m "docs(website): add local development README (fulgur-35m)"
```

---

## Task 9: 受け入れ基準フル検証

**Files:** (検証のみ、変更なし)

**Step 1: docs:install 確認**

```bash
mise run docs:install
```

期待: 成功。

**Step 2: docs:build --strict 確認**

```bash
mise run docs:build
```

期待: 警告/エラーなし。

**Step 3: 言語切替・トグル確認 (手動)**

```bash
mise run docs:serve &
SERVE_PID=$!
sleep 3
curl -s http://127.0.0.1:8000/ | grep -E '(palette|toggle|deep purple|amber)' | head -5
curl -s http://127.0.0.1:8000/ja/ | head -20
kill $SERVE_PID
```

期待: 英語版ルート `/` と日本語版 `/ja/` の両方が 200 で応答する。

注: ヘッドレス環境ではブラウザの実視認確認はできないため、HTML レスポンスの存在で代替する。

**Step 4: Rust に影響がないことを確認**

```bash
cargo check --workspace --all-targets
```

期待: 成功。`cargo build` は時間がかかるので check で代替。

**Step 5: cargo fmt --check と clippy**

```bash
cargo fmt --check
```

期待: 差分なし。

clippy はデフォルトで実行時間が長いので、CI に任せる。

**Step 6: markdown lint**

```bash
npx markdownlint-cli2 'website/**/*.md' 'docs/plans/2026-04-20-fulgur-35m-mkdocs-setup.md'
```

期待: 警告なし。

**Step 7: .gitignore 検証**

```bash
git check-ignore website/site website/.venv
```

期待: 両方とも標準出力に表示される (= ignore されている)。

**Step 8: uv.lock コミット確認**

```bash
git ls-files website/uv.lock
```

期待: 出力に `website/uv.lock` が含まれる。

---

## Task 10: 最終クリーンアップとプラン commit

**Files:**

- Add: `docs/plans/2026-04-20-fulgur-35m-mkdocs-setup.md` (このプラン自体)

**Step 1: 計画ドキュメントをコミット**

```bash
git add docs/plans/2026-04-20-fulgur-35m-mkdocs-setup.md
git commit -m "docs(plan): MkDocs Material setup implementation plan (fulgur-35m)"
```

注: 通常プランは worktree 開始時に commit するが、impl flow では design は beads に保存済みのため、プラン commit は最後にまとめても可。

**Step 2: ブランチの全コミット確認**

```bash
git log --oneline main..HEAD
```

期待: 8〜10件のコミットが綺麗に並んでいる。

---

## Common Pitfalls

- `uv sync` がプロキシ環境で詰まる場合 → `UV_INDEX_URL` を確認
- `mkdocs build --strict` が `nav` の警告で落ちる場合 → 該当ページを `nav:` に追加するか、`docs/` に存在させる
- `pymdownx.superfences` の Mermaid 設定で YAML パースエラー → `!!python/name:` の前後インデントとコロンを正しく
- `mkdocs-static-i18n` の `default: true` は plugin v1.2+ の構文。それ以前は `default_language` の指定方法が違う
- mise が `python` を fetch できない場合 → `mise install python@3.12` を手動実行
- markdownlint で MD046 (code-block-style) 警告が出たら、fence (` ``` `) で統一する

## Out of Scope

このプランで **やらない** こと (別 issue 担当):

- ランディングページ本文 → `fulgur-cna`
- GitHub Actions deploy / Pages 設定 → `fulgur-bxw`
- DNS 設定 → `fulgur-92q`
- 各 Phase B ページ本文 → `fulgur-lgr` `fulgur-qef` `fulgur-e9k` `fulgur-ljj` `fulgur-aax`
- 本ロゴデザイン → 後日、独立タスク化
