# Weaven — セッション引き継ぎ資料

**最終更新**: 2026-03-26  
**テスト**: Rust 229 + Debugger UI 84 + Browser TS 18 = **331 total, 0 failures**

---

## プロジェクト構成

```
weaven/
  CLAUDE.md                         — 作業手順・開発コマンド（次セッション用）
  Cargo.toml                        — workspace
  docs/
    weaven-spec.md
    weaven-debugger-brief.md
    weaven-debugger-design.md
  models/
    weaven.als                      — Alloy 形式仕様
    weaven-debugger.als
  weaven-core/                      — ゲームロジックランタイム
    src/ (types, tick, expr[+parser], schema, network[+scope API], spatial, trace, error)
    tests/ (20ファイル: ir_dirty_flag, parallel_phase2, sync_render_scope, parser, 他)
    Cargo.toml  features: trace, parallel
  weaven-bevy/                      — Phase 3 Bevy Adapter
    src/lib.rs  (load_world_from_schema, sync_position, run_headless_scenario, 12テスト)
  weaven-wasm/                      — Phase 4 WASM Adapter
    src/lib.rs  (WeavenSession wasm-bindgen bindings, 7テスト)
  weaven-browser/                   — Phase 4 TypeScript Adapter
    src/WeavenAdapter.ts  (RAF loop, HitStop, callbacks, 18テスト)
  weaven-debugger-core/             — Tauri非依存デバッガーロジック
  weaven-debugger/                  — Tauri 2.x デバッガーアプリ (84テスト)
```

---

## テスト合計

| 区分 | 数 |
|------|-----|
| weaven-core (trace+parallel) | 190 |
| weaven-bevy | 12 |
| weaven-wasm | 7 |
| weaven-debugger-core | 20 |
| weaven-debugger UI (vitest) | 84 |
| weaven-browser TS | 18 |
| **合計** | **331** |

---

## §11 Open Items — 全完了

11.1 BNF/Parser ✅ / 11.2 IR Dirty-flag ✅ / 11.3 Debugging ✅ / 11.5 Error ✅ / 11.6 Multi-threading ✅ / 11.7 Sync Scope ✅

---

## spec §13.4 フェーズ進捗

| Phase | 状態 |
|-------|------|
| 1: Core (Tier 1) | ✅ |
| 2: Spatial (Tier 2) | ✅ |
| 3: Bevy Adapter | ✅ |
| 4: WASM + Browser | ✅ |
| **5: Unity Adapter** | **oxidtr C# backend 待ち** |
| 6: Network APIs | 部分実装 |
| 7: Weaven Editor | Phase 4 後 |

---

## 重要な実装ノート

### IrSignal に source_sm フィールド追加済み
```rust
IrSignal {
    source_sm:   None,   // ← 全リテラルに追加必須
    target_sm:   ...,
    target_port: ...,
    signal:      ...,
}
```

### Schema JSON トップレベルキー
```json
{ "state_machines": [...], "connections": [], "named_tables": [] }
```
（"sms" は不正）

### parallel feature の制約
weaven-wasm では rayon 無効（WASM非対応）。weaven-bevy は任意。

---

## 開発コマンド

```bash
# Rust 全テスト
cargo test -p weaven-core -p weaven-bevy -p weaven-wasm -p weaven-debugger-core \
  --features "trace,parallel"

# WASM コンパイル確認
cargo build -p weaven-wasm --target wasm32-unknown-unknown

# Browser Adapter
cd weaven-browser && npx vitest run

# Debugger UI
cd weaven-debugger && npx vitest run

# oxidtr 検証
oxidtr generate models/weaven.als --target rust --output /tmp/weaven-check
oxidtr check --model models/weaven.als --impl /tmp/weaven-check
```

---

## 次のアクション（oxidtr C# backend 完成後）

1. `weaven-unity/` クレート作成
2. `oxidtr generate --target cs` で C# 型生成
3. C# Adapter 実装（`WeavenWorld.cs`, tick system, port bridge）
4. Unity native plugin build（C ABI）
