# Weaven Debugger — 設計仕様書

**Version**: 0.1.0  
**Status**: 設計完了・実装待ち  
**前提ドキュメント**: weaven-spec.md §11.3, weaven-debugger-brief.md  
**作成日**: 2026-03-25

---

## 1. 設計判断サマリ

| # | 問い | 決定 | 根拠 |
|---|------|------|------|
| D1 | TraceRecorder 実装方式 | **TickOutput 拡張** | APIの境界がクリーン。`cfg(feature = "trace")` でゼロコスト化可能。Cascade Replay（Phase 4 内部観察）にも対応 |
| D2 | oxidtr モデル化スコープ | **スコープ M**: TraceEvent + DebugSession + TopologyGraph + invariants | テストモデルで全パターン実証済み。invariant テスト自動生成の実利あり。UI ステートは含めない |
| D3 | グラフ描画ライブラリ | **React Flow + dagre** | SM のリッチなカスタムノード（active state + context 表示）が React コンポーネントとして自然に記述可能。デバッグ対象は数十〜百ノード規模が典型的 |
| D4 | Tauri バージョン | **2.x** | stable 済み。plugin API 改善。1.x を選ぶ理由なし |
| D5 | 型共有方式 | **oxidtr generate --target rust/ts 一本** | スコープ M ならoxidtr で大半カバー。specta や手書き不要 |
| D6 | debugger.als 配置 | **weaven リポ内 `models/`** | weaven.als と同ディレクトリで `open` 相互参照可能 |

---

## 2. アーキテクチャ

```
Tauri 2.x App
  ├── Frontend (React 18 + TypeScript)
  │     ├── TopologyCanvas   — React Flow + dagre レイアウト
  │     ├── TracePanel       — signal flow ログ（TraceEvent リスト）
  │     ├── InspectorPanel   — guard / context 詳細（EvalTreeNode ツリー表示）
  │     ├── TimelinePanel    — tick スライダー / cascade ステッパー
  │     └── State: Zustand or Jotai
  │
  └── Rust Backend (Tauri commands)
        ├── weaven-core を workspace member として直接 link
        ├── DebugSession     — World + snapshot 履歴保持
        ├── TraceRecorder    — TickOutput.trace_events から取得
        └── weaven-schema    — JSON 定義ファイル読み込み
```

### 2.1 データフロー

```
[weaven-core tick()]
    │
    ├── TickOutput.trace_events: Vec<TraceEvent>   ← cfg(feature = "trace")
    ├── TickOutput.state_changes
    ├── TickOutput.system_commands
    └── TickOutput.continuous_outputs
          │
          ▼
[DebugSession (Rust)]
    │
    ├── snapshot 保存（WorldSnapshot）
    ├── TraceEvent 蓄積
    └── TopologyGraph 構築
          │
          ▼
[Tauri command / event]
    │
    ├── #[tauri::command] で DebugSession 操作
    └── app.emit() で tick ごとに TraceEvent を push
          │
          ▼
[Frontend (React/TS)]
    │
    ├── TopologyCanvas: GraphNode / GraphEdge を React Flow ノード・エッジに変換
    ├── TracePanel: TraceEvent[] をフィルタ・表示
    ├── InspectorPanel: GuardInspectionResult の EvalTreeNode をツリー描画
    └── TimelinePanel: TickCursor 操作 → Tauri command で snapshot 復元
```

---

## 3. weaven-core 変更: TraceRecorder（D1）

### 3.1 feature flag

```toml
# weaven-core/Cargo.toml
[features]
default = []
trace = []
```

### 3.2 TickOutput 拡張

```rust
pub struct TickOutput {
    pub state_changes:      BTreeMap<SmId, (StateId, StateId)>,
    pub system_commands:    Vec<SystemCommand>,
    pub continuous_outputs: BTreeMap<SmId, BTreeMap<String, f64>>,
    #[cfg(feature = "trace")]
    pub trace_events:       Vec<TraceEvent>,
}
```

### 3.3 TraceEvent 収集ポイント

| Phase | 収集イベント | タイミング |
|-------|-------------|-----------|
| Phase 2 | `GuardEvaluated` | 各 Transition の Guard 評価直後 |
| Phase 2 | `IrMatched` | InteractionRule マッチ判定直後 |
| Phase 3 | `TransitionFired` | Transition 発火時 |
| Phase 3 | `SignalEmitted` | Output Port からのシグナル発行時 |
| Phase 4 | `CascadeStep` | signal queue 処理の各ステップ |
| Phase 4 | `PipelineFiltered` | Connection/Input Pipeline の Filter でシグナルがブロックされた時 |
| Phase 4 | `GuardEvaluated` | カスケード中の受信 SM の Guard 評価時 |
| Phase 4 | `TransitionFired` | カスケード中の Transition 発火時 |
| Phase 4 | `SignalEmitted` | カスケード中のシグナル発行時 |

収集ロジックは `#[cfg(feature = "trace")]` で囲み、feature 無効時はゼロコスト。

---

## 4. oxidtr モデル: weaven-debugger.als（D2）

### 4.1 スコープ

| カテゴリ | モデル化する | モデル化しない |
|---------|------------|--------------|
| ドメインモデル | DebugSession, WorldSnapshot, TickCursor, FilterConfig | Tauri command の request/response ラッパー |
| トポロジー | TopologyGraph, GraphNode, GraphEdge, EdgeKind | グラフレイアウト座標 (x, y) |
| トレース | TraceEvent（全バリアント）, CascadeStep | WebSocket/IPC プロトコル |
| インスペクター | GuardInspectionResult, EvalTreeNode, ContextSnapshot, SignalSnapshot | React コンポーネント構造 |
| UI ステート | — | SelectedNode, PanelLayout, アニメーション状態 |

### 4.2 モデル構造（確認済みパターン）

以下の Alloy パターンは oxidtr による Rust/TS 生成が検証済み。

**abstract sig + sub-sig → Rust enum / TS discriminated union**

```alloy
abstract sig TraceEvent { tick: one Tick, phase: one Phase }
sig SignalEmitted extends TraceEvent { sourceSm: one SmId, port: one PortId }
sig TransitionFired extends TraceEvent { ... }
```

生成結果（Rust）:
```rust
pub enum TraceEvent {
    SignalEmitted { tick: Tick, phase: Phase, sourceSm: SmId, port: PortId },
    TransitionFired { tick: Tick, phase: Phase, ... },
}
```

生成結果（TS）:
```typescript
export interface SignalEmitted {
  readonly kind: "SignalEmitted";
  readonly tick: Tick;
  readonly phase: Phase;
  readonly sourceSm: SmId;
  readonly port: PortId;
}
export type TraceEvent = SignalEmitted | TransitionFired | ...;
```

> **注意**: abstract sig の共通フィールド伝播には oxidtr パッチ（oxidtr-abstract-sig-fields.patch）の適用が必要。

**再帰構造 → Vec / Array**

```alloy
sig EvalTreeNode { exprKind: one ExprKind, children: seq EvalTreeNode }
fact NoCyclicEvalTree { no n: EvalTreeNode | n in n.^children }
```

- Rust: `children: Vec<EvalTreeNode>` + `tc_children()` BFS 関数自動生成
- TS: `children: EvalTreeNode[]` + `tcChildren()` 自動生成

**構造的不変条件 → テスト自動生成**

```alloy
fact NoSelfLoop { no e: GraphEdge | e.edgeSource = e.edgeTarget }
fact SnapshotNonEmpty { all d: DebugSession | #d.snapshots > 0 }
```

- invariant テスト、boundary テスト、invalid テスト、cross-test（fact × predicate 保存）が oxidtr により自動生成

### 4.3 型共有パイプライン（D5）

```
models/weaven-debugger.als
    │
    ├── oxidtr generate --target rust --output weaven-debugger/src/generated/
    │   → models.rs, helpers.rs, tests.rs, fixtures.rs, newtypes.rs
    │
    ├── oxidtr generate --target ts --output weaven-debugger/ui/src/generated/
    │   → models.ts, helpers.ts, tests.ts, fixtures.ts, validators.ts
    │
    └── oxidtr check --model models/weaven-debugger.als --impl weaven-debugger/src/generated/
        → CI gate: 0 structural diffs required
```

Tauri command の引数・戻り値は oxidtr 生成型の組み合わせで構成。フロントエンド側で薄いラッパー型を手書きする。

---

## 5. フロントエンド設計（D3）

### 5.1 技術スタック

| 層 | 選定 | 理由 |
|----|------|------|
| グラフ描画 | React Flow 11+ | カスタムノード = React コンポーネント。SM の active state / context をノード内に自然に描画可能 |
| レイアウト | dagre | 有向グラフの階層レイアウト。Connection の方向性を視覚的に表現 |
| 状態管理 | Zustand or Jotai | debug session state のシンプルな管理。グローバルストア 1 つで十分 |
| スタイル | Tailwind CSS | weaven エコシステムで実績あり |

### 5.2 カスタムノード設計

React Flow のカスタムノードとして以下を実装:

**SmNode**: SM を表すノード。表示内容:
- SM 名
- active state（ハイライト表示）
- context の主要フィールド値（展開/折りたたみ可能）
- Input/Output Port のハンドル（React Flow Handle）

**IrNode**: InteractionRule を表す中間ノード。参加 SM 群を点線エッジで接続。

### 5.3 エッジ設計

| 種類 | 描画 | React Flow edge type |
|------|------|---------------------|
| Static Connection | 実線 | default (bezier) |
| Spatial Connection | 破線 | custom (dashed) |
| InteractionRule | 点線 | custom (dotted) |
| Signal flow（アクティブ） | 太線 + 色付き | animated edge |

### 5.4 パネル構成

```
┌──────────────────────────────────────────────────┐
│  TimelinePanel (tick slider + cascade stepper)   │
├──────────────────────┬───────────────────────────┤
│                      │                           │
│   TopologyCanvas     │   TracePanel              │
│   (React Flow)       │   (signal flow log)       │
│                      │                           │
│                      ├───────────────────────────┤
│                      │                           │
│                      │   InspectorPanel          │
│                      │   (guard / context)       │
│                      │                           │
└──────────────────────┴───────────────────────────┘
```

- TopologyCanvas がメイン領域（左 2/3）
- 右側に TracePanel と InspectorPanel を縦分割
- TimelinePanel は上部固定バー

### 5.5 インタラクション

| 操作 | 効果 |
|------|------|
| SM ノードクリック | InspectorPanel に context / port 詳細表示。TracePanel をその SM でフィルタ |
| エッジクリック | Connection の pipeline ステップ表示。TracePanel をその Connection でフィルタ |
| TraceEvent クリック | TopologyCanvas 上で該当 signal flow パスをハイライト |
| Timeline tick スライダー | Tauri command → snapshot 復元 → 全パネル更新 |
| Cascade stepper (◀ ▶) | Phase 4 内の CascadeStep を 1 つずつ進退。signal queue 状態を TracePanel に表示 |

---

## 6. Tauri Backend 設計

### 6.1 Tauri command 一覧

| command | 引数 | 戻り値 | 説明 |
|---------|------|--------|------|
| `load_schema` | `path: String` | `Result<TopologyGraph>` | weaven-schema JSON を読み込み、DebugSession を初期化 |
| `tick` | — | `TickResult` | 1 tick 進行。trace_events + state_changes を返す |
| `tick_n` | `n: u32` | `TickResult` | n tick 一括進行 |
| `seek_tick` | `tick: u64` | `WorldState` | 指定 tick の snapshot を復元。全 SM の状態を返す |
| `get_topology` | — | `TopologyGraph` | 現在の SM / Connection / IR グラフ |
| `inspect_guard` | `transition_id: TransitionId` | `GuardInspectionResult` | 指定 Transition の Guard 評価ツリー |
| `get_cascade_steps` | `tick: u64` | `Vec<CascadeStep>` | 指定 tick の Phase 4 カスケードステップ一覧 |
| `set_filter` | `config: FilterConfig` | — | 表示フィルタ更新 |
| `inject_signal` | `port: PortId, payload: Value` | — | デバッグ用シグナル注入 |

### 6.2 イベントストリーミング

tick ごとに Tauri の `app.emit()` で TraceEvent をフロントエンドに push する方式と、command の戻り値に含める方式がある。

**推奨**: command 戻り値に含める（`TickResult` に `trace_events` を持たせる）。理由:
- tick は明示的に command で呼ぶため、イベントのタイミングが command のレスポンスと一致
- ストリーミングは自動実行（play/pause ボタン）実装時に追加検討

### 6.3 DebugSession 状態管理

```rust
pub struct DebugSession {
    world: World,
    snapshots: Vec<(u64, WorldSnapshot)>,  // (tick_number, snapshot)
    current_tick: u64,
    max_snapshots: usize,  // メモリ上限（configurable、デフォルト 1000）
    trace_buffer: Vec<TraceEvent>,  // 直近 N tick 分を保持
}
```

snapshot は毎 tick 保存するとメモリを消費するため、間引き戦略を採用:
- 直近 100 tick は毎 tick 保存
- それ以前は 10 tick 間隔
- `max_snapshots` 超過時は最古から破棄

間引かれた tick への seek は、直前の snapshot から再シミュレーションで到達。

---

## 7. oxidtr パッチ依存

本設計は oxidtr に対する以下のパッチ適用を前提とする。

**oxidtr-abstract-sig-fields.patch** (113 行追加 / 43 行削除)

修正内容: abstract sig のフィールドを sub-sig の enum variant / union variant に伝播。全 6 バックエンド（Rust, TypeScript, Kotlin, Java, Swift, Go）に適用。

影響範囲:
- `src/backend/rust/mod.rs` — `generate_enum()`
- `src/backend/typescript/mod.rs` — `generate_union_type()`
- `src/backend/jvm/kotlin.rs` — `generate_sealed_class()`
- `src/backend/jvm/java.rs` — `generate_sealed_interface()`
- `src/backend/swift/mod.rs` — `generate_enum()`
- `src/backend/go/mod.rs` — `generate_enum()`

検証:
- `cargo test` 全 suite FAILED ゼロ（新規 regression test 2 件含む）
- セルフホスト検証 `oxidtr generate + check` 整合性 OK
- デバッガー用 `.als` から Rust/TS 生成 → TraceEvent 全 variant に共通フィールド出現確認済み

---

## 8. 実装フェーズ

### Phase 1: 基盤構築

1. oxidtr パッチ適用・マージ
2. `models/weaven-debugger.als` 正式作成（§4 のモデル構造に基づく）
3. weaven-core に `trace` feature 追加（§3 の TickOutput 拡張 + TraceEvent 収集）
4. oxidtr パイプライン接続（debugger.als → Rust + TS 型生成をビルドスクリプトに組み込み）

### Phase 2: Tauri アプリ雛形

5. `weaven-debugger/` Tauri 2.x プロジェクト初期化
6. Rust backend: DebugSession + 基本 command 実装（`load_schema`, `tick`, `seek_tick`）
7. Frontend: React + React Flow + Tailwind セットアップ
8. TopologyCanvas: 静的グラフ描画（dagre レイアウト）

### Phase 3: MVP 機能

9. TracePanel: TraceEvent リスト表示 + SM/Connection フィルタ
10. InspectorPanel: Guard 評価ツリー表示（EvalTreeNode の再帰描画）
11. TimelinePanel: tick スライダー + snapshot seek
12. Cascade Replay: Phase 4 ステッパー

### Phase 4: 磨き込み

13. Signal flow ハイライト（TraceEvent クリック → TopologyCanvas パスアニメーション）
14. inject_signal command（デバッグ用シグナル注入）
15. FilterConfig UI
16. パフォーマンスチューニング（大規模グラフ時の React Flow 仮想化）

---

## 9. 将来機能（スコープ外）

ブリーフ §1.3 に記載の将来機能。本設計では対象外だが、アーキテクチャ上の拡張点を示す。

| 機能 | 拡張点 |
|------|--------|
| InteractionRule spatial 可視化 | TopologyCanvas に位置オーバーレイレイヤー追加。weaven-spatial の空間インデックスデータを DebugSession 経由で取得 |
| Named Table ブラウザ | 新規パネル追加。weaven-core の Named Table API を Tauri command で公開 |
| Expression Language REPL | InspectorPanel 内にREPL 入力欄追加。weaven-core の Expression evaluator を直接呼び出し |

---

## Appendix A: oxidtr 調査結果

### A.1 リポジトリ状態（2026-03-25 時点）

- **URL**: https://github.com/penta2himajin/oxidtr
- **実装済みバックエンド**: Rust, TypeScript, Kotlin, Java, Swift, Go（README は Rust のみ Done と記載だが、実際は全バックエンド実装済み）
- **テスト数**: 約 650（`cargo test` 全パス）
- **セルフホスト**: `models/oxidtr.als` で generate → check → 0 diff 確認済み

### A.2 デバッガー必須パターンの検証結果

| パターン | Alloy 記法 | Rust 生成 | TS 生成 | 状態 |
|---------|-----------|----------|---------|------|
| abstract sig + sub-sig（フィールドなし） | `abstract sig Role {}` + `one sig Admin extends Role {}` | `enum Role { Admin, Viewer }` | `type Role = "Admin" \| "Viewer"` | ✓ 既存 |
| abstract sig + sub-sig（共通フィールドあり） | `abstract sig Event { tick: one Tick }` + `sig Started extends Event { ... }` | `enum Event { Started { tick, ... } }` | `interface Started { kind, tick, ... }` | ✓ パッチ後 |
| 再帰構造 | `sig Node { children: seq Node }` | `children: Vec<Node>` | `children: Node[]` | ✓ 既存 |
| ^field 循環禁止 | `no n: Node \| n in n.^children` | `tc_children()` BFS 関数 + テスト | `tcChildren()` + テスト | ✓ 既存 |
| seq | `snapshots: seq Snapshot` | `Vec<Snapshot>` | `Snapshot[]` | ✓ 既存 |
| lone | `selected: lone SmId` | `Option<SmId>` | `SmId \| null` | ✓ 既存 |
| set | `trace: set TraceEvent` | `BTreeSet<TraceEvent>` | `Set<TraceEvent>` | ✓ 既存 |

### A.3 oxidtr が自動生成する成果物（debugger.als から）

| 成果物 | Rust | TypeScript |
|--------|------|-----------|
| 型定義 | `models.rs` — struct, enum | `models.ts` — interface, union |
| TC 関数 | `helpers.rs` — `tc_children()` 等 | `helpers.ts` — `tcChildren()` 等 |
| invariant テスト | `tests.rs` — property test + boundary + invalid | `tests.ts` — 同等 |
| cross-test | `tests.rs` — fact × pred 保存テスト（`#[ignore]` + `todo!()`) | `tests.ts` — 同等 |
| fixture | `fixtures.rs` — `default_*()`, `boundary_*()`, `invalid_*()` | `fixtures.ts` — 同等 |
| newtype | `newtypes.rs` — TryFrom validated wrappers | — |
| validator | — | `validators.ts` — invariant 検証関数 |

---

*関連ドキュメント: weaven-spec.md, weaven-debugger-brief.md*  
*oxidtr パッチ: oxidtr-abstract-sig-fields.patch*
