# npm Distribution Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** `npx @fulgur-rs/cli render input.html -o output.pdf` でfulgur CLIをゼロインストール起動できるnpmパッケージを配布する。

**Architecture:** esbuildスタイルの分割パッケージ方式。`@fulgur-rs/cli`（メタパッケージ）が`optionalDependencies`で6プラットフォームのバイナリパッケージを参照し、`install.js`がプラットフォームを検出してバイナリを`bin/fulgur`にコピーする。バイナリはCIのみが書き込む（gitには含まない）。

**Tech Stack:** Node.js (install.js/CJS), npm optionalDependencies, GitHub Actions, Rust cross-compilation (既存 release.yml)

---

## Task 1: packages/npm/ ディレクトリ構成とパッケージ雛形

### 対象ファイル

- Create: `packages/npm/meta/package.json`
- Create: `packages/npm/meta/install.js`
- Create: `packages/npm/meta/bin/.gitkeep`
- Create: `packages/npm/linux-x64/package.json`
- Create: `packages/npm/linux-x64/bin/.gitkeep`
- Create: `packages/npm/linux-x64-musl/package.json`
- Create: `packages/npm/linux-x64-musl/bin/.gitkeep`
- Create: `packages/npm/linux-arm64/package.json`
- Create: `packages/npm/linux-arm64/bin/.gitkeep`
- Create: `packages/npm/darwin-arm64/package.json`
- Create: `packages/npm/darwin-arm64/bin/.gitkeep`
- Create: `packages/npm/darwin-x64/package.json`
- Create: `packages/npm/darwin-x64/bin/.gitkeep`
- Create: `packages/npm/win32-x64/package.json`
- Create: `packages/npm/win32-x64/bin/.gitkeep`

### Step 1: メタパッケージの package.json を作成

`packages/npm/meta/package.json`:

```json
{
  "name": "@fulgur-rs/cli",
  "version": "0.0.0",
  "description": "HTML to PDF conversion CLI",
  "keywords": ["html", "pdf", "cli"],
  "license": "MIT OR Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/fulgur-rs/fulgur.git",
    "directory": "packages/npm/meta"
  },
  "bin": { "fulgur": "bin/fulgur" },
  "scripts": { "postinstall": "node install.js" },
  "optionalDependencies": {
    "@fulgur-rs/cli-linux-x64":      "0.0.0",
    "@fulgur-rs/cli-linux-x64-musl": "0.0.0",
    "@fulgur-rs/cli-linux-arm64":    "0.0.0",
    "@fulgur-rs/cli-darwin-arm64":   "0.0.0",
    "@fulgur-rs/cli-darwin-x64":     "0.0.0",
    "@fulgur-rs/cli-win32-x64":      "0.0.0"
  }
}
```

### Step 2: install.js を作成

`packages/npm/meta/install.js`:

```js
#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');

const PLATFORMS = {
  'linux-x64':      { os: 'linux',  cpu: 'x64',   pkg: '@fulgur-rs/cli-linux-x64',      bin: 'fulgur' },
  'linux-x64-musl': { os: 'linux',  cpu: 'x64',   pkg: '@fulgur-rs/cli-linux-x64-musl', bin: 'fulgur' },
  'linux-arm64':    { os: 'linux',  cpu: 'arm64', pkg: '@fulgur-rs/cli-linux-arm64',    bin: 'fulgur' },
  'darwin-arm64':   { os: 'darwin', cpu: 'arm64', pkg: '@fulgur-rs/cli-darwin-arm64',   bin: 'fulgur' },
  'darwin-x64':     { os: 'darwin', cpu: 'x64',   pkg: '@fulgur-rs/cli-darwin-x64',     bin: 'fulgur' },
  'win32-x64':      { os: 'win32',  cpu: 'x64',   pkg: '@fulgur-rs/cli-win32-x64',      bin: 'fulgur.exe' },
};

function isMusl() {
  try {
    const maps = fs.readFileSync('/proc/self/maps', 'utf8');
    return maps.includes('musl');
  } catch {
    return false;
  }
}

function detectPlatformKey() {
  const os = process.platform;
  const cpu = process.arch;

  if (os === 'linux' && cpu === 'x64') {
    return isMusl() ? 'linux-x64-musl' : 'linux-x64';
  }
  if (os === 'linux' && cpu === 'arm64') return 'linux-arm64';
  if (os === 'darwin' && cpu === 'arm64') return 'darwin-arm64';
  if (os === 'darwin' && cpu === 'x64') return 'darwin-x64';
  if (os === 'win32' && cpu === 'x64') return 'win32-x64';
  return null;
}

const platformKey = detectPlatformKey();
if (!platformKey) {
  process.stderr.write(
    `@fulgur-rs/cli: unsupported platform ${process.platform}/${process.arch}\n`
  );
  process.exit(1);
}

const platform = PLATFORMS[platformKey];
let pkgDir;
try {
  pkgDir = path.dirname(require.resolve(`${platform.pkg}/package.json`));
} catch {
  process.stderr.write(
    `@fulgur-rs/cli: platform package ${platform.pkg} not found.\n` +
    `This usually means it was not installed (e.g., --ignore-optional was used).\n`
  );
  process.exit(1);
}

const src = path.join(pkgDir, 'bin', platform.bin);
const destDir = path.join(__dirname, 'bin');
const dest = path.join(destDir, 'fulgur' + (process.platform === 'win32' ? '.exe' : ''));

fs.mkdirSync(destDir, { recursive: true });
fs.copyFileSync(src, dest);
fs.chmodSync(dest, 0o755);
```

### Step 3: bin/.gitkeep を作成

```bash
touch packages/npm/meta/bin/.gitkeep
```

### Step 4: プラットフォームパッケージの package.json を6つ作成

`packages/npm/linux-x64/package.json`:

```json
{
  "name": "@fulgur-rs/cli-linux-x64",
  "version": "0.0.0",
  "description": "fulgur CLI binary for Linux x64",
  "license": "MIT OR Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/fulgur-rs/fulgur.git",
    "directory": "packages/npm/linux-x64"
  },
  "os": ["linux"],
  "cpu": ["x64"],
  "files": ["bin/"]
}
```

同様のパターンで残り5つを作成（name, description, os, cpuのみ変える）:

| ディレクトリ | name | os | cpu |
|---|---|---|---|
| `linux-x64-musl/` | `@fulgur-rs/cli-linux-x64-musl` | `["linux"]` | `["x64"]` |
| `linux-arm64/` | `@fulgur-rs/cli-linux-arm64` | `["linux"]` | `["arm64"]` |
| `darwin-arm64/` | `@fulgur-rs/cli-darwin-arm64` | `["darwin"]` | `["arm64"]` |
| `darwin-x64/` | `@fulgur-rs/cli-darwin-x64` | `["darwin"]` | `["x64"]` |
| `win32-x64/` | `@fulgur-rs/cli-win32-x64` | `["win32"]` | `["x64"]` |

### Step 5: 各プラットフォームパッケージの bin/.gitkeep を作成

```bash
for d in linux-x64 linux-x64-musl linux-arm64 darwin-arm64 darwin-x64 win32-x64; do
  mkdir -p packages/npm/$d/bin
  touch packages/npm/$d/bin/.gitkeep
done
```

### Step 6: install.js の動作確認（ドライラン）

```bash
cd packages/npm/meta
# Node.js が install.js を構文エラーなく読み込めることを確認
node -c install.js
```

期待値: `install.js syntax OK`

### Step 7: コミット

```bash
git add packages/npm/
git commit -m "feat: add npm package scaffolding for @fulgur-rs/cli"
```

---

## Task 2: release.yml に darwin-x64 ビルドと npm publish ステップを追加

### 対象ファイル

- Modify: `.github/workflows/release.yml`

### Step 1: build-binaries matrix に x86_64-apple-darwin を追加

`.github/workflows/release.yml` の `build-binaries` ジョブの `matrix.include` に追記:

```yaml
          - target: x86_64-apple-darwin
            os: macos-13
            archive: tar.gz
```

`macos-13` は GitHub の最後の Intel macOS ランナー。`macos-latest` は現在 Apple Silicon (`macos-15`) を指すため使用不可。

### Step 2: build-binaries の upload-artifact step にnpm用artifactも追加

各プラットフォームビルド後、バイナリを `npm-<platform>` という名前でも upload する。既存の upload stepの後に追加：

```yaml
      - name: Stage npm binary (unix)
        if: matrix.archive == 'tar.gz'
        run: |
          mkdir -p npm-bin
          cp target/${{ matrix.target }}/release/fulgur npm-bin/fulgur

      - name: Stage npm binary (windows)
        if: matrix.archive == 'zip'
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force npm-bin
          Copy-Item "target/${{ matrix.target }}/release/fulgur.exe" "npm-bin/fulgur.exe"

      - uses: actions/upload-artifact@v4
        with:
          name: npm-${{ matrix.target }}
          path: npm-bin/
```

### Step 3: publish-npm ジョブを追加

`release` ジョブの後に `publish-npm` ジョブを追加する。このジョブは `build-binaries` と `publish` の両方が成功した後に実行される。

```yaml
  publish-npm:
    name: Publish npm packages
    needs: [publish, build-binaries]
    runs-on: ubuntu-latest
    environment: release
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@v4
        with:
          ref: "v${{ needs.publish.outputs.version }}"

      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'

      - name: Download all npm artifacts
        uses: actions/download-artifact@v4
        with:
          pattern: npm-*
          path: npm-artifacts
          merge-multiple: false

      - name: Publish platform packages
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
          VERSION: ${{ needs.publish.outputs.version }}
        run: |
          set -e
          # Target → npm package directory mapping
          declare -A TARGET_TO_DIR=(
            ["x86_64-unknown-linux-gnu"]="linux-x64"
            ["x86_64-unknown-linux-musl"]="linux-x64-musl"
            ["aarch64-unknown-linux-gnu"]="linux-arm64"
            ["aarch64-apple-darwin"]="darwin-arm64"
            ["x86_64-apple-darwin"]="darwin-x64"
            ["x86_64-pc-windows-msvc"]="win32-x64"
          )

          for target in "${!TARGET_TO_DIR[@]}"; do
            dir="${TARGET_TO_DIR[$target]}"
            pkg_dir="packages/npm/$dir"
            artifact_dir="npm-artifacts/npm-$target"

            # バイナリをパッケージのbin/へコピー
            if [ -f "$artifact_dir/fulgur.exe" ]; then
              cp "$artifact_dir/fulgur.exe" "$pkg_dir/bin/fulgur.exe"
            else
              cp "$artifact_dir/fulgur" "$pkg_dir/bin/fulgur"
              chmod +x "$pkg_dir/bin/fulgur"
            fi

            # バージョンを設定してpublish
            cd "$pkg_dir"
            npm version "$VERSION" --no-git-tag-version --allow-same-version
            npm publish --access public
            cd - > /dev/null
          done

      - name: Publish meta package
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
          VERSION: ${{ needs.publish.outputs.version }}
        run: |
          cd packages/npm/meta
          # optionalDependencies のバージョンも更新
          node -e "
            const fs = require('fs');
            const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
            pkg.version = process.env.VERSION;
            for (const k of Object.keys(pkg.optionalDependencies)) {
              pkg.optionalDependencies[k] = process.env.VERSION;
            }
            fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
          "
          npm publish --access public
```

### Step 4: 変更内容を検証

```bash
# YAMLの構文確認
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "YAML OK"
```

期待値: `YAML OK`

### Step 5: コミット

```bash
git add .github/workflows/release.yml
git commit -m "ci: add x86_64-apple-darwin build and npm publish to release.yml"
```

---

## 動作確認（ローカル模擬テスト）

リリースCI全体はローカル実行不可だが、以下でinstall.jsを疑似テストできる:

```bash
# 現在のプラットフォームのバイナリをビルドして模擬インストール
cargo build --release --bin fulgur -p fulgur-cli

# bin/ に手動配置してinstall.jsを実行
PLAT=$(node -e "
  const os = process.platform;
  const cpu = process.arch;
  const map = {'linux-x64':'linux-x64','darwin-arm64':'darwin-arm64','darwin-x64':'darwin-x64'};
  console.log(map[os+'-'+cpu] || 'linux-x64');
")
mkdir -p packages/npm/$PLAT/bin
cp target/release/fulgur packages/npm/$PLAT/bin/fulgur

cd packages/npm/meta
node -e "require.resolve = (p) => require('path').resolve('../' + p.replace('@fulgur-rs/cli-','') + '/package.json');"
# → 実際のテストはCIでのみ完結
```

実質的な動作確認はリリースドラフト環境での `npm publish --dry-run` で行う。

---

## 事前条件チェックリスト

実装前に確認:

- [ ] npm org `@fulgur-rs` が作成済みであること (`npm org create fulgur-rs`)
- [ ] npmjs.com の automation token が GitHub secrets `NPM_TOKEN` として登録済みであること
- [ ] GitHub Actions の `release` environment に `NPM_TOKEN` secret が設定済みであること
