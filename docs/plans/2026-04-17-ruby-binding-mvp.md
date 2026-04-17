# Ruby Binding MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** `fulgur` gem (Ruby binding) を Magnus で実装し、`Fulgur::Engine` / `Fulgur::AssetBundle` / `Fulgur::Pdf` を Ruby から呼び出せるようにする。v0.5.0 初版として RubyGems に precompiled gem を出す土台を整える（publish 自体は fulgur-qyf で別対応）。

**Architecture:** `crates/fulgur-ruby/` を cdylib として `magnus` 0.7 + `rb-sys` でビルド。`fulgur` crate の `Engine` / `AssetBundle` / `PageSize` / `Margin` を Ruby TypedData で wrap。レンダリング結果は `Fulgur::Pdf`（Rust 側 `Vec<u8>` 保持、lazy conversion）として返す。`render_html` は GVL 解放（`rb_sys::rb_thread_call_without_gvl` 経由）。テストは RSpec。

**Tech Stack:** Rust (magnus 0.7, rb-sys 0.9, base64 0.22, fulgur path dep)、Ruby 3.3+、RSpec、rake-compiler（cross build は fulgur-qyf）。

**Reference:** `crates/pyfulgur/` が PyO3 版の MVP。API 構造・エラーマッピング・GIL 解放の考え方はそのまま対応させる。差分は `docs/plans/2026-04-17-ruby-binding-mvp.md` の design セクション (beads fulgur-0x0) を参照。

**Worktree:** `/home/ubuntu/fulgur/.worktrees/ruby-binding-mvp` (branch `feature/ruby-binding-mvp`).

---

## Task 1: Crate スケルトン（Cargo + Ruby gem メタ）

**Files:**

- Create: `crates/fulgur-ruby/Cargo.toml`
- Create: `crates/fulgur-ruby/fulgur.gemspec`
- Create: `crates/fulgur-ruby/Gemfile`
- Create: `crates/fulgur-ruby/Rakefile`
- Create: `crates/fulgur-ruby/lib/fulgur.rb`
- Create: `crates/fulgur-ruby/lib/fulgur/version.rb`
- Create: `crates/fulgur-ruby/ext/fulgur/extconf.rb`
- Create: `crates/fulgur-ruby/ext/fulgur/Cargo.toml` (symlink/stub to cdylib target)
- Create: `crates/fulgur-ruby/src/lib.rs` （minimal `#[magnus::init]`）
- Modify: `Cargo.toml` (workspace members に `crates/fulgur-ruby` を追加)

**Step 1: `crates/fulgur-ruby/Cargo.toml`**

```toml
[package]
name = "fulgur-ruby"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Ruby bindings for fulgur — offline HTML/CSS to PDF conversion"
publish = false

[lib]
name = "fulgur"                  # lib<name>.so → Ruby 側 require "fulgur/fulgur"
crate-type = ["cdylib", "rlib"]

[features]
# ruby-api を有効化すると magnus/rb-sys を引き込み、Ruby 拡張としてビルドする。
# rake compile 経由でのみ有効化し、cargo build --workspace では off にして
# workspace テストを壊さない。pyfulgur の extension-module feature と同じ思想。
ruby-api = ["dep:magnus", "dep:rb-sys"]

[dependencies]
fulgur = { path = "../fulgur" }
base64 = "0.22"
magnus = { version = "0.7", optional = true }
rb-sys = { version = "0.9", optional = true }

[dev-dependencies]
static_assertions = "1.1"
```

**Step 2: workspace に追加**

`Cargo.toml` (workspace root) の `members` に `"crates/fulgur-ruby"` を追加。

**Step 3: `crates/fulgur-ruby/fulgur.gemspec`**

```ruby
# frozen_string_literal: true

require_relative "lib/fulgur/version"

Gem::Specification.new do |spec|
  spec.name = "fulgur"
  spec.version = Fulgur::VERSION
  spec.authors = ["Mitsuru Hayasaka"]
  spec.email = ["hayasaka.mitsuru@gmail.com"]

  spec.summary = "Offline HTML/CSS → PDF conversion"
  spec.description = "Ruby bindings for fulgur, a deterministic HTML/CSS to PDF rendering engine."
  spec.homepage = "https://github.com/mitsuru/fulgur"
  spec.license = "Apache-2.0"
  spec.required_ruby_version = ">= 3.3.0"

  spec.metadata["allowed_push_host"] = "https://rubygems.org"
  spec.metadata["source_code_uri"] = "https://github.com/mitsuru/fulgur"

  spec.files = Dir["lib/**/*.rb", "ext/**/*.{rs,toml,rb}", "Cargo.toml", "README.md"]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/fulgur/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9"
end
```

**Step 4: `crates/fulgur-ruby/Gemfile`**

```ruby
# frozen_string_literal: true

source "https://rubygems.org"
gemspec

gem "rake", "~> 13.0"
gem "rake-compiler", "~> 1.2"
gem "rspec", "~> 3.12"
```

**Step 5: `crates/fulgur-ruby/Rakefile`**

```ruby
# frozen_string_literal: true

require "bundler/gem_tasks"
require "rake/extensiontask"
require "rspec/core/rake_task"

RSpec::Core::RakeTask.new(:spec)

Rake::ExtensionTask.new("fulgur") do |ext|
  ext.lib_dir = "lib/fulgur"
  ext.source_pattern = "*.{rs,toml}"
end

task default: %i[compile spec]
```

**Step 6: `crates/fulgur-ruby/ext/fulgur/extconf.rb`**

```ruby
# frozen_string_literal: true

require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("fulgur/fulgur") do |r|
  r.features = ["ruby-api"]
end
```

**Step 7: `crates/fulgur-ruby/ext/fulgur/Cargo.toml`**

`rb_sys::mkmf` はこのファイルを見て cargo ビルドを呼ぶので、単純に親の `../../Cargo.toml` を指すワークスペース外 pkg にする。最小構成:

```toml
[workspace]

[package]
name = "fulgur"
version = "0.0.1"
edition = "2021"

[lib]
name = "fulgur"
crate-type = ["cdylib"]
path = "../../src/lib.rs"

[features]
default = ["ruby-api"]
ruby-api = ["magnus", "rb-sys"]

[dependencies]
fulgur = { path = "../../../fulgur" }
base64 = "0.22"
magnus = { version = "0.7", optional = true }
rb-sys = { version = "0.9", optional = true }
```

この二重 Cargo.toml は rb_sys 慣習。外側 `crates/fulgur-ruby/Cargo.toml` が workspace メンバーとしての build gate、内側 `ext/fulgur/Cargo.toml` が `rake compile` 時の gem-local ビルド。

**Step 8: `crates/fulgur-ruby/lib/fulgur/version.rb`**

```ruby
# frozen_string_literal: true

module Fulgur
  VERSION = "0.0.1"
end
```

**Step 9: `crates/fulgur-ruby/lib/fulgur.rb`**

```ruby
# frozen_string_literal: true

require_relative "fulgur/version"

begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require_relative "fulgur/#{Regexp.last_match(1)}/fulgur"
rescue LoadError
  require_relative "fulgur/fulgur"
end

module Fulgur
  class Error < StandardError; end
  class RenderError < Error; end
  class AssetError < Error; end
end
```

**Step 10: `crates/fulgur-ruby/src/lib.rs` (minimal)**

```rust
//! Ruby bindings for fulgur (HTML/CSS → PDF).
//!
//! すべての magnus 依存コードは `ruby-api` feature で gate している。
//! feature off の場合このクレートは空になり、`cargo build --workspace` が通る。
//! 実バイナリは `rake compile` が `features = ["ruby-api"]` を注入してビルドする。

#![cfg(feature = "ruby-api")]

use magnus::{define_module, prelude::*, Error};

/// fulgur 公開型が Send + Sync であることを compile time に保証する。
#[cfg(test)]
mod assertions {
    use static_assertions::assert_impl_all;
    assert_impl_all!(fulgur::Engine: Send, Sync);
    assert_impl_all!(fulgur::AssetBundle: Send, Sync);
}

#[magnus::init]
fn init(_ruby: &magnus::Ruby) -> Result<(), Error> {
    let _fulgur = define_module("Fulgur")?;
    Ok(())
}
```

**Step 11: ビルド確認**

```bash
cd crates/fulgur-ruby
bundle install
bundle exec rake compile
```

Expected: `lib/fulgur/fulgur.so` (Linux) が生成される。`require "fulgur"` で import できる。

**Step 12: Commit**

```bash
git add Cargo.toml crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): scaffold gem + crate skeleton"
```

---

## Task 2: Error モジュール（fulgur::Error → Ruby 例外マッピング）

**Files:**

- Create: `crates/fulgur-ruby/src/error.rs`
- Modify: `crates/fulgur-ruby/src/lib.rs` (mod error + register)
- Create: `crates/fulgur-ruby/spec/spec_helper.rb`
- Create: `crates/fulgur-ruby/spec/error_spec.rb`

**Step 1: `spec/spec_helper.rb`**

```ruby
# frozen_string_literal: true

require "fulgur"
require "fileutils"
require "tmpdir"

RSpec.configure do |config|
  config.example_status_persistence_file_path = ".rspec_status"
  config.disable_monkey_patching!
  config.expect_with :rspec do |c|
    c.syntax = :expect
  end
end
```

**Step 2: Write failing test — `spec/error_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"

RSpec.describe "Fulgur error hierarchy" do
  it "defines Fulgur::Error as a StandardError" do
    expect(Fulgur::Error.ancestors).to include(StandardError)
  end

  it "defines Fulgur::RenderError under Fulgur::Error" do
    expect(Fulgur::RenderError.ancestors).to include(Fulgur::Error)
  end

  it "defines Fulgur::AssetError under Fulgur::Error" do
    expect(Fulgur::AssetError.ancestors).to include(Fulgur::Error)
  end
end
```

**Step 3: Run to fail**

```bash
cd crates/fulgur-ruby && bundle exec rspec spec/error_spec.rb
```

Expected: ... 全部パス（lib/fulgur.rb で既に定義済み）。OK なら Rust 側 `map_fulgur_error` を実装してエラー挙動テストをカバーする。

**Step 4: `crates/fulgur-ruby/src/error.rs`**

```rust
use fulgur::Error as FulgurError;
use magnus::{exception, Error, ExceptionClass, Ruby};

/// Ruby 側の Fulgur::Error / Fulgur::RenderError / Fulgur::AssetError クラスを取得する。
/// lib/fulgur.rb で既に定義済みなので、ここでは lookup のみ行う。
fn class<'a>(ruby: &'a Ruby, name: &str) -> Result<ExceptionClass, Error> {
    let fulgur = ruby.class_object().const_get::<_, magnus::RModule>("Fulgur")?;
    fulgur.const_get::<_, ExceptionClass>(name)
}

pub fn map_fulgur_error(ruby: &Ruby, err: FulgurError) -> Error {
    match err {
        FulgurError::Io(io_err) => match io_err.kind() {
            std::io::ErrorKind::NotFound => {
                // Ruby 標準 Errno::ENOENT を使う (File.open と同じ挙動)
                let errno = ruby
                    .class_object()
                    .const_get::<_, magnus::RModule>("Errno")
                    .and_then(|m| m.const_get::<_, ExceptionClass>("ENOENT"))
                    .unwrap_or_else(|_| exception::standard_error());
                Error::new(errno, io_err.to_string())
            }
            _ => render_error(ruby, io_err.to_string()),
        },
        FulgurError::Asset(msg) | FulgurError::UnsupportedFontFormat(msg) => {
            asset_error(ruby, msg)
        }
        FulgurError::WoffDecode(msg)
        | FulgurError::HtmlParse(msg)
        | FulgurError::Layout(msg)
        | FulgurError::PdfGeneration(msg)
        | FulgurError::Template(msg) => render_error(ruby, msg),
    }
}

fn render_error(ruby: &Ruby, msg: String) -> Error {
    class(ruby, "RenderError")
        .map(|c| Error::new(c, msg.clone()))
        .unwrap_or_else(|_| Error::new(exception::runtime_error(), msg))
}

fn asset_error(ruby: &Ruby, msg: String) -> Error {
    class(ruby, "AssetError")
        .map(|c| Error::new(c, msg.clone()))
        .unwrap_or_else(|_| Error::new(exception::runtime_error(), msg))
}
```

**Step 5: `crates/fulgur-ruby/src/lib.rs` に `mod error` を追加**

```rust
mod error;
```

**Step 6: Rebuild and run**

```bash
bundle exec rake compile
bundle exec rspec spec/error_spec.rb
```

Expected: PASS（3 examples）。

**Step 7: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add error mapping (Fulgur::{Error,RenderError,AssetError} + Errno::ENOENT)"
```

---

## Task 3: PageSize wrapper（symbol/string/class 全受け入れ）

**Files:**

- Create: `crates/fulgur-ruby/src/page_size.rs`
- Modify: `crates/fulgur-ruby/src/lib.rs`
- Create: `crates/fulgur-ruby/spec/page_size_spec.rb`

**Step 1: Failing tests — `spec/page_size_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"

RSpec.describe Fulgur::PageSize do
  describe "constants" do
    it "exposes A4" do
      expect(described_class::A4.width).to be_within(0.1).of(595.28)
      expect(described_class::A4.height).to be_within(0.1).of(841.89)
    end

    it "exposes Letter" do
      expect(described_class::LETTER.width).to be_within(0.1).of(612.0)
    end

    it "exposes A3" do
      expect(described_class::A3).not_to be_nil
    end
  end

  describe ".custom" do
    it "accepts width/height in mm" do
      ps = described_class.custom(100, 200)
      # mm → pt: 100mm ≈ 283.46pt
      expect(ps.width).to be_within(0.1).of(283.46)
    end
  end

  describe "#landscape" do
    it "returns a rotated PageSize" do
      a4 = described_class::A4
      land = a4.landscape
      expect(land.width).to be_within(0.1).of(a4.height)
      expect(land.height).to be_within(0.1).of(a4.width)
    end
  end
end
```

**Step 2: Run — fail (PageSize undefined)**

```bash
bundle exec rspec spec/page_size_spec.rb
```

Expected: FAIL — uninitialized constant Fulgur::PageSize.

**Step 3: `crates/fulgur-ruby/src/page_size.rs`**

```rust
use fulgur::PageSize;
use magnus::{
    class, function, method,
    prelude::*,
    Error, RModule, Ruby, Symbol, Value,
};

#[magnus::wrap(class = "Fulgur::PageSize", free_immediately, size)]
#[derive(Clone, Copy)]
pub struct RbPageSize {
    pub(crate) inner: PageSize,
}

impl RbPageSize {
    pub(crate) fn new(inner: PageSize) -> Self {
        Self { inner }
    }

    fn width(&self) -> f32 {
        self.inner.width
    }

    fn height(&self) -> f32 {
        self.inner.height
    }

    fn landscape(&self) -> Self {
        Self::new(self.inner.landscape())
    }

    fn inspect(&self) -> String {
        format!(
            "#<Fulgur::PageSize width={:.2} height={:.2}>",
            self.inner.width, self.inner.height
        )
    }

    fn custom(width_mm: f32, height_mm: f32) -> Self {
        Self::new(PageSize::custom(width_mm, height_mm))
    }
}

/// `Symbol` / `String` / `Fulgur::PageSize` いずれも受けて `fulgur::PageSize` に変換する。
pub fn extract(value: Value) -> Result<PageSize, Error> {
    if let Ok(ps) = <&RbPageSize>::try_convert(value) {
        return Ok(ps.inner);
    }
    if let Ok(sym) = Symbol::try_convert(value) {
        return parse_name(&sym.name()?);
    }
    if let Ok(s) = String::try_convert(value) {
        return parse_name(&s);
    }
    Err(Error::new(
        magnus::exception::arg_error(),
        "page_size must be Symbol, String, or Fulgur::PageSize",
    ))
}

fn parse_name(name: &str) -> Result<PageSize, Error> {
    match name.to_ascii_uppercase().as_str() {
        "A4" => Ok(PageSize::A4),
        "LETTER" => Ok(PageSize::LETTER),
        "A3" => Ok(PageSize::A3),
        other => Err(Error::new(
            magnus::exception::arg_error(),
            format!("unknown page size: {other}"),
        )),
    }
}

pub fn define(ruby: &Ruby, fulgur: &RModule) -> Result<(), Error> {
    let class = fulgur.define_class("PageSize", ruby.class_object())?;
    class.define_singleton_method("custom", function!(RbPageSize::custom, 2))?;
    class.define_method("width", method!(RbPageSize::width, 0))?;
    class.define_method("height", method!(RbPageSize::height, 0))?;
    class.define_method("landscape", method!(RbPageSize::landscape, 0))?;
    class.define_method("inspect", method!(RbPageSize::inspect, 0))?;
    class.define_method("to_s", method!(RbPageSize::inspect, 0))?;

    // 定数 A4 / LETTER / A3 を登録
    class.const_set("A4", RbPageSize::new(PageSize::A4))?;
    class.const_set("LETTER", RbPageSize::new(PageSize::LETTER))?;
    class.const_set("A3", RbPageSize::new(PageSize::A3))?;
    Ok(())
}
```

**Step 4: `src/lib.rs` に統合**

```rust
mod page_size;
...
#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), Error> {
    let fulgur = define_module("Fulgur")?;
    page_size::define(ruby, &fulgur)?;
    Ok(())
}
```

**Step 5: Rebuild + run**

```bash
bundle exec rake compile
bundle exec rspec spec/page_size_spec.rb
```

Expected: PASS (8+ examples).

**Step 6: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add PageSize wrapper (A4/LETTER/A3 + custom + landscape)"
```

---

## Task 4: Margin wrapper（CSS流 positional + kwargs）

**Files:**

- Create: `crates/fulgur-ruby/src/margin.rs`
- Modify: `crates/fulgur-ruby/src/lib.rs`
- Create: `crates/fulgur-ruby/lib/fulgur/margin.rb` （Ruby 側で positional/kwargs を解釈して Rust に渡す）
- Create: `crates/fulgur-ruby/spec/margin_spec.rb`

**Step 1: Failing tests — `spec/margin_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"

RSpec.describe Fulgur::Margin do
  describe ".new" do
    it "accepts 1 positional (uniform)" do
      m = described_class.new(72)
      expect([m.top, m.right, m.bottom, m.left]).to eq([72.0, 72.0, 72.0, 72.0])
    end

    it "accepts 2 positional (symmetric v, h)" do
      m = described_class.new(72, 36)
      expect([m.top, m.right, m.bottom, m.left]).to eq([72.0, 36.0, 72.0, 36.0])
    end

    it "accepts 4 positional (CSS t, r, b, l)" do
      m = described_class.new(72, 36, 48, 24)
      expect([m.top, m.right, m.bottom, m.left]).to eq([72.0, 36.0, 48.0, 24.0])
    end

    it "accepts kwargs" do
      m = described_class.new(top: 72, right: 36, bottom: 48, left: 24)
      expect([m.top, m.right, m.bottom, m.left]).to eq([72.0, 36.0, 48.0, 24.0])
    end

    it "raises ArgumentError for 3 positional" do
      expect { described_class.new(1, 2, 3) }.to raise_error(ArgumentError)
    end
  end

  describe ".uniform / .symmetric" do
    it ".uniform(pt) — all sides equal" do
      m = described_class.uniform(50)
      expect(m.top).to eq(50.0)
      expect(m.left).to eq(50.0)
    end

    it ".symmetric(v, h)" do
      m = described_class.symmetric(72, 36)
      expect(m.top).to eq(72.0)
      expect(m.right).to eq(36.0)
    end
  end
end
```

**Step 2: `crates/fulgur-ruby/src/margin.rs`**

```rust
use fulgur::Margin;
use magnus::{class, function, method, prelude::*, Error, RModule, Ruby};

#[magnus::wrap(class = "Fulgur::Margin", free_immediately, size)]
#[derive(Clone, Copy)]
pub struct RbMargin {
    pub(crate) inner: Margin,
}

impl RbMargin {
    pub(crate) fn new(inner: Margin) -> Self {
        Self { inner }
    }

    fn top(&self) -> f32 { self.inner.top }
    fn right(&self) -> f32 { self.inner.right }
    fn bottom(&self) -> f32 { self.inner.bottom }
    fn left(&self) -> f32 { self.inner.left }

    fn inspect(&self) -> String {
        format!(
            "#<Fulgur::Margin top={:.2} right={:.2} bottom={:.2} left={:.2}>",
            self.inner.top, self.inner.right, self.inner.bottom, self.inner.left
        )
    }

    fn uniform(pt: f32) -> Self {
        Self::new(Margin::uniform(pt))
    }

    fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self::new(Margin::symmetric(vertical, horizontal))
    }

    /// Rust 側 primitive constructor (Ruby 側ヘルパーから呼ぶ用)
    fn from_trbl(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self::new(Margin { top, right, bottom, left })
    }
}

pub fn define(_ruby: &Ruby, fulgur: &RModule) -> Result<(), Error> {
    let class = fulgur.define_class("Margin", class::object())?;
    // Ruby 側で positional/kwargs を解釈して __build__ を呼ぶ構造にする
    class.define_singleton_method("__build__", function!(RbMargin::from_trbl, 4))?;
    class.define_singleton_method("uniform", function!(RbMargin::uniform, 1))?;
    class.define_singleton_method("symmetric", function!(RbMargin::symmetric, 2))?;
    class.define_method("top", method!(RbMargin::top, 0))?;
    class.define_method("right", method!(RbMargin::right, 0))?;
    class.define_method("bottom", method!(RbMargin::bottom, 0))?;
    class.define_method("left", method!(RbMargin::left, 0))?;
    class.define_method("inspect", method!(RbMargin::inspect, 0))?;
    class.define_method("to_s", method!(RbMargin::inspect, 0))?;
    Ok(())
}
```

**Step 3: `crates/fulgur-ruby/lib/fulgur/margin.rb`**

```ruby
# frozen_string_literal: true

module Fulgur
  class Margin
    class << self
      alias_method :__native_new__, :new

      def new(*args, **kwargs)
        if !kwargs.empty?
          raise ArgumentError, "positional and kwargs are mutually exclusive" unless args.empty?
          required = %i[top right bottom left]
          missing = required - kwargs.keys
          raise ArgumentError, "missing keys: #{missing.join(", ")}" unless missing.empty?
          t, r, b, l = kwargs.values_at(*required).map(&:to_f)
          return __build__(t, r, b, l)
        end

        case args.length
        when 1
          v = args[0].to_f
          __build__(v, v, v, v)
        when 2
          vv, hh = args.map(&:to_f)
          __build__(vv, hh, vv, hh)
        when 4
          t, r, b, l = args.map(&:to_f)
          __build__(t, r, b, l)
        else
          raise ArgumentError, "wrong number of arguments (#{args.length} for 1, 2, 4, or kwargs)"
        end
      end
    end
  end
end
```

**Step 4: `lib/fulgur.rb` に `require_relative "fulgur/margin"` を追加**

**Step 5: Rebuild + test**

```bash
bundle exec rake compile
bundle exec rspec spec/margin_spec.rb
```

Expected: PASS.

**Step 6: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add Margin wrapper (positional + kwargs + factory)"
```

---

## Task 5: AssetBundle wrapper（long + short alias）

**Files:**

- Create: `crates/fulgur-ruby/src/asset_bundle.rs`
- Create: `crates/fulgur-ruby/lib/fulgur/asset_bundle.rb`
- Modify: `crates/fulgur-ruby/src/lib.rs`
- Create: `crates/fulgur-ruby/spec/asset_bundle_spec.rb`
- Create: `crates/fulgur-ruby/spec/fixtures/style.css` (single line `body { color: red }`)
- Create: `crates/fulgur-ruby/spec/fixtures/noto_sans.ttf` (copy from examples/.fonts/NotoSansJP-Regular.ttf)

**Step 1: Failing tests — `spec/asset_bundle_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"

RSpec.describe Fulgur::AssetBundle do
  let(:bundle) { described_class.new }
  let(:fixtures) { File.expand_path("fixtures", __dir__) }

  describe "#add_css / #css" do
    it "accepts inline CSS via long name" do
      bundle.add_css("body { color: red }")
      # 成功すれば例外を投げない
    end

    it "has #css alias" do
      expect(bundle.method(:css)).to eq(bundle.method(:add_css))
    end
  end

  describe "#add_css_file / #css_file" do
    it "reads CSS file" do
      bundle.add_css_file(File.join(fixtures, "style.css"))
    end

    it "raises Errno::ENOENT for missing file" do
      expect { bundle.add_css_file("/nonexistent.css") }.to raise_error(Errno::ENOENT)
    end

    it "has #css_file alias" do
      expect(bundle.method(:css_file)).to eq(bundle.method(:add_css_file))
    end
  end

  describe "#add_font_file / #font_file" do
    it "reads a .ttf font" do
      bundle.add_font_file(File.join(fixtures, "noto_sans.ttf"))
    end

    it "raises Errno::ENOENT for missing font" do
      expect { bundle.add_font_file("/nope.ttf") }.to raise_error(Errno::ENOENT)
    end
  end

  describe "#add_image / #image" do
    it "registers image bytes with name" do
      bundle.add_image("logo", "\x89PNG\r\n\x1a\n".b)
    end
  end
end
```

**Step 2: `crates/fulgur-ruby/src/asset_bundle.rs`**

```rust
use crate::error::map_fulgur_error;
use fulgur::AssetBundle;
use magnus::{function, method, prelude::*, Error, RModule, RString, Ruby};
use std::path::PathBuf;

#[magnus::wrap(class = "Fulgur::AssetBundle", free_immediately, size)]
pub struct RbAssetBundle {
    // RefCell: take_inner で所有権を取り出す必要がある + Magnus wrap は
    // Sync が要らないため内部可変でOK。Ruby は GVL 下で single-threaded。
    pub(crate) inner: std::cell::RefCell<AssetBundle>,
}

impl RbAssetBundle {
    pub(crate) fn new() -> Self {
        Self { inner: std::cell::RefCell::new(AssetBundle::new()) }
    }

    /// Engine builder に渡す際に中身を奪う。奪った後は empty AssetBundle が残る。
    pub(crate) fn take_inner(&self) -> AssetBundle {
        std::mem::replace(&mut *self.inner.borrow_mut(), AssetBundle::new())
    }

    fn add_css(&self, css: String) {
        self.inner.borrow_mut().add_css(css);
    }

    fn add_css_file(&self, path: String) -> Result<(), Error> {
        let ruby = Ruby::get().expect("ruby vm");
        self.inner
            .borrow_mut()
            .add_css_file(PathBuf::from(path))
            .map_err(|e| map_fulgur_error(&ruby, e))
    }

    fn add_font_file(&self, path: String) -> Result<(), Error> {
        let ruby = Ruby::get().expect("ruby vm");
        self.inner
            .borrow_mut()
            .add_font_file(PathBuf::from(path))
            .map_err(|e| map_fulgur_error(&ruby, e))
    }

    fn add_image(&self, name: String, data: RString) {
        let bytes = unsafe { data.as_slice() }.to_vec();
        self.inner.borrow_mut().add_image(name, bytes);
    }

    fn add_image_file(&self, name: String, path: String) -> Result<(), Error> {
        let ruby = Ruby::get().expect("ruby vm");
        self.inner
            .borrow_mut()
            .add_image_file(name, PathBuf::from(path))
            .map_err(|e| map_fulgur_error(&ruby, e))
    }
}

pub fn define(_ruby: &Ruby, fulgur: &RModule) -> Result<(), Error> {
    let class = fulgur.define_class("AssetBundle", magnus::class::object())?;
    class.define_singleton_method("new", function!(RbAssetBundle::new, 0))?;
    class.define_method("add_css", method!(RbAssetBundle::add_css, 1))?;
    class.define_method("add_css_file", method!(RbAssetBundle::add_css_file, 1))?;
    class.define_method("add_font_file", method!(RbAssetBundle::add_font_file, 1))?;
    class.define_method("add_image", method!(RbAssetBundle::add_image, 2))?;
    class.define_method("add_image_file", method!(RbAssetBundle::add_image_file, 2))?;
    Ok(())
}
```

**Step 3: `crates/fulgur-ruby/lib/fulgur/asset_bundle.rb`**

```ruby
# frozen_string_literal: true

module Fulgur
  class AssetBundle
    alias_method :css, :add_css
    alias_method :css_file, :add_css_file
    alias_method :font_file, :add_font_file
    alias_method :image, :add_image
    alias_method :image_file, :add_image_file
  end
end
```

**Step 4: `lib/fulgur.rb` に require_relative 追加**

**Step 5: fixtures 準備**

```bash
echo 'body { color: red }' > crates/fulgur-ruby/spec/fixtures/style.css
cp examples/.fonts/NotoSansJP-Regular.ttf crates/fulgur-ruby/spec/fixtures/noto_sans.ttf
```

**Step 6: Rebuild + test**

```bash
bundle exec rake compile
bundle exec rspec spec/asset_bundle_spec.rb
```

**Step 7: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add AssetBundle wrapper + long/short aliases"
```

---

## Task 6: Engine + EngineBuilder（まず最小、kwargs と builder chain）

**Files:**

- Create: `crates/fulgur-ruby/src/engine.rs`
- Modify: `crates/fulgur-ruby/src/lib.rs`
- Create: `crates/fulgur-ruby/spec/engine_spec.rb`
- Create: `crates/fulgur-ruby/spec/fixtures/simple.html`

**Step 1: Failing tests — `spec/engine_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"

RSpec.describe Fulgur::Engine do
  let(:html) { File.read(File.expand_path("fixtures/simple.html", __dir__)) }

  describe ".new" do
    it "accepts no kwargs" do
      expect { described_class.new }.not_to raise_error
    end

    it "accepts page_size as string" do
      described_class.new(page_size: "A4")
    end

    it "accepts page_size as symbol" do
      described_class.new(page_size: :a4)
    end

    it "accepts page_size as PageSize constant" do
      described_class.new(page_size: Fulgur::PageSize::A4)
    end

    it "raises ArgumentError for unknown page_size string" do
      expect { described_class.new(page_size: "XYZ") }.to raise_error(ArgumentError)
    end
  end

  describe ".builder" do
    it "returns an EngineBuilder" do
      expect(described_class.builder).to be_a(Fulgur::EngineBuilder)
    end

    it "builds an Engine via chain" do
      engine = described_class.builder.page_size(:a4).build
      expect(engine).to be_a(described_class)
    end

    it "raises RuntimeError on double build" do
      b = described_class.builder
      b.build
      expect { b.build }.to raise_error(/already been built/)
    end
  end
end
```

**Step 2: simple.html fixture**

```html
<!DOCTYPE html>
<html><body><h1>Hello</h1><p>Ruby binding test</p></body></html>
```

**Step 3: `crates/fulgur-ruby/src/engine.rs`**

```rust
use crate::asset_bundle::RbAssetBundle;
use crate::error::map_fulgur_error;
use crate::margin::RbMargin;
use crate::page_size::extract as extract_page_size;
use fulgur::{Engine, EngineBuilder};
use magnus::{
    class, function, kwargs, method,
    prelude::*,
    scan_args::{get_kwargs, scan_args},
    Error, RModule, Ruby, Value,
};
use std::cell::RefCell;

#[magnus::wrap(class = "Fulgur::EngineBuilder", free_immediately, size)]
pub struct RbEngineBuilder {
    inner: RefCell<Option<EngineBuilder>>,
}

impl RbEngineBuilder {
    fn new() -> Self {
        Self { inner: RefCell::new(Some(Engine::builder())) }
    }

    fn take(&self) -> Result<EngineBuilder, Error> {
        self.inner
            .borrow_mut()
            .take()
            .ok_or_else(|| Error::new(magnus::exception::runtime_error(), "EngineBuilder has already been built"))
    }

    fn map(&self, f: impl FnOnce(EngineBuilder) -> EngineBuilder) -> Result<(), Error> {
        let b = self.take()?;
        *self.inner.borrow_mut() = Some(f(b));
        Ok(())
    }
}

// chain API: 各 setter は self を返す（Ruby 側は同じインスタンスを返す）
fn builder_page_size(b: magnus::Obj<RbEngineBuilder>, value: Value) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    let ps = extract_page_size(value)?;
    b.map(|inner| inner.page_size(ps))?;
    Ok(b)
}

fn builder_margin(b: magnus::Obj<RbEngineBuilder>, m: &RbMargin) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    b.map(|inner| inner.margin(m.inner))?;
    Ok(b)
}

fn builder_assets(b: magnus::Obj<RbEngineBuilder>, bundle: &RbAssetBundle) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    let taken = bundle.take_inner();
    b.map(|inner| inner.assets(taken))?;
    Ok(b)
}

fn builder_landscape(b: magnus::Obj<RbEngineBuilder>, v: bool) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    b.map(|inner| inner.landscape(v))?;
    Ok(b)
}

fn builder_title(b: magnus::Obj<RbEngineBuilder>, s: String) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    b.map(|inner| inner.title(s))?;
    Ok(b)
}

fn builder_author(b: magnus::Obj<RbEngineBuilder>, s: String) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    b.map(|inner| inner.author(s))?;
    Ok(b)
}

fn builder_lang(b: magnus::Obj<RbEngineBuilder>, s: String) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    b.map(|inner| inner.lang(s))?;
    Ok(b)
}

fn builder_bookmarks(b: magnus::Obj<RbEngineBuilder>, v: bool) -> Result<magnus::Obj<RbEngineBuilder>, Error> {
    b.map(|inner| inner.bookmarks(v))?;
    Ok(b)
}

fn builder_build(b: magnus::Obj<RbEngineBuilder>) -> Result<RbEngine, Error> {
    let built = b.take()?;
    Ok(RbEngine { inner: built.build() })
}

#[magnus::wrap(class = "Fulgur::Engine", free_immediately, size)]
pub struct RbEngine {
    pub(crate) inner: Engine,
}

/// kwargs-only constructor。positional は受け付けない。
fn engine_new(args: &[Value]) -> Result<RbEngine, Error> {
    // positional 0 個, kwargs オプショナル
    let scanned = scan_args::<(), (), (), (), _, ()>(args)?;
    let kw = get_kwargs::<
        _,
        (),
        (Option<Value>, Option<&RbMargin>, Option<bool>, Option<String>, Option<String>, Option<String>, Option<bool>, Option<&RbAssetBundle>),
        (),
    >(
        scanned.keywords,
        &[],
        &["page_size", "margin", "landscape", "title", "author", "lang", "bookmarks", "assets"],
    )?;
    let (page_size, margin, landscape, title, author, lang, bookmarks, assets) = kw.optional;

    let mut b = Engine::builder();
    if let Some(v) = page_size { b = b.page_size(extract_page_size(v)?); }
    if let Some(m) = margin { b = b.margin(m.inner); }
    if let Some(v) = landscape { b = b.landscape(v); }
    if let Some(s) = title { b = b.title(s); }
    if let Some(s) = author { b = b.author(s); }
    if let Some(s) = lang { b = b.lang(s); }
    if let Some(v) = bookmarks { b = b.bookmarks(v); }
    if let Some(bundle) = assets { b = b.assets(bundle.take_inner()); }
    Ok(RbEngine { inner: b.build() })
}

fn engine_builder() -> RbEngineBuilder {
    RbEngineBuilder::new()
}

pub fn define(_ruby: &Ruby, fulgur: &RModule) -> Result<(), Error> {
    let engine = fulgur.define_class("Engine", class::object())?;
    engine.define_singleton_method("new", function!(engine_new, -1))?;
    engine.define_singleton_method("builder", function!(engine_builder, 0))?;

    let builder = fulgur.define_class("EngineBuilder", class::object())?;
    builder.define_method("page_size", method!(builder_page_size, 1))?;
    builder.define_method("margin", method!(builder_margin, 1))?;
    builder.define_method("assets", method!(builder_assets, 1))?;
    builder.define_method("landscape", method!(builder_landscape, 1))?;
    builder.define_method("title", method!(builder_title, 1))?;
    builder.define_method("author", method!(builder_author, 1))?;
    builder.define_method("lang", method!(builder_lang, 1))?;
    builder.define_method("bookmarks", method!(builder_bookmarks, 1))?;
    builder.define_method("build", method!(builder_build, 0))?;
    Ok(())
}
```

**Step 4: register in lib.rs**

**Step 5: Rebuild + test**

```bash
bundle exec rake compile
bundle exec rspec spec/engine_spec.rb
```

Expected: PASS（render_html はこのタスクではまだ未実装。engine 生成・builder chain だけ検証）。

**Step 6: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add Engine + EngineBuilder (kwargs + chain)"
```

---

## Task 7: Pdf result object（to_s / bytesize / to_base64 / to_data_uri）

**Files:**

- Create: `crates/fulgur-ruby/src/pdf.rs`
- Modify: `crates/fulgur-ruby/src/lib.rs`
- Create: `crates/fulgur-ruby/spec/pdf_spec.rb`

**Step 1: Failing tests — `spec/pdf_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"
require "base64"

RSpec.describe Fulgur::Pdf do
  let(:html) { File.read(File.expand_path("fixtures/simple.html", __dir__)) }
  let(:engine) { Fulgur::Engine.new(page_size: :a4) }
  let(:pdf) { engine.render_html(html) }

  it "returns a Fulgur::Pdf" do
    expect(pdf).to be_a(described_class)
  end

  describe "#bytesize" do
    it "returns a positive integer" do
      expect(pdf.bytesize).to be > 100
    end
  end

  describe "#to_s" do
    it "returns ASCII-8BIT encoded string starting with %PDF-" do
      s = pdf.to_s
      expect(s.encoding).to eq(Encoding::ASCII_8BIT)
      expect(s[0, 5]).to eq("%PDF-".b)
    end

    it "to_s.bytesize == bytesize" do
      expect(pdf.to_s.bytesize).to eq(pdf.bytesize)
    end
  end

  describe "#to_base64" do
    it "returns a String that round-trips via Base64.strict_decode64" do
      b64 = pdf.to_base64
      expect(b64).to match(/\A[A-Za-z0-9+\/=]+\z/)
      decoded = Base64.strict_decode64(b64)
      expect(decoded.force_encoding(Encoding::ASCII_8BIT)).to eq(pdf.to_s)
    end
  end

  describe "#to_data_uri" do
    it "returns data URI with application/pdf prefix" do
      expect(pdf.to_data_uri).to start_with("data:application/pdf;base64,")
    end
  end
end
```

**Step 2: `crates/fulgur-ruby/src/pdf.rs`**

```rust
use base64::Engine as _;
use magnus::{class, method, prelude::*, Error, RModule, RString, Ruby};

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
        // from_slice は ASCII-8BIT (binary) encoding で作る
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
    let class = fulgur.define_class("Pdf", class::object())?;
    class.define_method("bytesize", method!(RbPdf::bytesize, 0))?;
    class.define_method("to_s", method!(RbPdf::to_s, 0))?;
    class.define_method("to_base64", method!(RbPdf::to_base64, 0))?;
    class.define_method("to_data_uri", method!(RbPdf::to_data_uri, 0))?;
    class.define_method("inspect", method!(RbPdf::inspect, 0))?;
    Ok(())
}
```

**Step 3: Engine.render_html (minimal, GVL 後回し) を追加**

engine.rs に追加:

```rust
use crate::pdf::RbPdf;

impl RbEngine {
    fn render_html(&self, html: String) -> Result<RbPdf, Error> {
        // GVL 解放は Task 9 で導入。ここではまず同期呼び出し。
        let ruby = Ruby::get().expect("ruby vm");
        let bytes = self.inner.render_html(&html).map_err(|e| map_fulgur_error(&ruby, e))?;
        Ok(RbPdf::new(bytes))
    }
}
```

そして define 内に:

```rust
engine.define_method("render_html", method!(RbEngine::render_html, 1))?;
```

**Step 4: Run tests**

```bash
bundle exec rake compile
bundle exec rspec spec/pdf_spec.rb
```

**Step 5: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add Pdf result object (to_s/bytesize/to_base64/to_data_uri) + render_html"
```

---

## Task 8: Pdf.write_to_path / write_to_io（chunked, binmode）

**Files:**

- Modify: `crates/fulgur-ruby/src/pdf.rs`
- Modify: `crates/fulgur-ruby/spec/pdf_spec.rb`

**Step 1: Add tests**

```ruby
describe "#write_to_path" do
  it "writes binary PDF to file" do
    Dir.mktmpdir do |dir|
      path = File.join(dir, "out.pdf")
      pdf.write_to_path(path)
      expect(File.binread(path)).to eq(pdf.to_s)
    end
  end
end

describe "#write_to_io" do
  it "writes binary PDF to StringIO, ensuring binmode" do
    io = StringIO.new
    pdf.write_to_io(io)
    expect(io.string).to eq(pdf.to_s)
  end

  it "writes in 64KB chunks (smoke test)" do
    io = StringIO.new
    pdf.write_to_io(io)
    expect(io.string.bytesize).to eq(pdf.bytesize)
  end
end
```

**Step 2: `pdf.rs` に追加**

```rust
use magnus::{value::ReprValue, Value};

const CHUNK: usize = 64 * 1024;

impl RbPdf {
    fn write_to_path(&self, path: String) -> Result<(), Error> {
        let ruby = Ruby::get().expect("ruby vm");
        std::fs::write(&path, &self.bytes).map_err(|e| {
            // ENOENT は Errno::ENOENT、それ以外は RenderError
            let fulgur_err = fulgur::Error::Io(e);
            crate::error::map_fulgur_error(&ruby, fulgur_err)
        })
    }

    fn write_to_io(&self, io: Value) -> Result<(), Error> {
        // IO#binmode を呼んで encoding 変換を防ぐ
        let _: Value = io.funcall("binmode", ())?;
        for chunk in self.bytes.chunks(CHUNK) {
            let s = RString::from_slice(chunk);
            let _: Value = io.funcall("write", (s,))?;
        }
        Ok(())
    }
}
```

define に追加:

```rust
class.define_method("write_to_path", method!(RbPdf::write_to_path, 1))?;
class.define_method("write_to_io", method!(RbPdf::write_to_io, 1))?;
```

**Step 3: require "stringio" in spec**

spec_helper.rb に `require "stringio"` を追加。

**Step 4: Rebuild + run**

```bash
bundle exec rake compile
bundle exec rspec spec/pdf_spec.rb
```

**Step 5: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add Pdf#write_to_path + #write_to_io (64KB chunked, binmode)"
```

---

## Task 9: GVL 解放（rb_thread_call_without_gvl）

**Files:**

- Modify: `crates/fulgur-ruby/src/engine.rs`
- Create: `crates/fulgur-ruby/src/gvl.rs`
- Modify: `crates/fulgur-ruby/spec/engine_spec.rb`

**Background:** magnus 0.7 は GVL 解放ヘルパを提供していないため、rb-sys で `rb_thread_call_without_gvl` を直接呼ぶ。Send + Sync 保証のため `Engine` と `html: String` のみを借用渡しに留め、結果 `Result<Vec<u8>, fulgur::Error>` をボックス経由で返す。

**Step 1: `src/gvl.rs`**

```rust
use std::ffi::c_void;

/// 与えた closure を GVL 解放状態で実行する。
///
/// 引数 `Data` は `Send` でなくてよいが、closure 内で Ruby VM / RString に触れてはならない
/// （別スレッドで実行されるため UB）。fulgur::Engine と borrow した &str を渡す想定。
pub fn without_gvl<Data, Ret>(data: Data, body: fn(Data) -> Ret) -> Ret
where
    Ret: Default,
{
    struct Payload<D, R> {
        data: Option<D>,
        body: fn(D) -> R,
        result: Option<R>,
    }

    unsafe extern "C" fn shim<D, R>(arg: *mut c_void) -> *mut c_void {
        let p = &mut *(arg as *mut Payload<D, R>);
        let data = p.data.take().unwrap();
        p.result = Some((p.body)(data));
        std::ptr::null_mut()
    }

    let mut payload: Payload<Data, Ret> = Payload {
        data: Some(data),
        body,
        result: None,
    };
    unsafe {
        rb_sys::rb_thread_call_without_gvl(
            Some(shim::<Data, Ret>),
            &mut payload as *mut _ as *mut c_void,
            None,
            std::ptr::null_mut(),
        );
    }
    payload.result.unwrap_or_default()
}
```

Note: `Ret: Default` 制約は `unwrap_or_default` を避けるため、実運用では `Option<Ret>` を Box で返す方が安全。詳細は implementation 時にレビュー。

**Step 2: engine.rs を更新**

```rust
impl RbEngine {
    fn render_html(&self, html: String) -> Result<RbPdf, Error> {
        // `self.inner` は Engine: Send + Sync なので GVL 解放中に参照してよい。
        // html は String でここで所有、GVL 外スレッドに move する。
        let engine_ref: &Engine = &self.inner;
        let result = crate::gvl::without_gvl::<(&Engine, String), Option<Result<Vec<u8>, fulgur::Error>>>(
            (engine_ref, html),
            |(e, h)| Some(e.render_html(&h)),
        );
        let ruby = Ruby::get().expect("ruby vm");
        match result {
            Some(Ok(bytes)) => Ok(RbPdf::new(bytes)),
            Some(Err(e)) => Err(map_fulgur_error(&ruby, e)),
            None => Err(Error::new(magnus::exception::runtime_error(), "render interrupted")),
        }
    }
}
```

**Step 3: Concurrency smoke test — spec**

```ruby
describe "GVL release" do
  it "allows concurrent Ruby threads during render_html" do
    engine = Fulgur::Engine.new
    html = File.read(File.expand_path("fixtures/simple.html", __dir__))

    counter = 0
    ticker = Thread.new do
      40.times { counter += 1; sleep 0.005 }
    end
    10.times { engine.render_html(html) }
    ticker.join
    # render_html 中もカウンタが進む (GVL 保持時は 0 付近で止まる)
    expect(counter).to be >= 20
  end
end
```

**Step 4: Rebuild + test**

**Step 5: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): release GVL during render_html"
```

---

## Task 10: render_html_to_file + 統合テスト

**Files:**

- Modify: `crates/fulgur-ruby/src/engine.rs`
- Modify: `crates/fulgur-ruby/spec/engine_spec.rb`
- Create: `crates/fulgur-ruby/spec/integration_spec.rb`

**Step 1: add render_html_to_file**

```rust
fn render_html_to_file(&self, html: String, path: String) -> Result<(), Error> {
    let engine_ref: &Engine = &self.inner;
    let p = std::path::PathBuf::from(path);
    let result = crate::gvl::without_gvl::<(&Engine, String, std::path::PathBuf), Option<Result<(), fulgur::Error>>>(
        (engine_ref, html, p),
        |(e, h, path)| Some(e.render_html_to_file(&h, &path)),
    );
    let ruby = Ruby::get().expect("ruby vm");
    match result {
        Some(Ok(())) => Ok(()),
        Some(Err(e)) => Err(map_fulgur_error(&ruby, e)),
        None => Err(Error::new(magnus::exception::runtime_error(), "render interrupted")),
    }
}
```

define に追加:

```rust
engine.define_method("render_html_to_file", method!(RbEngine::render_html_to_file, 2))?;
```

**Step 2: `spec/integration_spec.rb`**

```ruby
# frozen_string_literal: true

require "spec_helper"

RSpec.describe "Integration" do
  let(:fixtures) { File.expand_path("fixtures", __dir__) }
  let(:html) { File.read(File.join(fixtures, "simple.html")) }

  it "renders with AssetBundle (font + CSS)" do
    bundle = Fulgur::AssetBundle.new
    bundle.css "body { color: red }"
    bundle.font_file File.join(fixtures, "noto_sans.ttf")
    engine = Fulgur::Engine.new(page_size: :a4, assets: bundle)
    pdf = engine.render_html(html)
    expect(pdf.bytesize).to be > 100
    expect(pdf.to_s[0, 5]).to eq("%PDF-".b)
  end

  it "renders via builder chain + custom margin" do
    engine = Fulgur::Engine.builder
      .page_size(:letter)
      .margin(Fulgur::Margin.new(72, 36))
      .build
    pdf = engine.render_html(html)
    expect(pdf.bytesize).to be > 100
  end

  it "render_html_to_file writes %PDF-" do
    engine = Fulgur::Engine.new
    Dir.mktmpdir do |dir|
      path = File.join(dir, "out.pdf")
      engine.render_html_to_file(html, path)
      expect(File.binread(path)[0, 5]).to eq("%PDF-".b)
    end
  end
end
```

**Step 3: Run all specs**

```bash
bundle exec rake compile
bundle exec rspec
```

Expected: 全パス。

**Step 4: Commit**

```bash
git add crates/fulgur-ruby/
git commit -m "feat(fulgur-ruby): add render_html_to_file + integration specs"
```

---

## Task 11: README + 開発者ドキュメント

**Files:**

- Create: `crates/fulgur-ruby/README.md`
- Create: `crates/fulgur-ruby/CHANGELOG.md`
- Modify: `README.md` (workspace root、bindings セクションに Ruby を追加)

**Step 1: `crates/fulgur-ruby/README.md`**

以下を含む:

- インストール (`gem install fulgur`、ビルド要件: Rust toolchain)
- クイックスタート (Engine.new + render_html → write_to_path)
- API overview (Engine / AssetBundle / PageSize / Margin / Pdf)
- LLM 連携例 (to_base64)
- Development (bundle install, rake compile, rspec)

**Step 2: CHANGELOG**

```markdown
# Changelog

## 0.0.1 (unreleased)

- Initial Ruby binding (Magnus)
- Engine: kwargs constructor + builder chain
- AssetBundle: add_* + short aliases (css / font_file / image / image_file)
- Pdf result object: to_s / bytesize / to_base64 / to_data_uri / write_to_path / write_to_io
- GVL released during render_html / render_html_to_file
- Ruby 3.3+
```

**Step 3: workspace README に Ruby binding の行を追加**

**Step 4: markdownlint**

```bash
npx markdownlint-cli2 'crates/fulgur-ruby/**/*.md'
```

**Step 5: Commit**

```bash
git add crates/fulgur-ruby/README.md crates/fulgur-ruby/CHANGELOG.md README.md
git commit -m "docs(fulgur-ruby): add README + CHANGELOG"
```

---

## Task 12: 最終検証

**Step 1: workspace-wide ビルド**

```bash
cargo build --workspace
```

ruby-api feature off で fulgur-ruby も空 crate としてビルドされる。

**Step 2: fulgur crate テスト**

```bash
cargo test -p fulgur --lib
```

Expected: 441 passed（変化なし）。

**Step 3: fulgur-ruby の compile + rspec**

```bash
cd crates/fulgur-ruby
bundle exec rake
```

Expected: 全テストパス（Task 2-10 で書いた全 specs）。

**Step 4: fmt + clippy**

```bash
cd /home/ubuntu/fulgur/.worktrees/ruby-binding-mvp
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

**Step 5: Issue クローズ準備**

- `bd show fulgur-0x0` で acceptance criteria が満たされているか人手チェック
- finishing-a-development-branch skill に渡す

---

## Execution Options

**1. Subagent-Driven (this session)** — 同セッション内で Task 1 から順に fresh subagent に投げる。Task 間で code-review を挟む。fulgur-i5c（Python binding）と同じ進行で、ruff/typecheck の代わりに `cargo clippy` + `rspec` を回す。

**2. Parallel Session** — 別ターミナルでこの worktree に入り、`/executing-plans docs/plans/2026-04-17-ruby-binding-mvp.md` を流す。バッチ実行 + チェックポイントで進む。
