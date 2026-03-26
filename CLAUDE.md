# CLAUDE.md — Weaven プロジェクト 作業指示書

## 作業手順の原則

### oxidtr バグ対応手順
1. Red テスト追加（`tests/backend_rust.rs` 等）
2. `cargo test` で失敗確認
3. `src/` 以下を修正
4. 対象テストが Green になることを確認
5. `cargo test`（全 suite）で FAILED ゼロ確認
6. `cargo build --release`
7. `oxidtr generate models/oxidtr.als --target rust --output /tmp/self` + `oxidtr check --model models/oxidtr.als --impl /tmp/self` でセルフホスト検証
8. Weaven で `generate → copy → cargo build → cargo test` まで通ることを確認
9. `git diff > patch.patch` でパッチ出力。probe 用テストは最後に削除してから diff 取ること。

### Weaven バグ対応手順（weaven-core 変更時）
1. Red テスト追加（`tests/` 以下の該当ファイル）
2. `cargo test -p weaven-core` で失敗確認
3. `src/` 以下を修正
4. 対象テストが Green になることを確認
5. `cargo test -p weaven-core -p weaven-bevy -p weaven-wasm -p weaven-debugger-core --features "trace,parallel"` で全スイート FAILED ゼロ確認
6. `cargo build --release -p weaven-core --features "trace,parallel"`
7. `oxidtr generate models/weaven.als --target rust --output /tmp/weaven-check` + `oxidtr check --model models/weaven.als --impl /tmp/weaven-check` で検証

### フロントエンド UI 実装方針
oxidtr での型保証が効かない部分なので、Red-Green-Refactoring（テスト先行）で進めること。コンポーネントテスト（vitest + testing-library）とロジックテストを先に書いてから実装する。

### セッション引き継ぎ手順
ユーザーから提案があるまで引き継ぎ作業（CLAUDE.md + README_HANDOFF.md 作成・tar.gz 圧縮・present_files）に入らないこと。ユーザーが明示的に指示した場合のみ実行する。

---

## 開発コマンド早見表

```bash
# Rust テスト（weaven-debugger は GTK 依存で除外）
cargo test -p weaven-core -p weaven-bevy -p weaven-wasm -p weaven-debugger-core \
  --features "trace,parallel"

# oxidtr セルフホスト検証
OXIDTR=./oxidtr/target/release/oxidtr
$OXIDTR generate models/oxidtr.als --target rust --output /tmp/self
$OXIDTR check --model models/oxidtr.als --impl /tmp/self

# Weaven oxidtr 検証
$OXIDTR generate weaven/models/weaven.als --target rust --output /tmp/weaven-check
$OXIDTR check --model weaven/models/weaven.als --impl /tmp/weaven-check

# Frontend テスト（デバッガー UI）
cd weaven-debugger && npx vitest run

# Browser Adapter テスト
cd weaven-browser && npx vitest run

# WASM コンパイル確認（wasm-pack build は時間がかかるため cargo build で代替）
cargo build -p weaven-wasm --target wasm32-unknown-unknown

# Golden fixture 再生成（Rust 型変更時）
cargo test -p weaven-debugger-core --test golden_fixtures -- --ignored
cp weaven-debugger-core/tests/fixtures/*.json weaven-debugger/src/test/fixtures/
```

---

## プロジェクトナレッジ資料

以下のドキュメントはプロジェクトの全体設計を定義している。変更前に参照すること。

- `docs/weaven-spec.md` — Weaven Framework 設計仕様（全 13 章 + Appendix）
- `docs/weaven-debugger-brief.md` — デバッガー調査・設計依頼書
- `docs/weaven-debugger-design.md` — デバッガー設計仕様書（D1〜D6 判断完了）

---

## 次の作業（oxidtr C# backend 追加後）

### 完了済み
- Phase 1: Weaven Core (Tier 1) ✅
- Phase 2: Weaven Spatial (Tier 2) ✅
- Phase 3: Bevy Adapter ✅
- Phase 4: WASM + Browser Adapter ✅
- §11 全 Open Items (11.1〜11.7) ✅
- デバッガーツール (Tauri + React) ✅
- Phase 5: Unity Adapter ✅
  - `weaven-unity/` Rust FFI クレート（C ABI, cdylib）— 13テスト
  - `generated/cs/` oxidtr C# 型定義（Models.cs, Validators.cs）
  - `WeavenNative.cs` P/Invoke 宣言（iOS __Internal / その他 weaven_unity）
  - `WeavenWorld.cs` 高レベル C# Adapter（IDisposable, TickResult JSON パース）
- Phase 6: Network APIs — Adapter 統合 ✅
  - Core Network APIs（§8）全関数を 4 Adapter に統合
  - `weaven-bevy`: diff, policy filter, scoped snapshot, input buffer, rewind — 17テスト
  - `weaven-wasm`: JSON シリアライズ WASM バインディング 10 メソッド — 12テスト
  - `weaven-unity`: C ABI FFI 10 関数（diff, policy, scoped snapshot, input buffer, rewind）— 18テスト
  - `weaven-browser`: TypeScript ラッパー 10 メソッド + 型定義 — 29テスト

### 次のフェーズ候補
- **Phase 7: Weaven Editor** — ブラウザベース SM ビジュアルエディタ
  - weaven-browser + React Flow でトポロジー編集
  - weaven-wasm でライブ検証（tick preview）
  - Weaven Schema JSON の読み書き
