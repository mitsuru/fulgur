# Release Setup: Trusted Publishing

pyfulgur (PyPI) と fulgur (RubyGems) を OIDC Trusted Publishing で publish するための、
一度だけ必要な設定手順。

## 初回公開時の注意

pyfulgur と fulgur gem はどちらも PyPI / RubyGems に未登録の可能性がある。
既存プロジェクトと新規プロジェクトで UI フローが異なる:

- **新規 (pending publisher)**: プロジェクト名だけ予約し、初回 publish 時に
  OIDC claim で自動的に project が作成される。
- **既存 publisher 追加**: 既に project が存在する場合は publisher を追加登録。

## PyPI Trusted Publisher

### Production (pypi.org)

1. <https://pypi.org/manage/account/publishing/> にログイン
2. "Add a new pending publisher" (新規の場合) または既存 project の
   "Publishing" タブから publisher 追加:
   - PyPI Project Name: `pyfulgur`
   - Owner: `mitsuru`
   - Repository name: `fulgur`
   - Workflow name: `release-python.yml`
   - Environment name: `pypi`
3. GitHub リポジトリで Environment `pypi` を作成 (Settings → Environments → New environment)。保護ルール不要。

### TestPyPI (test.pypi.org)

本番公開前に dry-run を試す場合のみ。

1. <https://test.pypi.org/manage/account/publishing/> で同様に登録
2. Environment name: `testpypi`
3. GitHub リポジトリで Environment `testpypi` を作成

Dry-run 発火:

```bash
gh workflow run release-python.yml --field dry_run=true
```

## RubyGems Trusted Publisher

1. <https://rubygems.org/profile/oidc/api_key_roles> にログイン
2. "New API key role" → OIDC provider: GitHub Actions
3. 以下を登録:
   - Gem: `fulgur` (新規の場合は "Pending publishing" で予約)
   - Repository: `mitsuru/fulgur`
   - Workflow: `release-ruby.yml`
   - Environment: `rubygems`
4. 生成された role 名 (例: `rg_oidc_akr_xxxxxxxx`) をコピーして
   `.github/workflows/release-ruby.yml` の `role-to-assume:` 値を差し替える
5. GitHub リポジトリで Environment `rubygems` を作成

注意: RubyGems には TestPyPI に相当する staging 環境がないため、`release-ruby.yml`
の `workflow_dispatch` dry-run は publish をスキップするのみ (build + smoke test
のみ走る)。OIDC / credential フローの実動作検証は初回の本番リリースで行う。

## GitHub Environments

以下の 3 つの Environment を作成:

- `pypi`
- `testpypi` (dry-run 用)
- `rubygems`

保護ルール不要 (OIDC claim で scope されるため)。

## Release 手順

1. `release-prepare.yml` を `workflow_dispatch` で起動 (version 入力)
2. 作成された `release/vX.Y.Z` PR を merge
3. `release.yml` が tag + crates.io publish + GitHub Release publish
4. `release: published` で `release-python.yml` と `release-ruby.yml` が並行発火
5. 数分〜十数分後に PyPI / RubyGems へ反映
