# Weaven Debugger — 調査・設計依頼書

**対象**: Weaven Framework デバッグ・可視化ツール  
**実装方針**: Tauri + Rust バックエンド（weaven-core 直結）+ フロントエンド UI  
**依頼背景**: §11.3 Open Items。氷山の一角しか見えないブラックボックス問題を解決し、「なぜ遷移しなかったか」「シグナルはどこを通ったか」を即座に調査できるようにする。

---

## 1. 目的と機能要件

### 1.1 解決したい問題
- ガードが通らない原因が分からない（context 値 / signal payload / 優先度競合）
- Phase 4 カスケードの途中でシグナルが消える（Pipeline Filter? Redirect?）
- InteractionRule がマッチしなかった理由が不明
- Tick N の状態を任意の時点に巻き戻して確認したい

### 1.2 必須機能（MVP）
1. **Topology View** — SM・Connection・InteractionRule をグラフとして描画
   - ノード: SM（active state を表示）
   - エッジ: Connection（delay / pipeline ステップ数）
   - InteractionRule ノードは SM ノード群を結ぶ点線
2. **Signal Flow Tracer** — 1 tick 内のシグナル経路を追跡
   - source SM → Connection pipeline → target port → guard 評価結果
3. **Guard Inspector** — 特定 Transition がなぜ発火しなかったかを表示
   - context フィールド値、signal payload、guard 式の評価ツリー
4. **Cascade Replay** — Phase 4 を 1 ステップずつ再生
   - signal_queue の状態をスナップショットごとに表示
5. **Snapshot Browser** — 任意の tick に巻き戻して状態確認

### 1.3 将来機能（優先度低）
- InteractionRule マッチの spatial 可視化（位置オーバーレイ）
- Named Table ブラウザ
- Expression Language REPL（context / signal を与えて式を評価）

---

## 2. アーキテクチャ候補

```
Tauri App
  ├── Frontend (React/TS)
  │     ├── TopologyCanvas   — グラフ描画（React Flow 等）
  │     ├── TracePanel       — signal flow ログ
  │     ├── InspectorPanel   — guard / context 詳細
  │     └── TimelinePanel    — tick スライダー / cascade ステッパー
  │
  └── Rust Backend (Tauri commands)
        ├── weaven-core を直接 link
        ├── DebugSession     — World + snapshot 履歴保持
        ├── TraceRecorder    — tick 実行中にイベントを記録
        └── weaven-schema    — JSON 定義ファイルの読み込み
```

---

## 3. oxidtr モデル化の調査ポイント

oxidtr は Alloy (.als) モデルから **型定義・バリデータ・テスト** を生成する。  
以下の観点で「モデル化できる範囲」と「できない範囲（UI描画ロジック等）」を調査・判断してほしい。

### 3.1 モデル化が期待できる部分

#### A) DebugSession ドメインモデル
```alloy (イメージ)
sig DebugSession {
  world:           one WeavenWorld,
  snapshots:       seq WorldSnapshot,
  current_tick:    one TickCursor,
  selected_sm:     lone SmId,
  trace:           set TraceEvent
}
fact SnapshotOrdering { ... }  -- tick 番号の単調増加
```

- `WorldSnapshot` の型（既に weaven-core に存在）
- `TraceEvent` のバリアント定義（SignalEmitted, GuardEvaluated, TransitionFired, CascadeStep, IrMatched 等）
- `TickCursor` の範囲制約（0 <= current <= max_tick）
- `FilterConfig`（どの SM / Connection を表示するか）

#### B) トポロジーグラフのデータモデル
```alloy (イメージ)
sig GraphNode { sm: one SmId }
sig GraphEdge { source: one GraphNode, target: one GraphNode, conn: one ConnectionId }
sig TopologyGraph { nodes: set GraphNode, edges: set GraphEdge }
fact NoSelfLoop { no e: GraphEdge | e.source = e.target }
```

- ノード・エッジの制約（自己ループ禁止、Connection の方向性）
- PortKind による edge の分類（Static / Spatial / IR）

#### C) TraceEvent のバリアント
```alloy (イメージ)
abstract sig TraceEvent { tick: one Tick, phase: one Phase }
sig SignalEmitted    extends TraceEvent { source_sm: one SmId, port: one PortId }
sig PipelineFiltered extends TraceEvent { connection: one ConnectionId }
sig GuardEvaluated  extends TraceEvent { transition: one TransitionId, result: one Bool }
sig CascadeStep     extends TraceEvent { depth: one Int, queue_size: one Int }
```

- イベントの順序制約（Phase 1→2→3→4→5→6）
- cascade depth の上限制約（max_cascade_depth）
- guard 評価は Phase 2 または Phase 4 のみ

#### D) Inspector クエリ結果
```alloy (イメージ)
sig GuardInspectionResult {
  transition: one TransitionId,
  fired:       one Bool,
  context_at_eval: one ContextSnapshot,
  signal_at_eval:  lone SignalSnapshot,
  expr_tree:       one EvalTreeNode
}
sig EvalTreeNode {
  expr_kind: one ExprKind,
  value:     one Float,
  children:  seq EvalTreeNode
}
fact NoCyclicEvalTree { no n: EvalTreeNode | n in n.^children }
```

### 3.2 モデル化が難しい / 不要な部分

- **グラフレイアウトアルゴリズム**（Dagre / Force-directed）— UI ライブラリに委譲
- **描画座標（x, y）**— UI ステートであり Alloy の関心外
- **WebSocket / IPC プロトコルの詳細**— Tauri の command/event 機構で対応
- **アニメーション状態**— CSS / Framer Motion 等
- **React コンポーネントの構造**— UI フレームワークに委譲

---

## 4. Tauri 固有の調査ポイント

### 4.1 基本アーキテクチャ確認
- Tauri 2.x vs 1.x の差分（型定義の違い、plugin API）
- `#[tauri::command]` で weaven-core の型を直接 serialize するための serde 対応
- イベント streaming（`app.emit()` で tick ごとに TraceEvent を push）vs ポーリング

### 4.2 weaven-core との結合方法
- `weaven-core` を Tauri backend の workspace member として直接 link
- `TraceRecorder` を weaven-core に hook する方法：
  - Option A: `World` にコールバック Vec を追加（侵襲的）
  - Option B: `tick()` の戻り値 `TickOutput` を拡張（`trace_events: Vec<TraceEvent>`）
  - Option C: 別プロセスで weaven-core を動かし IPC 経由で観察（非侵襲的だが複雑）
- **推奨調査**: Option B が最もクリーンか？ `TickOutput` の拡張コストを評価

### 4.3 フロントエンド技術選定
- **グラフ描画**: React Flow vs D3 vs Cytoscape.js — topology view のインタラクティブ性要件
- **状態管理**: Zustand / Jotai — debug session state のシンプルな管理
- **スタイル**: Tailwind CSS（既に weaven-core の生成コードで実績あり）
- **型共有**: `specta` crate で Rust 型を TypeScript 型に自動変換（要調査）

### 4.4 開発体験
- hot reload（フロントエンド変更時）
- Rust backend の変更時の rebuild 速度（weaven-core が大きくなった場合）

---

## 5. oxidtr パイプラインとの統合

oxidtr は `weaven-debugger.als` を作成し、以下を生成できる可能性がある：

| 生成物 | oxidtr feature | 対象言語 |
|---|---|---|
| `TraceEvent` 型定義 | `generate --target rust` | Rust backend |
| `TraceEvent` TypeScript 型 | `generate --target ts` | Frontend |
| `TraceEvent` JSON Schema | JSON Schema 生成 | IPC 検証 |
| invariant validator（SnapshotOrdering 等） | newtypes | Rust |
| `check` で型整合性保証 | `check --model --impl` | 全言語 |

**調査すべき具体的な問い：**
1. `EvalTreeNode`（再帰構造）は oxidtr でモデル化できるか？  
   → `fact NoCyclicEvalTree` が oxidtr のパーサーで通るか
2. `seq TraceEvent`（順序付きイベントリスト）の Rust 生成で `Vec<TraceEvent>` になるか
3. `abstract sig TraceEvent` の sub-sig が Rust 側で `enum` になるか（既に確認済みだが debugger.als 規模で問題が出ないか）
4. TypeScript backend で `TraceEvent` の union type が正しく生成されるか

---

## 6. 設計上の意思決定事項

調査後に以下を決定してほしい：

| 番号 | 問い | 選択肢 |
|---|---|---|
| D1 | TraceRecorder の実装方式 | TickOutput 拡張 / World コールバック / 別プロセス |
| D2 | oxidtr のモデル化スコープ | TraceEvent のみ / 全 DebugSession / UI state も含む |
| D3 | フロントエンドのグラフ描画ライブラリ | React Flow / D3 / Cytoscape.js |
| D4 | Tauri バージョン | 1.x / 2.x |
| D5 | 型共有方式 | specta / 手書き / JSON Schema 経由 |
| D6 | debugger.als の配置 | weaven リポジトリ内 / 独立リポジトリ |

---

## 7. 成果物イメージ

調査完了後に作成してほしいもの：

1. `models/weaven-debugger.als` — DebugSession・TraceEvent・TopologyGraph の Alloy モデル
2. `weaven-debugger/` crate — Tauri backend（`src-tauri/`）
3. `weaven-debugger/ui/` — React + TypeScript frontend
4. oxidtr パイプライン設定（`weaven-debugger.als` → Rust + TS 型生成）

---

## 8. 参考：weaven-core の関連インターフェース

調査時に把握しておくべき既存 API：

```rust
// tick.rs
pub struct TickOutput {
    pub state_changes:      BTreeMap<SmId, (StateId, StateId)>,
    pub system_commands:    Vec<SystemCommand>,
    pub continuous_outputs: BTreeMap<SmId, BTreeMap<String, f64>>,
    // ← ここに trace_events: Vec<TraceEvent> を追加する想定
}

// types.rs
pub struct World {
    pub defs:             BTreeMap<SmId, SmDef>,
    pub instances:        BTreeMap<SmId, SmInstance>,
    pub active_set:       BTreeSet<SmId>,
    pub connections:      Vec<Connection>,
    pub interaction_rules: Vec<InteractionRuleDef>,
    pub signal_queue:     VecDeque<QueuedSignal>,
    pub spatial_index:    Option<SpatialIndex>,
    // ... 他多数
}

// network.rs
pub fn snapshot(world: &World) -> WorldSnapshot { ... }
pub fn restore(world: &mut World, snap: &WorldSnapshot) { ... }
pub fn diff_snapshots(before: &WorldSnapshot, after: &WorldSnapshot) -> Vec<SmStateDiff> { ... }
```

---

*作成日: 2026-03-25*  
*関連: weaven-spec.md §11.3, §7.1, §12.5*
