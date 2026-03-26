# Project Weaven — Design Specification

**Version**: 0.2.0 (Draft)
**Status**: Design Phase

---

## 1. Overview

### 1.1 What Weaven Is

Weaven is an **Interaction-Topology-Oriented Game Framework**. It models a game world as a collection of entities, each holding one or more state machines (SMs), and describes all inter-entity interactions as typed connections between those SMs. The topology of connections — who affects whom, through what, and under what conditions — is a first-class, declarative, inspectable data structure.

### 1.2 Design Principle

The central thesis is: **any game mechanic that involves discrete state changes propagating between entities can be expressed as a graph of StateMachines connected through typed Ports.** Elemental reactions, status effects, automation chains, environmental propagation, combat interactions, quest progression — these are structurally identical under this model.

### 1.3 What Weaven Is Not

Weaven is not a physics engine, a renderer, or a general-purpose programming environment. It is the **discrete event/state layer** that sits between external continuous systems (physics, audio, rendering) and game logic. It consumes external values and emits commands, but does not own continuous simulation.

---

## 2. Core Primitives

### 2.1 State

A node within a StateMachine. Each State has:

- **id**: Unique identifier within its parent SM.
- **context**: A typed data record holding mutable associated data (Extended State Machine model). Examples: `{ remainingDuration: 30, intensity: 5 }`, `{ hp: 100, maxHp: 100 }`.

Context avoids state-count explosion (e.g. HP=100 does not require 100 distinct States).

### 2.2 Transition

A directed edge within a StateMachine. Each Transition has:

- **source**: Origin State.
- **target**: Destination State.
- **priority**: Explicit integer. When multiple Transitions from the same source State have their Guards satisfied simultaneously, the highest-priority Transition fires. Ties at the same priority are treated as a design error; the framework must detect and warn at definition time.
- **guard**: A boolean expression (see §5 Expression Language) evaluated to determine whether the Transition fires. May reference:
  - The SM's current context.
  - Signals present on the SM's Input Ports.
  - **No other SM's state or context** (purity constraint; see §2.7 Interaction Rule).
- **effects**: An ordered list of actions executed upon firing:
  - Context mutations (expressed as assignment expressions within the Expression Language).
  - Signal emissions to Output Ports.
  - System Commands to the Executor (see §7.3).
  - Spawn/Despawn directives.

### 2.3 StateMachine (SM)

The fundamental behavioral unit. Each SM has:

- **states**: A set of States (including exactly one initial State).
- **transitions**: A set of Transitions.
- **ports**: A set of Input Ports, Output Ports, Continuous Input Ports, and Continuous Output Ports.
- **active state**: At runtime, exactly one State is active at any time.
- **elapse capability**: One of `Deterministic`, `Approximate`, or `NonElapsable` (see §4.3).

An Entity may hold **multiple SMs operating in parallel** (see §4 Hierarchy).

### 2.4 Port

The interface through which an SM communicates with the outside world. All cross-SM data flow passes through Ports. There are four kinds:

#### 2.4.1 Input Port

Receives discrete signals from Connections or Interaction Rules.

- **signal type**: A named, typed schema. Example: `ElementIn: { element: ElementType, intensity: number }`.
- **input pipeline**: An ordered sequence of Transform / Filter / Redirect steps applied to incoming signals before they reach the Guard evaluation (see §6 Pipeline).

#### 2.4.2 Output Port

Emits discrete signals when a Transition's Effect fires.

- **signal type**: A named, typed schema.

#### 2.4.3 Continuous Input Port

Binds an external continuous value source to the SM's context. Updated every tick (Phase 1).

- **binding**: A reference to an external value (e.g. `physics.velocity.magnitude`, `gameTime.elapsedSeconds`).
- **target field**: The context field this value writes into.

This preserves Guard purity: the Guard reads `context.speed`, unaware that it originates externally.

#### 2.4.4 Continuous Output Port

Publishes selected context fields and the current active State for external read-only consumption (rendering, audio, UI, network sync).

- **exposed fields**: An explicit whitelist of context field names and the active State identifier.

Only declared fields are visible. Internal implementation details do not leak.

### 2.5 Signal

The data unit flowing between Ports. A Signal has:

- **type**: Matches the Port's declared signal type schema.
- **payload**: Typed data conforming to the schema.

Signals are immutable once emitted. Transformations in Pipelines produce new Signal instances.

### 2.6 Connection

A static binding from one Output Port to one Input Port. Established at design time or entity spawn time. Each Connection has:

- **source**: An Output Port on some SM.
- **target**: An Input Port on some SM.
- **pipeline**: An ordered sequence of Transform / Filter / Redirect steps (see §6). This pipeline represents **world rules** (type effectiveness tables, distance attenuation, elemental reaction multipliers).
- **delay**: Optional. Number of ticks before the signal is delivered. Default: 0 (same-tick delivery within Phase 4).

Static Connections are deterministic and inspectable: the full graph of who-sends-to-whom can be enumerated at any point.

### 2.7 Interaction Rule

A declarative, global pattern-matching rule that detects multi-entity interactions and routes signals accordingly. This is the mechanism for **dynamic, simultaneous cross-entity evaluation** — the role that ECS Systems play, but constrained to detection and routing only.

Each Interaction Rule has:

- **match condition**: A predicate over **two or more SMs** specifying:
  - Port type / active State / context field requirements for each participant.
  - Spatial or logical proximity constraints (e.g. distance < R).
- **result signals**: For each matched participant, a Signal to deliver to a specified Input Port.
- **group**: A namespace for organizational purposes (e.g. `elemental_reactions`, `combat`, `environment`).

Key constraints:

- Interaction Rules are **evaluated only in Phase 2** (see §3). They do not re-evaluate during Phase 4 cascade.
- Interaction Rules have **no side effects** other than enqueuing signals. Actual state changes occur in each SM's own Transitions.
- Interaction Rules are always evaluated under **Server authority** in networked contexts (see §8).

### 2.8 Named Table

A global, read-only, keyed data structure available to the Expression Language (see §5).

- **structure**: A nested map supporting multi-key lookup. Example: `elementReactionTable[Fire][Water] → { reaction: Vaporize, multiplier: 2.0 }`.
- **immutability**: Tables cannot be modified at runtime. Changes are design-time data edits.
- **purpose**: Externalizes combinatorial game-design data (type charts, reaction tables, damage multipliers) from SM definitions, enabling data-driven extension and modding.

---

## 3. Executor — Tick Lifecycle

Each tick is divided into six strictly ordered phases. All operations within a phase complete before the next phase begins.

### Phase 1: Input

1. Update all Continuous Input Port bindings (external values → context fields).
2. Apply player input to relevant SMs.
3. Synchronize external system state (physics engine, game clock, etc.).

### Phase 2: Evaluate

1. Evaluate Guards on all Transitions of all **active SMs** (those in the Active Set).
2. Determine which Transitions fire. Do **not** execute them yet — state is read-only in this phase.
3. Evaluate all Interaction Rule match conditions against current (pre-transition) SM states.
4. Determine which Interaction Rule signals to enqueue. Do **not** deliver them yet.

**Critical invariant**: Phase 2 is a pure read phase. No state mutation occurs.

### Phase 3: Execute

1. Fire all Transitions determined in Phase 2, **simultaneously**. Each SM's active State changes; context mutations and Output Port signal emissions are processed.
2. Enqueue all emitted signals (from Transition Effects and Interaction Rules) into the **signal delivery queue**.

### Phase 4: Propagate

1. Process the signal delivery queue:
   a. Route each signal through its Connection pipeline (or Interaction Rule routing) → Input Port pipeline → deliver to target SM.
   b. Evaluate Guards on the receiving SM's Transitions that reference the receiving Input Port.
   c. Fire any resulting Transitions; enqueue any further emitted signals.
2. Repeat step 1 until the queue is empty **or** the maximum cascade depth is reached.
3. **Interaction Rules are NOT re-evaluated** during this phase. Only Transition-emitted signals cascade. New state combinations produced by cascading are picked up in the next tick's Phase 2.
4. **Time-based Guards are NOT re-evaluated** during cascade. Only signal-triggered Transitions participate.

### Phase 5: Lifecycle

1. **Despawning**: All entities flagged for despawn during Phases 3–4 enter the Despawning state.
   a. Each despawning entity's OnDespawn Transitions are evaluated and fired.
   b. All resulting signals are collected into a **batch queue** (not delivered immediately).
   c. After all despawning entities have been processed, the batch queue is delivered **simultaneously** (order-independent among despawning entities).
   d. Cascade from despawn signals follows Phase 4 rules.
2. **Destroyed**: Despawned entities have all Connections severed and are removed from the spatial index.
3. **Spawning**: New entities flagged for creation during this tick are registered (SMs initialized, static Connections established, spatial index updated). **They do NOT enter the Active Set until the next tick.**

### Phase 6: Output

1. Execute System Commands accumulated during the tick (hit stop, slow motion, etc.).
2. Push state changes to Continuous Output Ports (presentation layer reads these).
3. Emit network synchronization data (see §8).

### Determinism Guarantee

Given identical initial state and identical input sequence, the tick lifecycle produces identical results. This requires:

- Phase execution order is fixed (1 → 2 → 3 → 4 → 5 → 6).
- Signal delivery order within a queue: Static Connection signals before Dynamic Connection (spatial routing / Interaction Rule) signals. Within each category, ordered by source entity ID ascending (lexicographic on ID, content-independent).
- Expression Language evaluation order: strict left-to-right, no algebraic reordering (see §5).
- Floating-point: evaluation order is fixed per above. Frameworks targeting lock-step networking should provide a fixed-point arithmetic option.

---

## 4. Hierarchy and Lifecycle

### 4.1 Compound State (Hierarchical SM)

A State may contain **sub-SMs** that operate in parallel while the parent State is active. This follows the Statecharts model of composite states.

- When the parent State becomes active, its sub-SMs are initialized (or resumed, per Suspend Policy).
- When the parent State is exited (parent Transition fires), sub-SMs are handled per their Suspend Policy.
- Sub-SMs may operate in parallel within the same Compound State.

**An entity holding multiple parallel SMs** is modeled as having an implicit root Compound State containing those SMs. This unifies "entity has multiple SMs" and "State has sub-SMs" into a single mechanism.

### 4.2 Suspend Policy

Each sub-SM within a Compound State declares a Suspend Policy, applied when the parent State is exited:

- **Freeze**: Sub-SM state is preserved in full. Upon re-entry, execution resumes exactly where it left off. Use case: manufacturing progress halted by power outage; mid-conversation dialogue state.
- **Elapse**: Sub-SM's elapsed time is recorded. Upon re-entry, the Elapse Function (see §4.3) is applied with the accumulated duration. Use case: weather progression during battle; crop growth while away.
- **Discard**: Sub-SM is destroyed. Upon re-entry, a fresh instance is initialized from the initial State. Use case: battle-specific SMs after battle ends.

### 4.3 Elapse Function

SMs that declare Elapse capability provide a function `(currentState, context, elapsedTicks) → (newState, newContext)`.

Three capability levels:

- **Deterministic**: SM's transitions depend only on time and internal context. Elapse function is exact. Example: crop growth SM, weather cycle SM.
- **Approximate**: SM has external dependencies, but a reasonable heuristic can be provided by the designer. The framework does not validate accuracy; it is a design-time contract. Example: NPC patrol SM (ignoring collision during elapsed time).
- **NonElapsable**: No Elapse function can be meaningfully defined. Falls back to Freeze. Example: player combat SM, interactive dialogue SM.

### 4.4 Port Promotion

A sub-SM's Output Port can be **promoted** to the parent SM's scope. This allows sub-SM events to serve as Guard conditions for parent-level Transitions.

- Promoted Ports appear as if they were the parent SM's own Output Ports.
- Enables patterns like: "all enemies defeated (sub-SM event) → exit Battle state (parent Transition)."
- Promotion is declared in the Compound State definition, not in the sub-SM itself (the sub-SM remains unaware of its hierarchical context).

### 4.5 Entity Lifecycle

Every entity goes through these phases:

1. **Spawn**: SMs initialized to initial States. Static Connections established from a **Connection Template** declared in the entity definition. Entity registered in spatial index. **Entity becomes active in the next tick** (not the current tick).
2. **Active**: Normal operation within the tick lifecycle.
3. **Despawning**: Triggered by a Transition Effect or external directive. OnDespawn Transitions fire; final signals are batch-delivered (see §3 Phase 5). The entity still exists and is referenceable during this phase.
4. **Destroyed**: All Connections severed. Removed from spatial index and Active Set. Unreferenceable.

**Connection Templates**: When spawning an entity, the spawner can specify Connections to establish. Example: "summoned skeleton's `CommandIn` port connects to summoner's `CommandOut` port." Templates are parameterized by the spawning context.

---

## 5. Expression Language

Guards, Transforms, Filters, context mutations, and Elapse functions are written in a restricted declarative expression language. The language is intentionally constrained to preserve static analyzability, serializability, and data-driven extensibility.

### 5.1 Permitted Constructs

**Literals and References**

- Numeric, string, boolean literals.
- `signal.<field>`: Field access on the received signal.
- `context.<field>`: Field access on the SM's own context.
- `port.<portName>.received`: Boolean — whether a signal arrived on this Port in the current evaluation.

**Operators**

- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `AND`, `OR`, `NOT`
- Conditional: `if <cond> then <expr> else <expr>` (expression-level only, no statements)

**Table Lookup**

- `table.<tableName>[<key1>][<key2>]...`: Named Table reference. Returns the value at the specified keys.

**Limited Collection Operations**

- `<collection>.any(<predicate>)`: Returns `true` if any element satisfies the predicate.
- `<collection>.count(<predicate>)`: Returns the number of elements satisfying the predicate.
- `<collection>.sum(<field>)`: Returns the sum of the named field across all elements.

These always return scalar values. No collection-to-collection transformations.

### 5.2 Prohibited Constructs

- Variable assignment, loops, recursion.
- Arbitrary function calls.
- References to other SMs' state or context (enforced purity; use Interaction Rules).
- Collection mapping, filtering to produce new collections.

### 5.3 Evaluation Order

Strict left-to-right evaluation with no algebraic reordering. This is critical for floating-point determinism in lock-step networked scenarios. `a * b + c` and `c + a * b` may differ in floating-point; the framework guarantees the written order is the executed order.

---

## 6. Signal Pipeline

Signals pass through pipelines between emission and delivery. There are two pipeline stages, applied in order:

### 6.1 Connection-Side Pipeline (World Rules)

Defined on the Connection or Interaction Rule. Represents rules of the game world.

- **Transform**: Modifies signal payload fields. Written as assignment expressions in the Expression Language. Example: `signal.damage = signal.damage * table.typeChart[signal.element][target.type]`. May reference the signal, the target SM's Continuous Output Port values, and Named Tables.
- **Filter**: Boolean expression. If `false`, the signal is blocked. Example: `signal.element != target.context.immuneElement`.
- **Redirect**: If the Filter blocks the signal, optionally re-route it to a different Input Port on the same target entity. Example: fire-type signal blocked by the "Flash Fire" ability → redirected to `AbilityTriggerIn` port.

Steps are applied in declared order as a pipeline.

### 6.2 Input-Port-Side Pipeline (Entity Rules)

Defined on the receiving SM's Input Port. Represents the entity's individual characteristics.

Same three step types (Transform, Filter, Redirect) applied after the Connection-side pipeline completes.

### 6.3 Pipeline Ordering

1. Connection-side Transform(s)
2. Connection-side Filter
3. If filtered: Connection-side Redirect (or discard)
4. Input-Port-side Transform(s)
5. Input-Port-side Filter
6. If filtered: Input-Port-side Redirect (or discard)
7. Signal delivered to Input Port → available for Guard evaluation.

---

## 7. Spatial Routing and Active Set Management

### 7.1 Spatial Routing Layer

Manages dynamic connections based on spatial relationships. Internally maintains a spatial index (quad-tree, grid hash, or equivalent).

Each Output Port may declare an **influence radius** as metadata. The spatial routing layer matches Output Ports of active SMs against Input Ports of nearby SMs within the declared radius, establishing transient connections for the current tick.

Spatial routing results feed into Phase 2 evaluation as Interaction Rule match candidates. The spatial index is updated in Phase 5 when entities spawn, despawn, or move.

### 7.2 Active Set Management

Not all SMs are evaluated every tick. The **Active Set** tracks which SMs require evaluation.

An SM enters the Active Set when:

- A signal is delivered to one of its Input Ports.
- Its Continuous Input Port value changes beyond a defined threshold.
- It is explicitly activated by a Spawn.

An SM exits the Active Set when:

- A tick completes with no Transitions fired and no signals sent.
- The entity is Despawned/Destroyed.

SMs not in the Active Set are **dormant**: they consume no evaluation time, but their state is preserved. Dormant SMs are still present in the spatial index and can be awakened by incoming signals.

This ensures computational cost scales with **active interaction count**, not total entity count. A Minecraft-scale world with millions of block-SMs is tractable because only burning/flowing/powered blocks are active.

### 7.3 System Commands

Special signals emitted by Transition Effects that target the Executor itself rather than other SMs.

- **HitStop**: Pause tick advancement for N frames. Accumulated during Phase 3/4, applied in Phase 6.
- **SlowMotion**: Reduce tick rate by a factor for N ticks.
- **TimeScale**: Adjust the global time delta fed to Continuous Input Ports.

System Commands are processed in Phase 6, after all SM evaluation is complete.

---

## 8. Network Model

Weaven does not mandate a specific network architecture. Instead, it provides per-SM declarative policies that enable lock-step, server-authoritative, and rollback-based networking.

### 8.1 Authority

Each SM declares who has final decision power over its Transitions.

- **Server**: The server (or host) is authoritative. Clients may predict locally but are overridden by server results.
- **Owner**: The client that owns the entity is authoritative. Minimizes input latency for player-controlled SMs (e.g. movement).
- **Local**: Each client evaluates independently. No synchronization. Used for presentation-only SMs (particles, local UI effects).

**Interaction Rules always evaluate under Server authority**, as they reference multiple entities whose owners may differ.

### 8.2 Sync Policy

Each SM declares what data is synchronized over the network.

- **InputSync**: Only input signals are synchronized. All clients run identical simulations from identical input. **Requires full determinism.** Used for lock-step networking (Factorio, RTS).
- **StateSync**: The active State identifier is synchronized. Lightweight. Sufficient for most SMs.
- **ContextSync**: Active State plus selected context fields (matching Continuous Output Port exposed fields) are synchronized.
- **None**: Not synchronized. Local-authority SMs only.

### 8.3 Reconciliation

When a client's predicted state diverges from the authoritative result:

- **Snap**: Immediately overwrite client state with authoritative state. Suitable for discrete state changes where interpolation is meaningless (elemental reaction outcomes, inventory changes).
- **Interpolate**: Blend from current client state toward authoritative state over N ticks. Suitable for continuous-like values (position, displayed HP bar).
- **Rewind**: Roll back to the divergence point and re-simulate with corrected inputs. Most accurate but most expensive. Requires **state snapshots** — the framework provides an SM snapshot/restore API, with configurable history depth.

### 8.4 Interest Management

Determines which SMs' data is sent to which client. Leverages the spatial routing layer:

- Each client has an **interest region** (spatial area, potentially chunked).
- Only SMs within a client's interest region are synchronized to that client.
- SMs outside all clients' interest regions may be dormant on the server as well (if no server-side logic requires them).

**Distinction between sync scope and render scope**: In RTS with fog-of-war, all clients simulate the full map (InputSync), but only render units within their visibility. Sync scope = full map; render scope = fog-limited. This distinction is a presentation-layer concern, not a Weaven-core concern, but the framework exposes the necessary SM state for the presentation layer to implement it.

---

## 9. Relationship to Existing Architectures

### 9.1 vs. Entity-Component-System (ECS)

ECS separates data (Components) from behavior (Systems). Interaction logic resides in global Systems that query entities by component signature.

Weaven differs in three structural ways:

1. **Behavior locality**: In ECS, behavior is external (System). In Weaven, behavior is local (SM Transitions). Each entity knows its own rules.
2. **Interaction topology as first-class data**: In ECS, which entity affects which is implicit in System code. In Weaven, Ports/Connections/Interaction Rules are declarative, inspectable, serializable data.
3. **Hierarchical lifecycle**: ECS is flat. Weaven's Compound States, Suspend Policies, and Port Promotion provide structured lifecycle management.

Interaction Rules are functionally analogous to ECS Systems, but are constrained to **detection and signal routing only** — they cannot directly mutate state. All mutations occur within SM Transitions.

### 9.2 vs. Actor Model

The Actor Model features asynchronous message passing between isolated actors. Weaven differs in that SMs have explicit state-transition graph structure (not opaque message handlers), and connections are typed and declarative (not ad-hoc send targets). Weaven's tick-based execution is synchronous within a tick, unlike Actors' inherent asynchrony.

### 9.3 vs. Statecharts (Standalone)

Harel Statecharts provide hierarchical, concurrent state machines — Weaven's SM model borrows heavily from this. The key extension is that Statecharts do not define a vocabulary for **inter-SM connection**: how one Statechart's events reach another. Weaven's Port/Connection/Interaction Rule layer is the missing piece.

### 9.4 vs. Petri Nets

Petri Nets model concurrency and synchronization via token flow through places and transitions. Weaven shares the focus on declarative interaction structure, but is entity-centric (each SM belongs to an entity) rather than token-centric. Petri Nets lack the concept of typed, directional ports and hierarchical lifecycle management.

---

## 10. Applicability Spectrum

### 10.1 Strong Fit

Games where discrete state changes propagate between entities through structured interactions:

- **Elemental reaction systems** (Genshin Impact, Divinity: Original Sin 2): Element attachment SMs, reaction Interaction Rules, cascade via signal attenuation.
- **Automation/factory games** (Factorio, Satisfactory): Machine SMs connected by belt/pipe Connections. Throughput as signal flow. Power grid as Continuous Input Port.
- **Environmental propagation** (Minecraft redstone, fire spread, water flow): Tile SMs with static neighbor Connections. Attenuation via Connection Transform.
- **Turn-based strategy and RPG** (Chess, Slay the Spire, traditional RPG): Game-phase SM hierarchy. Status effects as SM interactions. Quest progression as hierarchical SM with Suspend Policies.
- **Tower defense**: Enemy path SM + tower attack SM with spatial routing.

### 10.2 Moderate Fit (Requires External System Cooperation)

- **Real-time action** (Dark Souls, Apex Legends): Combat frame data as SM Transitions, but tight coupling with physics/hitbox systems via Continuous I/O Ports. Interaction Rules handle hit detection, but the 1-tick evaluation delay is a structural constraint (mitigable by widening detection windows).
- **Sandbox** (Minecraft-scale): Viable via Active Set Management (dormant block SMs), but requires careful budgeting of active SM count. Elapse functions needed for off-screen chunk simulation.
- **RTS** (StarCraft): Functional via lock-step InputSync, but hundreds of units imply heavy spatial routing evaluation. Influence radius tuning and spatial index efficiency are critical.

### 10.3 Weak Fit (Weaven as Supporting Layer Only)

- **Physics-dominant games** (Kerbal Space Program, Angry Birds): Continuous simulation is the core mechanic. Weaven handles only discrete decision points (stage separation, parachute deployment) — a thin layer over the physics engine.
- **Rhythm games**: Input-timing evaluation against a score timeline is better modeled as stream processing than state machines.

---

## 11. Open / Undecided Items

### 11.1 Expression Language Formal Specification

The permitted constructs are defined (§5), but a formal grammar (BNF or equivalent) has not been written. Needed before implementation.

### 11.2 Interaction Rule Optimization

Interaction Rules perform N×M matching across active SMs. For large active sets, this becomes a performance concern. Potential strategies include:

- Pre-filtering by Port type signature (only match SMs that have compatible Ports).
- Spatial bucketing (only match within spatial cells).
- Dirty-flag optimization (only re-evaluate rules involving SMs whose state changed since last tick).

None selected yet.

### 11.3 Debugging and Visualization Tooling

The declarative, inspectable nature of the connection topology is a major advantage. Specific tooling design is deferred:

- Real-time topology graph visualization (SMs as nodes, Connections as edges).
- Signal flow tracing (follow a signal from emission through pipelines to delivery).
- Cascade replay (step through Phase 4 propagation).
- Guard evaluation inspector (why did/didn't this Transition fire).

### 11.4 Serialization Format

SM definitions, Connection Templates, Interaction Rules, and Named Tables are all data. A serialization format (JSON, binary, custom DSL) has not been chosen. This affects modding support, hot-reloading, and tooling.

### 11.5 Error Handling

- What happens when a Connection references a destroyed entity's Port? (Currently: Connection is severed at Destroy phase. But what about in-flight signals?)
- What happens when an Elapse function produces an invalid state? (Currently: undefined.)
- Maximum cascade depth exceeded — what is the recovery behavior? (Currently: stop propagation, log warning. But should affected SMs be rolled back?)

### 11.6 Concurrency / Multi-threading

The tick phases are sequential, but within Phase 2 (evaluation), Guards on independent SMs could be evaluated in parallel. Similarly, Phase 4 cascade branches that do not share target SMs could parallelize. Threading strategy is deferred to implementation phase.

### 11.7 Sync Scope vs. Render Scope Formalization

Identified in the RTS analysis (§8.4). The framework exposes SM state; the presentation layer decides what to render. But the boundary and API between Weaven core and presentation layer for visibility-filtered rendering is not formally specified.

---

## 12. System Architecture

### 12.1 Positioning

Weaven is **not** a game engine. It is a **portable game logic runtime library** — the discrete event/state layer that game engines consume. It does not own rendering, physics, audio, input abstraction, asset management, or scene graphs. It provides the declarative state-interaction substrate that those systems read from and write to.

### 12.2 Component Structure

The system is composed of four distinct components:

**Weaven Core** — The engine-independent pure logic library. Stateless between ticks; all state is explicitly held in SM data structures. Responsibilities: SM evaluation, signal propagation and cascade, Interaction Rule matching, Expression Language evaluation, Named Table lookup, Active Set management, State Diff computation, Snapshot/Restore.

**Weaven Spatial** (optional module) — Spatial index and spatial routing layer. Provides Interaction Rule spatial match conditions and dynamic connection resolution. Can be omitted when the host engine's spatial query system is used instead (see §12.3 Tier Model).

**Weaven Schema** — The data format for SM definitions, Connections, Interaction Rules, Named Tables, and Expression Language expressions. Initially JSON; a custom DSL is a future option. Schema files are the primary artifact game designers author and version-control.

**Weaven Adapter** — A thin per-engine integration layer. Written in the host engine's language. Bridges engine frame loop, physics, input, rendering, audio, and networking to Weaven Core's Port-based API.

### 12.3 Tier Model

Weaven is deployed in one of three tiers. Higher tiers include all lower-tier capabilities.

**Tier 1: Core Only**

SM evaluation, signal propagation, Interaction Rule evaluation (without spatial conditions), Expression Language, Named Tables. Spatial query results and network inputs are injected externally by the Adapter.

Suitable for: factory/automation games (static Connections only), turn-based games, status effect systems embedded in existing engines.

The Adapter is minimal — call `tick()` per frame, push values into Continuous Input Ports, read Continuous Output Ports and Output Port events.

**Tier 2: Core + Spatial**

Adds the Weaven Spatial module. Weaven owns its spatial index and evaluates spatial match conditions for Interaction Rules internally.

Suitable for: games without a heavy physics engine (2D action, tower defense), or custom engine stacks (Rust + Rapier + wgpu) where no conflicting spatial index exists.

When the host engine also maintains a spatial index (Unity Physics, Godot PhysicsServer), dual management occurs. In such cases, Tier 1 with external spatial query injection is recommended to avoid redundancy.

**Tier 3: Core + Spatial + Network (Future)**

Adds a full network transport and reconciliation implementation. Weaven owns packet serialization, send/receive, and Rewind execution. Not on the initial implementation roadmap.

In Tier 1 and Tier 2, network integration works as follows:

- Weaven provides **metadata**: per-SM Authority, Sync Policy, and Reconciliation declarations.
- Weaven provides **APIs**: State Diff (which SMs changed, which fields), Snapshot/Restore (serialize/deserialize SM state), Input Injection (deliver inputs tagged with tick numbers).
- The Adapter or engine's network layer uses these APIs to implement actual synchronization. Weaven decides *what* to sync; the engine decides *how* and *when*.

### 12.4 Adapter API Surface

The Adapter interacts with Weaven Core through a boundary defined entirely by Ports:

**Inbound (Engine → Weaven)**:
- `tick()`: Advance one tick. The Adapter calls this once per frame (or per simulation step).
- Continuous Input Port writes: Push external values (physics state, game clock, player input) into bound context fields.
- Signal injection: Deliver discrete signals to Input Ports (input events, external system events).
- Spatial query results (Tier 1 only): Provide match candidates for Interaction Rules with spatial conditions.

**Outbound (Weaven → Engine)**:
- Continuous Output Port reads: Presentation layer polls or subscribes to exposed state/context fields.
- Output Port event stream: Discrete signals emitted by Transitions, consumed by rendering/audio/UI.
- System Commands: Hit stop, slow motion, time scale directives.
- State Diff: Per-tick delta of changed SMs/fields, consumed by network layer.
- Snapshot/Restore: Serialized SM state for Rewind reconciliation.

### 12.5 Engine Integration Profiles

**Unity**: Weaven Core compiled as a native plugin (C shared library via FFI) or IL2CPP-compatible managed wrapper. Adapter written in C#. `MonoBehaviour.Update` or `FixedUpdate` calls `tick()`. `Rigidbody` values bound to Continuous Input Ports. Continuous Output Ports drive Animator parameters, VFX Graph properties, UI bindings. Unity Netcode consumes State Diff API for multiplayer.

**Godot**: Weaven Core as a GDExtension (C shared library). Adapter in GDScript or C++. `_physics_process` calls `tick()`. Node properties bound to Ports. Godot's Signal system bridges to Weaven Output Port events via Adapter translation.

**Bevy**: Weaven Core as a Rust crate, consumed directly — no FFI boundary. Adapter is a Bevy plugin (set of Systems). Bevy Components hold SM context data; Bevy Events bridge to Weaven Signals. Most natural integration of all engines due to shared language and ECS affinity.

**Custom Stack (Rust + Rapier + wgpu)**: Weaven Core as a crate. Tier 2 with Weaven Spatial. No Adapter abstraction needed; direct API calls in the game loop. Full control over all layers.

**Browser (Phaser / Custom)**: Weaven Core compiled to WASM via `wasm-pack`. Adapter in TypeScript/JavaScript. `requestAnimationFrame` loop calls `tick()`. Lightweight 2D games and prototypes.

**Love2D**: Weaven Core as a native shared library called via LuaJIT FFI. Adapter in Lua. `love.update(dt)` calls `tick()`.

---

## 13. Implementation Strategy

### 13.1 Implementation Language

Weaven Core is implemented in **Rust**. This choice is driven by the intersection of all deployment requirements:

- **FFI**: Rust exposes C ABI directly. Required for Unity (native plugin), Godot (GDExtension), Love2D (LuaJIT FFI), and any C-callable host.
- **WASM**: Rust's WASM toolchain (`wasm-pack`, `wasm-bindgen`) is mature. Required for browser deployment.
- **No GC**: Deterministic evaluation (§3) requires that memory management never introduces non-deterministic pauses or ordering. Rust's ownership model guarantees this.
- **Memory layout control**: Snapshot/Restore (§8.3) benefits from POD-like structures that can be efficiently serialized as byte copies. Rust's `repr(C)` and lack of hidden runtime state make this straightforward.
- **Expression evaluation order**: Rust does not perform algebraic reordering of floating-point operations. Combined with the fixed evaluation order specified in §5.3, this guarantees deterministic results across platforms.
- **Bevy compatibility**: Bevy is Rust-native; Weaven Core integrates as a crate dependency with zero FFI overhead.

Weaven Adapters are written in the host engine's language (C# for Unity, GDScript/C++ for Godot, Rust for Bevy, TypeScript for browser, Lua for Love2D).

Weaven Schema is initially JSON. A custom DSL is a future consideration (see §11.4).

### 13.2 Build Pipeline — oxidtr Integration

[oxidtr](https://github.com/penta2himajin/oxidtr) is adopted as the formal specification and multi-language code generation backbone of the Weaven build pipeline. oxidtr takes Alloy (`.als`) formal models as the single source of truth and deterministically generates type definitions, invariant validation functions, property tests, and structural consistency checks across multiple target languages.

#### 13.2.1 Role of Alloy in Weaven

A formal Alloy model (`weaven.als`) defines the structural specification of all Weaven primitives:

- **Sig declarations** for State, Transition, StateMachine, Port (all four kinds), Signal, Connection, Interaction Rule, Named Table, Pipeline, PipelineStep, Compound State, Entity, and all associated enums (SuspendPolicy, Authority, SyncPolicy, Reconciliation, etc.).
- **Field declarations** with multiplicity constraints capturing the relationships defined in §2 (e.g., a Transition has `one` source State, `one` target State, `set` Effects).
- **Fact declarations** encoding structural invariants: no cyclic SM hierarchy, unique Transition priorities per source State, Port type compatibility on Connections, Connection pipeline type preservation.
- **Assert declarations** for safety properties: determinism of signal delivery order, cascade termination guarantee, lifecycle phase ordering.
- **Pred declarations** for operations: `fireTick`, `deliverSignal`, `evaluateInteractionRule`, `spawnEntity`, `despawnEntity`.

This model serves as the canonical, language-independent definition of "what Weaven is" at the structural level.

#### 13.2.2 What oxidtr Generates

From `weaven.als`, oxidtr generates for each target language:

| Artifact | Description | oxidtr Feature |
|----------|-------------|----------------|
| Type definitions | Structs/classes/interfaces for all Weaven primitives | `generate` — sig → struct/class/interface |
| Multiplicity-aware fields | `Set<Port>`, `Option<State>`, `Vec<Effect>` etc. with correct collection types | `generate` — `set`/`lone`/`seq` → language-idiomatic types |
| Invariant validators | Executable boolean functions for every Alloy fact | `generate` — fact → validator function with `@alloy:` comment |
| Property tests | Test cases from assert declarations with fixture data | `generate` — assert → test + fixtures |
| Cross-tests | Fact × predicate preservation tests with boundary values | `generate` — cross-test scaffolding |
| Doc comments | `@pre`/`@post` conditions, constraint documentation | `generate` — doc comments with original Alloy syntax |
| Newtypes (Rust) | `TryFrom` validated wrappers with range checks from cardinality bounds | `generate --target rust` — newtype generation |
| JSON Schema | Structural schema for Weaven Schema serialization format | `generate` — JSON Schema with min/max/unique constraints |
| Bean Validation (JVM) | `@NotNull`, `@Size` annotations for JVM Adapter types | `generate --target kt/java` — annotation generation |
| Serde derives (Rust) | `Serialize`/`Deserialize` for Snapshot/Restore | `generate --target rust --features serde` |

#### 13.2.3 Structural Consistency Verification

oxidtr's `check` command is integrated into CI to enforce that implementations conform to the Alloy specification:

```
oxidtr check --model weaven.als --impl weaven-core/src/       # Rust Core
oxidtr check --model weaven.als --impl adapters/unity/         # C# Adapter types
oxidtr check --model weaven.als --impl adapters/godot/         # C++/GDScript types
oxidtr check --model weaven.als --impl adapters/browser/       # TypeScript types
```

Detectable divergences: `MISSING_STRUCT`, `EXTRA_STRUCT`, `MISSING_FIELD`, `EXTRA_FIELD`, `MULTIPLICITY_MISMATCH`, `MISSING_FN`, `EXTRA_FN`. Any divergence fails the CI gate, ensuring all Adapters stay synchronized with the Core specification.

When the Alloy model is updated (e.g., a new Port kind is added), `check` immediately identifies all Adapters that need corresponding updates.

#### 13.2.4 Round-Trip and Extract

oxidtr's `extract` command enables reverse engineering and migration:

- **Adapter audit**: Extract an Alloy model draft from an Adapter's source code and compare against `weaven.als` to detect undocumented divergences.
- **Game logic migration**: Extract SM-like patterns from existing game code (in any supported language) to produce Weaven Schema drafts, lowering the barrier to adoption.
- **Multi-language merge**: When Adapters in different languages are extracted simultaneously, oxidtr merges and reports conflicts — ensuring cross-language structural consistency.

The lossless round-trip property (`weaven.als → generate → extract → structural match`) is verified as part of CI.

#### 13.2.5 Build Pipeline Architecture

```
weaven.als (Alloy formal specification — single source of truth)
    │
    │  oxidtr generate
    ├──────────────────────────────────────────────────────────────
    │        │              │            │           │           │
    │   --target rust  --target ts  --target kt  --target cs  --target swift
    │        │              │            │           │           │
    │   Weaven Core    Browser      Android     Unity        iOS
    │   types +        Adapter      Adapter     Adapter      Adapter
    │   validators     types        types       types        types
    │   + newtypes     + JSON       + Bean      + JSON       + tests
    │   + serde        Schema       Validation  Schema
    │   + tests
    │
    │  oxidtr check (CI gate)
    ├──────────────────────────────────────────────────────────────
    │   --model weaven.als --impl <each target directory>
    │   → 0 structural diffs required for CI pass
    │
    │  oxidtr extract (audit / migration)
    ├──────────────────────────────────────────────────────────────
    │   extract from impl → compare with weaven.als
    │   extract from existing game code → Weaven Schema draft
    │
    │  Game developer workflow
    ├──────────────────────────────────────────────────────────────
    │   Author SM definitions, Interaction Rules, Named Tables
    │   in Weaven Schema (JSON) or future DSL
    │   → Weaven Core loads and validates at startup
    │   → Invariant validators (oxidtr-generated) run on Schema load
    │
    │  Runtime
    ├──────────────────────────────────────────────────────────────
    │   Engine frame loop
    │     → Adapter pushes external values into Continuous Input Ports
    │     → Adapter calls Weaven Core tick()
    │     → Adapter reads Continuous Output Ports, Output Port events,
    │       System Commands, State Diffs
    │     → Engine renders, plays audio, sends network packets
```

#### 13.2.6 oxidtr Roadmap Alignment

| oxidtr Capability | Status | Weaven Usage |
|---|---|---|
| Rust backend | Complete | Weaven Core types, validators, tests, newtypes, serde |
| TypeScript backend | Complete | Browser Adapter types, JSON Schema |
| Kotlin backend | Complete | Android Adapter types, Bean Validation |
| Java backend | Complete | JVM server types (if applicable) |
| Swift backend | Complete | iOS Adapter types |
| Go backend | Complete | Server-side types (if applicable) |
| C# backend | Planned (Phase 9) | **Unity Adapter types** — critical path |
| Lean backend (polarstar) | Planned (Phase 10) | Future formal verification of Weaven invariants |
| Alloy 6 temporal | Complete | Temporal property specification for SM lifecycle |
| `check` command | Complete | CI structural consistency gate |
| `extract` command | Complete | Adapter audit, migration tooling |
| JSON Schema generation | Complete | Weaven Schema validation |
| Multi-language merge | Complete | Cross-Adapter consistency verification |

The C# backend (Phase 9) is on the critical path for Unity Adapter support. Until then, Unity Adapter types are manually maintained with `check` validation against the Alloy model deferred.

### 13.3 Expression Language Implementation

The Expression Language (§5) is implemented as follows:

- **AST**: Defined as Rust enums within Weaven Core. The AST structure is also modeled in `weaven.als` and type-checked by oxidtr.
- **Parser**: Hand-written recursive descent parser in Rust (no external parser dependencies, consistent with oxidtr's "minimal dependencies" philosophy).
- **Evaluator**: Tree-walking interpreter with strict left-to-right evaluation. No JIT compilation. Determinism over performance — sufficient for the Expression Language's restricted scope.
- **Serialization**: AST serializes to/from JSON as part of Weaven Schema. Future DSL compilation targets the same AST.

### 13.4 Development Phases

**Phase 1**: Weaven Core (Tier 1). SM evaluation, signal propagation, cascade, Expression Language evaluator, Named Tables, Active Set management. `weaven.als` model and oxidtr pipeline. JSON Schema for SM definitions. Validation: property tests from oxidtr, plus manual integration tests with a simple tick-driven test harness.

**Phase 2**: Weaven Spatial (Tier 2). Spatial index (grid hash), spatial routing, spatial Interaction Rule conditions. Validation: environmental cascade scenario (Appendix A).

**Phase 3**: First Adapter. Bevy Adapter (Rust-native, no FFI complexity). Proof-of-concept game demonstrating elemental reactions, environmental propagation, and entity lifecycle.

**Phase 4**: WASM build + browser Adapter. TypeScript Adapter for Phaser or standalone Canvas. Enables rapid prototyping and Weaven Editor frontend.

**Phase 5**: Unity Adapter. Requires oxidtr C# backend (Phase 9) or manual type maintenance with deferred `check`. Native plugin build. Proof-of-concept in Unity project.

**Phase 6**: Network APIs. State Diff, Snapshot/Restore, Input Injection APIs on Weaven Core. Adapter-side integration with Bevy's networking and Unity Netcode.

**Phase 7**: Weaven Editor prototype. Browser-based visual editor for SM definitions, Connection topology, Interaction Rules. Reads/writes Weaven Schema. Uses WASM Weaven Core for live validation and simulation preview.

---

## Appendix A: Validation Scenario — Environmental Cascade

**Setup**: Player character (PC) with `ElementalStatus SM` in `Burning` state stands on grass tile T1. T1 has static Connections to adjacent tiles T2, T3. Weather SM transitions to `Raining`.

**Tick N execution**:

| Phase | Action |
|-------|--------|
| Phase 1 | Weather SM's Continuous Input Port updates game clock. |
| Phase 2 | Weather SM: Guard `clock >= rainStartTime` satisfied → Transition `Clear → Raining` determined. Interaction Rule: PC `Burning` + T1 `Grass` + spatial overlap → fire signal to T1 determined. |
| Phase 3 | Weather: `Clear → Raining` executed. Fire signal `{ element: Fire, intensity: 5 }` enqueued to T1's `ElementIn`. |
| Phase 4 | T1 receives fire signal → Guard satisfied → `Grass → Burning` transition fires. T1 emits `{ element: Fire, intensity: 4 }` (Connection Transform: `intensity -= 1`) to T2, T3. T2: `Grass → Burning`, emits `{ Fire, intensity: 3 }` to its neighbors. Cascade continues until `intensity == 0`. Interaction Rule `Raining × Burning → extinguish` is **NOT** re-evaluated (Phase 2 only). |
| Phase 5 | No spawns/despawns. |
| Phase 6 | Continuous Output Ports updated. Presentation layer sees T1, T2, etc. as `Burning`. |

**Tick N+1 execution**:

| Phase | Action |
|-------|--------|
| Phase 2 | Interaction Rule: `Raining` + all `Burning` entities → extinguish signals determined. PC, T1, T2, etc. all matched. |
| Phase 3 | Extinguish signals delivered. `Burning → Grass` (tiles), `Burning → Wet` (PC). |

Result: Fire spreads in one tick, then rain extinguishes in the next. Causality is clear and deterministic.

---

## Appendix B: Validation Scenario — Parry (Combat)

**Setup**: Enemy `Attack SM` in `WindUp` state, timer about to expire. PC inputs parry.

**Tick N**:

| Phase | Action |
|-------|--------|
| Phase 1 | PC parry input registered. |
| Phase 2 | Enemy: `WindUp → ActiveFrame` determined. PC: `Idle → Parry` determined. Interaction Rule: `ActiveFrame AND Parry AND proximity` evaluated — **does not match** (both still in pre-transition states). |
| Phase 3 | Enemy → `ActiveFrame`. PC → `Parry`. |

**Tick N+1**:

| Phase | Action |
|-------|--------|
| Phase 2 | Interaction Rule: Enemy `ActiveFrame` AND PC `Parry` AND proximity → **match**. Signals: `StaggerIn` to Enemy, `ParrySuccessIn` to PC. |
| Phase 3 | Enemy: `ActiveFrame → Staggered`. PC: `Parry → Riposte`. |

1-tick delay is structural. At 60 FPS this is ~16ms. Designers compensate by widening the Parry window or the ActiveFrame duration by 1 frame.

---

## Appendix C: Validation Scenario — Entity Lifecycle (Summoning + Death Explosion)

**Setup**: Necromancer summons a skeleton. Skeleton has an `OnDespawn` Transition that emits an explosion signal.

**Tick N**: Necromancer's Transition Effect issues Spawn directive for Skeleton with Connection Template `{ Necromancer.CommandOut → Skeleton.CommandIn }`.

**Tick N, Phase 5**: Skeleton entity created. SMs initialized. Connection established. Registered in spatial index. **Not in Active Set until Tick N+1.**

**Tick N+1**: Skeleton is active. Receives commands via `CommandIn`.

**Tick M**: Skeleton's HP reaches 0.

**Tick M, Phase 3**: Skeleton's `Health SM` transitions to `Dead`. Despawn flagged.

**Tick M, Phase 5**:

1. Skeleton enters Despawning. OnDespawn Transition fires, emitting `{ explosion, damage: 50, radius: 3 }` into the batch queue.
2. (If other entities despawn simultaneously, their signals also enter the batch queue.)
3. Batch queue delivered simultaneously. Nearby enemies receive explosion damage signals. **Order among multiple despawning entities' signals does not affect results** because delivery is batched.
4. Skeleton Destroyed. Connections severed. Removed from spatial index.

---

## Appendix D: Glossary

| Term | Definition |
|------|-----------|
| **SM** | StateMachine. The fundamental behavioral unit holding States, Transitions, and Ports. |
| **State** | A node in an SM. Has an ID and a typed context (mutable data). |
| **Transition** | A directed edge in an SM. Has source, target, priority, Guard, and Effects. |
| **Guard** | A boolean expression determining whether a Transition fires. |
| **Effect** | An action executed upon Transition firing: context mutation, signal emission, system command, or spawn/despawn directive. |
| **Port** | An SM's external interface. Four kinds: Input, Output, Continuous Input, Continuous Output. |
| **Signal** | A typed, immutable data unit flowing between Ports. |
| **Connection** | A static binding from an Output Port to an Input Port, with an optional pipeline. |
| **Pipeline** | An ordered sequence of Transform, Filter, and Redirect steps applied to a Signal in transit. |
| **Interaction Rule** | A global, declarative pattern-matching rule that detects multi-entity state combinations and routes signals. |
| **Named Table** | A global, read-only keyed data structure for combinatorial game-design data. |
| **Active Set** | The set of SMs evaluated each tick. Dormant SMs are excluded. |
| **Compound State** | A State containing parallel sub-SMs (hierarchical SM model). |
| **Suspend Policy** | Per-sub-SM declaration of behavior when parent State exits: Freeze, Elapse, or Discard. |
| **Elapse Function** | A function `(state, context, ticks) → (state, context)` for fast-forwarding a dormant SM. |
| **Port Promotion** | Exposing a sub-SM's Output Port at the parent SM's scope. |
| **Connection Template** | A parameterized Connection specification applied at entity Spawn time. |
| **Authority** | Per-SM declaration of who has final decision power in networked contexts: Server, Owner, or Local. |
| **Sync Policy** | Per-SM declaration of what data is network-synchronized: InputSync, StateSync, ContextSync, or None. |
| **Reconciliation** | Per-SM strategy for resolving client-server prediction divergence: Snap, Interpolate, or Rewind. |
| **System Command** | A special signal targeting the Executor itself (hit stop, slow motion, etc.). |
| **Weaven Core** | The engine-independent pure logic library. Rust implementation. Handles SM evaluation, signals, cascades, Expression Language. |
| **Weaven Spatial** | Optional spatial index module. Provides Interaction Rule spatial matching and dynamic connection routing. |
| **Weaven Schema** | The data format (JSON / future DSL) for SM definitions, Connections, Interaction Rules, Named Tables. The primary artifact game designers author. |
| **Weaven Adapter** | A thin per-engine integration layer bridging engine systems to Weaven Core's Port-based API. Written in the host engine's language. |
| **Tier 1** | Deployment mode: Core only. Spatial queries and network inputs injected externally. |
| **Tier 2** | Deployment mode: Core + Spatial. Weaven owns spatial index. |
| **Tier 3** | Deployment mode (future): Core + Spatial + Network. Weaven owns transport and reconciliation. |
| **oxidtr** | Alloy-to-multi-language code generation and verification toolchain. Generates Weaven types, validators, and tests from `weaven.als`. |
| **weaven.als** | The Alloy formal specification serving as the single source of truth for all Weaven primitive structures and invariants. |
| **State Diff** | Per-tick delta of changed SM states/fields. Consumed by the network layer for synchronization. |
