/// Runtime type definitions for Weaven Core.
///
/// The generated `models.rs` captures the structural shape from `weaven.als`.
/// These types implement the actual runtime representation using stable IDs,
/// value-typed context, and owned collections that can be mutated during ticks.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

// ---------------------------------------------------------------------------
// Stable IDs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct SmId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct StateId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct TransitionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct PortId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalTypeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ConnectionId(pub u32);

// ---------------------------------------------------------------------------
// Signal
// ---------------------------------------------------------------------------

/// A typed, immutable data unit flowing between Ports.
/// Payload is a simple key-value map for this skeleton.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub signal_type: SignalTypeId,
    pub payload: BTreeMap<String, f64>,
}

// ---------------------------------------------------------------------------
// Port (runtime)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortKind {
    Input,
    Output,
    ContinuousInput,
    ContinuousOutput,
}

#[derive(Debug)]
pub struct Port {
    pub id: PortId,
    pub kind: PortKind,
    pub signal_type: SignalTypeId,
    /// Input-Port-side pipeline (§6.2, §6.3 steps 4–6).
    pub input_pipeline: Vec<PipelineStep>,
    /// Influence radius for spatial routing (§7.1). Output Ports only.
    /// When set, the spatial routing layer matches nearby SMs within this radius.
    pub influence_radius: Option<f64>,
}

impl Port {
    /// Convenience constructor with empty input pipeline and no influence radius.
    pub fn new(id: PortId, kind: PortKind, signal_type: SignalTypeId) -> Self {
        Self { id, kind, signal_type, input_pipeline: vec![], influence_radius: None }
    }

    /// Constructor with influence radius (for spatial output ports).
    pub fn with_radius(id: PortId, kind: PortKind, signal_type: SignalTypeId,
                       radius: f64) -> Self {
        Self { id, kind, signal_type, input_pipeline: vec![], influence_radius: Some(radius) }
    }
}

// ---------------------------------------------------------------------------
// Transition (runtime)
// ---------------------------------------------------------------------------

/// Determines how an SM handles time passage while suspended (§4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElapseCapabilityRt {
    /// Elapse function is exact. SM can fast-forward deterministically.
    Deterministic,
    /// Elapse function is approximate (designer-provided heuristic).
    Approximate,
    /// No meaningful elapse function. Falls back to Freeze.
    NonElapsable,
}

/// Elapse function signature: given the current state, context, and the number
/// of ticks that elapsed while suspended, return the new (state, context).
pub type ElapseFn =
    Box<dyn Fn(StateId, &Context, u64) -> (StateId, Context) + Send + Sync>;



/// A guard is a closure over the SM's context and the received signal.
pub type GuardFn = Box<dyn Fn(&Context, Option<&Signal>) -> bool + Send + Sync>;

// ---------------------------------------------------------------------------
// System Commands (§7.3)
// ---------------------------------------------------------------------------

/// Special commands targeting the Executor itself, emitted by Transition Effects.
/// Accumulated during Phase 3/4 and applied in Phase 6.
#[derive(Debug, Clone)]
pub enum SystemCommand {
    /// Pause tick advancement for N frames (sub-frame freeze for hit feel).
    HitStop { frames: u32 },
    /// Reduce tick rate by `factor` for `duration_ticks` ticks.
    /// factor=0.5 means half speed.
    SlowMotion { factor: f64, duration_ticks: u32 },
    /// Adjust the global time delta fed to Continuous Input Ports.
    TimeScale(f64),
}

/// Output produced by a Transition Effect (§2.2).
pub enum EffectOutput {
    /// Emit a signal to an Output Port.
    Signal(PortId, Signal),
    /// Issue a System Command to the Executor.
    Cmd(SystemCommand),
}

/// An effect mutates the SM context and/or produces EffectOutputs.
pub type EffectFn = Box<dyn Fn(&mut Context) -> Vec<EffectOutput> + Send + Sync>;

pub struct Transition {
    pub id: TransitionId,
    pub source: StateId,
    pub target: StateId,
    /// Higher priority fires first when multiple guards pass simultaneously.
    pub priority: u32,
    /// None means the transition fires unconditionally when source is active.
    pub guard: Option<GuardFn>,
    pub effects: Vec<EffectFn>,
}

impl std::fmt::Debug for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transition")
            .field("id", &self.id)
            .field("source", &self.source)
            .field("target", &self.target)
            .field("priority", &self.priority)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Context (extended state)
// ---------------------------------------------------------------------------

/// The mutable data record associated with a State.
/// Supports both scalar f64 fields and array fields (for collection operations §5.1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    pub scalars: BTreeMap<String, f64>,
    /// Array fields: each element is a record (map of field→value).
    /// Used by `any()`, `count()`, `sum()` collection operations.
    pub arrays: BTreeMap<String, Vec<BTreeMap<String, f64>>>,
}

impl Context {
    pub fn get(&self, key: &str) -> f64 {
        *self.scalars.get(key).unwrap_or(&0.0)
    }

    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.scalars.insert(key.into(), value);
    }

    /// Set an array field (for collection operations).
    pub fn set_array(&mut self, key: impl Into<String>, value: Vec<BTreeMap<String, f64>>) {
        self.arrays.insert(key.into(), value);
    }

    /// Get an array field (empty slice if not present).
    pub fn get_array(&self, key: &str) -> &[BTreeMap<String, f64>] {
        self.arrays.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

// ---------------------------------------------------------------------------
// StateMachine definition (design-time)
// ---------------------------------------------------------------------------

pub struct SmDef {
    pub id: SmId,
    pub states: BTreeSet<StateId>,
    pub initial_state: StateId,
    pub transitions: Vec<Transition>,
    pub input_ports: Vec<Port>,
    pub output_ports: Vec<Port>,
    /// Transitions fired when this SM's entity is despawned (§4.5 Phase 5).
    pub on_despawn_transitions: Vec<Transition>,
    /// How this SM handles time passage while suspended (§4.3).
    pub elapse_capability: ElapseCapabilityRt,
    /// Optional elapse function for Deterministic/Approximate SMs.
    /// `None` means fall back to Freeze regardless of elapse_capability.
    pub elapse_fn: Option<ElapseFn>,
}

impl std::fmt::Debug for SmDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmDef")
            .field("id", &self.id)
            .field("states", &self.states)
            .field("initial_state", &self.initial_state)
            .field("elapse_capability", &self.elapse_capability)
            .finish()
    }
}

impl SmDef {
    /// Convenience constructor. `on_despawn_transitions` defaults to empty;
    /// set it explicitly when the entity needs a death reaction (§4.5).
    pub fn new(
        id: SmId,
        states: impl IntoIterator<Item = StateId>,
        initial_state: StateId,
        transitions: Vec<Transition>,
        input_ports: Vec<Port>,
        output_ports: Vec<Port>,
    ) -> Self {
        Self {
            id,
            states: states.into_iter().collect(),
            initial_state,
            transitions,
            input_ports,
            output_ports,
            on_despawn_transitions: vec![],
            elapse_capability: ElapseCapabilityRt::NonElapsable,
            elapse_fn: None,
        }
    }
}

// ---------------------------------------------------------------------------
// StateMachine instance (runtime)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct SmInstance {
    pub def_id: SmId,
    pub active_state: StateId,
    pub context: Context,
    /// Signals buffered into this SM's input ports this tick.
    pub pending_signals: Vec<(PortId, Signal)>,
}

impl SmInstance {
    pub fn new(def: &SmDef) -> Self {
        Self {
            def_id: def.id,
            active_state: def.initial_state,
            context: Context::default(),
            pending_signals: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Signal Pipeline (§6)
// ---------------------------------------------------------------------------

/// A single step in a Connection-side or Input-Port-side pipeline.
///
/// Steps are applied in declared order: Transform → Filter → Redirect.
/// Transforms produce a new Signal (immutable once emitted).
/// A Filter returning false drops the signal (or triggers a Redirect if one follows).
pub enum PipelineStep {
    /// Modifies signal payload fields. Returns a new Signal with the mutation applied.
    Transform(Box<dyn Fn(Signal) -> Signal + Send + Sync>),
    /// Boolean predicate. If false, the signal is blocked (dropped or redirected).
    Filter(Box<dyn Fn(&Signal) -> bool + Send + Sync>),
    /// If the preceding Filter blocked the signal, re-route to this port instead.
    /// If no Filter blocked, this step is a no-op.
    Redirect(PortId),
}

impl std::fmt::Debug for PipelineStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineStep::Transform(_) => write!(f, "PipelineStep::Transform(..)"),
            PipelineStep::Filter(_)    => write!(f, "PipelineStep::Filter(..)"),
            PipelineStep::Redirect(p)  => write!(f, "PipelineStep::Redirect({p:?})"),
        }
    }
}

/// Apply a pipeline to a signal. Returns `None` if filtered (and no redirect),
/// or `Some((port_override, signal))` where `port_override` is `Some` only when
/// a Redirect fired.
pub fn apply_pipeline(
    steps: &[PipelineStep],
    mut signal: Signal,
    default_port: PortId,
) -> Option<(PortId, Signal)> {
    let mut blocked = false;
    let mut redirect_port: Option<PortId> = None;

    for step in steps {
        match step {
            PipelineStep::Transform(f) => {
                if !blocked {
                    signal = f(signal);
                }
            }
            PipelineStep::Filter(pred) => {
                if !blocked && !pred(&signal) {
                    blocked = true;
                }
            }
            PipelineStep::Redirect(port) => {
                if blocked {
                    redirect_port = Some(*port);
                    blocked = false; // redirect un-blocks the signal
                }
            }
        }
    }

    if blocked {
        None
    } else {
        Some((redirect_port.unwrap_or(default_port), signal))
    }
}

// ---------------------------------------------------------------------------
// Compound State / Hierarchy (§4)
// ---------------------------------------------------------------------------

/// Design-time definition of a Compound State.
/// Attached to a specific StateId within a parent SM.
#[derive(Debug)]
pub struct CompoundStateDef {
    /// The State that "contains" these sub-SMs.
    pub parent_state: StateId,
    /// The SM that owns parent_state.
    pub parent_sm: SmId,
    /// Sub-SMs that run in parallel while parent_state is active.
    pub sub_machines: Vec<SmId>,
    /// What happens to sub-SMs when the parent State is exited.
    pub suspend_policy: SuspendPolicyRt,
    /// Sub-SM Output Ports promoted to the parent SM's scope (§4.4).
    /// Guards on parent-level transitions can reference these ports.
    pub promoted_ports: Vec<(SmId, PortId)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuspendPolicyRt {
    /// Preserve sub-SM state exactly. Resume where left off on re-entry.
    Freeze,
    /// Apply Elapse function with accumulated elapsed ticks on re-entry.
    Elapse,
    /// Destroy sub-SM on exit. Re-initialize from initial state on re-entry.
    Discard,
}

/// Snapshot of a frozen sub-SM (SuspendPolicy::Freeze or Elapse fallback).
#[derive(Debug, Clone)]
pub struct FrozenSmSnapshot {
    pub sm_id:        SmId,
    pub active_state: StateId,
    pub context:      Context,
    /// The tick number when the SM was frozen (used by Elapse to compute elapsed ticks).
    pub frozen_at_tick: u64,
}



/// A pending signal produced by an Interaction Rule match,
/// to be delivered in Phase 3.
#[derive(Debug)]
pub struct IrSignal {
    /// Source SM for spatial condition evaluation (§7.1).
    /// When `Some(id)`, `spatial_condition(spatial, source_sm, target_sm)` is
    /// evaluated as a post-filter in Phase 2. When `None`, spatial filtering
    /// is skipped for this signal (backward-compatible default).
    pub source_sm:   Option<SmId>,
    pub target_sm:   SmId,
    pub target_port: PortId,
    pub signal:      Signal,
}

/// Dirty-flag watch specification for an Interaction Rule (§11.2 optimization).
///
/// Controls whether `match_fn` is invoked during Phase 2:
/// - `All`:      always evaluate — backward-compatible default.
/// - `AnySm(s)`: skip evaluation unless at least one SM in `s` transitioned
///               during the previous tick (i.e., appears in `World::dirty_sms`).
///
/// When to use `AnySm`: declare the set of SMs whose state changes can affect
/// the rule's match condition. If none of those SMs changed, the rule's
/// output is guaranteed identical to the previous tick, so the call is skipped.
#[derive(Debug, Clone)]
pub enum IrWatch {
    /// Always evaluate — identical to pre-optimization behaviour.
    All,
    /// Evaluate only when at least one SM in the set was in `dirty_sms`.
    AnySm(std::collections::BTreeSet<SmId>),
}

/// Runtime definition of an Interaction Rule.
///
/// The match function receives a read-only snapshot of all SM instances
/// (active states + contexts, as they are in Phase 2 — before transitions fire).
/// It returns the signals to enqueue if the rule matches, empty vec otherwise.
///
/// Key constraints from §2.7 and §3:
///   - Evaluated only in Phase 2 (pre-transition states).
///   - No side effects beyond signal emission.
///   - Always evaluated under Server authority in networked contexts.
///   - NOT re-evaluated during Phase 4 cascade.
pub struct InteractionRuleDef {
    pub id:    u32,
    pub group: &'static str,
    /// Dirty-flag optimization: when to evaluate `match_fn`. Default: `All`.
    pub watch: IrWatch,
    pub match_fn: Box<
        dyn Fn(&std::collections::BTreeMap<SmId, SmInstance>) -> Vec<IrSignal>
            + Send + Sync,
    >,
    /// Optional spatial condition (§7.1). When set, the rule is only evaluated
    /// for SM pairs that satisfy the spatial predicate.
    /// Receives the SpatialIndex and two SM IDs; returns true if they are
    /// close enough for this rule to apply.
    pub spatial_condition: Option<crate::spatial::SpatialConditionFn>,
}

impl std::fmt::Debug for InteractionRuleDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InteractionRuleDef")
            .field("id", &self.id)
            .field("group", &self.group)
            .field("watch", &self.watch)
            .field("has_spatial_condition", &self.spatial_condition.is_some())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Continuous Port binding (§2.4.3, §2.4.4, Phase 1/6)
// ---------------------------------------------------------------------------

/// Binds an external continuous value source to an SM's context field.
/// Executed every tick in Phase 1 (§2.4.3).
pub struct ContinuousInputBinding {
    pub sm_id:        SmId,
    /// The context field this value writes into.
    pub target_field: String,
    /// The external value source (e.g. physics velocity, game clock).
    pub source:       Box<dyn Fn() -> f64 + Send + Sync>,
}

impl std::fmt::Debug for ContinuousInputBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContinuousInputBinding")
            .field("sm_id", &self.sm_id)
            .field("target_field", &self.target_field)
            .finish()
    }
}

/// Declares which context fields are exposed via a Continuous Output Port (§2.4.4).
/// Phase 6 publishes these for read-only consumption by rendering/audio/UI.
#[derive(Debug, Clone)]
pub struct ContinuousOutputDecl {
    pub sm_id:          SmId,
    /// Whitelist of context field names to expose. Internal fields do not leak.
    pub exposed_fields: Vec<String>,
}



/// Issued by a Transition Effect to spawn a new entity next tick.
#[derive(Debug)]
pub struct SpawnRequest {
    /// SM IDs to instantiate (must already be registered via World::register_sm).
    pub sm_ids: Vec<SmId>,
    /// Connection Templates to establish at spawn time (§4.5).
    pub connections: Vec<Connection>,
}

/// Issued by a Transition Effect or external directive to despawn an entity.
#[derive(Debug)]
pub struct DespawnRequest {
    /// All SM IDs belonging to the entity to despawn.
    pub sm_ids: Vec<SmId>,
}

#[derive(Debug)]
pub struct Connection {
    pub id: ConnectionId,
    pub source_sm: SmId,
    pub source_port: PortId,
    pub target_sm: SmId,
    pub target_port: PortId,
    /// Ticks before delivery. 0 = same tick (Phase 4 cascade).
    pub delay_ticks: u32,
    /// Connection-side pipeline (world rules): Transform → Filter → Redirect steps.
    pub pipeline: Vec<PipelineStep>,
}

// ---------------------------------------------------------------------------
// Signal delivery queue entry
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct QueuedSignal {
    pub target_sm:   SmId,
    pub target_port: PortId,
    pub signal:      Signal,
    /// Remaining ticks before delivery. 0 = deliver this tick.
    pub delay:       u32,
    /// Source connection, if this signal came from a static Connection (for diagnostics).
    pub source_conn: Option<ConnectionId>,
}

// ---------------------------------------------------------------------------
// World state
// ---------------------------------------------------------------------------

/// The complete runtime state of the Weaven simulation.
#[derive(Debug, Default)]
pub struct World {
    /// SM definitions (immutable design-time data).
    pub defs: BTreeMap<SmId, SmDef>,
    /// SM instances (mutable runtime state).
    pub instances: BTreeMap<SmId, SmInstance>,
    /// Active Set: SMs that require evaluation this tick.
    pub active_set: BTreeSet<SmId>,
    /// Static connections between ports.
    pub connections: Vec<Connection>,
    /// Interaction Rules evaluated each tick in Phase 2.
    pub interaction_rules: Vec<InteractionRuleDef>,
    /// SMs that transitioned in the current tick's Phase 3/4 (§11.2 dirty-flag).
    /// Swapped into `prev_dirty_sms` at the start of each tick.
    pub dirty_sms: BTreeSet<SmId>,
    /// SMs that transitioned during the previous tick (§11.2 dirty-flag).
    /// Populated from `dirty_sms` at Phase 1. Read by `IrWatch::AnySm` in Phase 2
    /// to skip rules whose watched SMs had no state change since last tick.
    pub prev_dirty_sms: BTreeSet<SmId>,
    /// Continuous Input Port bindings — evaluated each tick in Phase 1 (§2.4.3).
    pub continuous_inputs: Vec<ContinuousInputBinding>,
    /// Continuous Output Port declarations — published each tick in Phase 6 (§2.4.4).
    pub continuous_outputs: Vec<ContinuousOutputDecl>,
    /// Spatial index (§7.1, Tier 2). None = Tier 1 (spatial queries injected externally).
    pub spatial_index: Option<crate::spatial::SpatialIndex>,
    /// Compound State definitions: StateId → sub-SMs + suspend policy.
    pub compound_defs: BTreeMap<StateId, CompoundStateDef>,
    /// Frozen sub-SM snapshots (SuspendPolicy::Freeze) keyed by sub-SM ID.
    pub frozen_snapshots: BTreeMap<SmId, FrozenSmSnapshot>,
    /// Pending spawn requests (processed in Phase 5).
    pub spawn_queue: Vec<SpawnRequest>,
    /// Pending despawn requests (processed in Phase 5).
    pub despawn_queue: Vec<DespawnRequest>,
    /// Global Named Tables (§2.8) — read-only at runtime.
    pub tables: crate::expr::TableRegistry,
    /// Signal delivery queue (Phase 4).
    pub signal_queue: VecDeque<QueuedSignal>,
    /// System Commands accumulated during Phase 3/4, applied in Phase 6.
    pub pending_system_commands: Vec<SystemCommand>,
    /// Per-SM network policies (§8.1-§8.3, §11.7). Keyed by SmId.
    pub network_policies: BTreeMap<SmId, crate::network::SmNetworkPolicy>,
    /// Current time scale (1.0 = normal, 0.5 = half speed). Applied to Continuous Input Ports.
    pub time_scale: f64,
    /// Remaining HitStop frames (0 = not in hit stop).
    pub hit_stop_frames: u32,
    /// Remaining SlowMotion ticks.
    pub slow_motion_remaining: u32,
    /// SlowMotion factor while active (1.0 when not in slow motion).
    pub slow_motion_factor: f64,
    /// Current tick number (for determinism tracking).
    pub tick: u64,
    /// Maximum cascade depth before halting Phase 4.
    pub max_cascade_depth: u32,
    /// What to do when cascade depth is exceeded (§11.5).
    pub cascade_overflow_policy: crate::error::CascadeOverflowPolicy,
}

impl World {
    pub fn new() -> Self {
        Self {
            max_cascade_depth: 64,
            time_scale: 1.0,
            slow_motion_factor: 1.0,
            ..Default::default()
        }
    }

    /// Register an SM definition and create its initial instance.
    pub fn register_sm(&mut self, def: SmDef) -> SmId {
        let id = def.id;
        let instance = SmInstance::new(&def);
        self.defs.insert(id, def);
        self.instances.insert(id, instance);
        id
    }

    /// Add a connection between two SM ports.
    pub fn connect(&mut self, conn: Connection) {
        self.connections.push(conn);
    }

    /// Register an Interaction Rule (evaluated in Phase 2 every tick).
    pub fn register_rule(&mut self, rule: InteractionRuleDef) {
        self.interaction_rules.push(rule);
    }

    /// Register a per-SM network policy (§8, §11.7).
    pub fn register_network_policy(&mut self, policy: crate::network::SmNetworkPolicy) {
        self.network_policies.insert(policy.sm_id, policy);
    }

    /// Register a Compound State definition.
    pub fn register_compound(&mut self, def: CompoundStateDef) {
        self.compound_defs.insert(def.parent_state, def);
    }

    /// Enable Tier 2 spatial routing with the given cell size.
    pub fn enable_spatial(&mut self, cell_size: f64) {
        self.spatial_index = Some(crate::spatial::SpatialIndex::new(cell_size));
    }

    /// Update an SM's spatial position (§7.1). Automatically wakes the SM.
    pub fn set_position(&mut self, sm_id: SmId, x: f64, y: f64) {
        if let Some(ref mut spatial) = self.spatial_index {
            spatial.update(sm_id, x, y);
        }
        self.active_set.insert(sm_id);
    }

    /// Remove an SM from the spatial index (called at despawn time).
    pub fn remove_from_spatial(&mut self, sm_id: SmId) {
        if let Some(ref mut spatial) = self.spatial_index {
            spatial.remove(sm_id);
        }
    }

    /// Query SMs within radius of a given position.
    pub fn query_radius(&self, x: f64, y: f64, radius: f64) -> Vec<SmId> {
        self.spatial_index.as_ref()
            .map(|s| s.query_radius(x, y, radius))
            .unwrap_or_default()
    }
    pub fn register_table(&mut self, name: impl Into<String>, data: crate::expr::NamedTableData) {
        self.tables.register(name, data);
    }

    /// Bind an external continuous value source to an SM's context field (§2.4.3).
    /// The source function is called each tick in Phase 1.
    pub fn bind_continuous_input(
        &mut self,
        sm_id: SmId,
        target_field: impl Into<String>,
        source: impl Fn() -> f64 + Send + Sync + 'static,
    ) {
        self.continuous_inputs.push(ContinuousInputBinding {
            sm_id,
            target_field: target_field.into(),
            source: Box::new(source),
        });
        // Ensure the SM is active so Phase 2 sees the updated context.
        self.active_set.insert(sm_id);
    }

    /// Declare which context fields a SM exposes via Continuous Output Port (§2.4.4).
    pub fn declare_continuous_output(&mut self, sm_id: SmId, fields: Vec<String>) {
        self.continuous_outputs.push(ContinuousOutputDecl {
            sm_id,
            exposed_fields: fields,
        });
    }
    /// Effects from on_despawn transitions are batch-delivered after all
    /// despawning entities have been processed (§3 Phase 5).
    pub fn request_despawn(&mut self, sm_ids: Vec<SmId>) {
        self.despawn_queue.push(DespawnRequest { sm_ids });
    }

    /// Queue a spawn request (processed in Phase 5).
    /// The spawned SMs will enter the Active Set on the *next* tick (§4.5).
    pub fn request_spawn(&mut self, sm_ids: Vec<SmId>, connections: Vec<Connection>) {
        self.spawn_queue.push(SpawnRequest { sm_ids, connections });
    }

    /// Wake up an SM (add to Active Set).
    pub fn activate(&mut self, sm_id: SmId) {
        self.active_set.insert(sm_id);
    }

    /// Inject a signal directly into an SM's input port (e.g. from Phase 1 or tests).
    pub fn inject_signal(&mut self, target_sm: SmId, target_port: PortId, signal: Signal) {
        self.signal_queue.push_back(QueuedSignal {
            target_sm,
            target_port,
            signal,
            delay: 0,
            source_conn: None,
        });
        self.active_set.insert(target_sm);
    }
}
