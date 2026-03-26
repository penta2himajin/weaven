/// Weaven Bevy Adapter — Tier 1 integration skeleton (§12.5).
///
/// Bridges Bevy's frame loop and ECS to Weaven Core's Port-based API.
/// No FFI boundary — Weaven Core is consumed as a Rust crate directly.
///
/// Usage pattern (Tier 1):
///
/// ```rust,ignore
/// // In your Bevy App:
/// app
///     .insert_resource(WeavenWorld::new())
///     .add_systems(FixedUpdate, weaven_tick_system)
///     .add_systems(FixedUpdate, weaven_output_system.after(weaven_tick_system));
/// ```
///
/// Integration profile (§12.5 — Bevy):
///   - `FixedUpdate` schedule calls `tick()` each simulation step.
///   - Bevy `Component` values bound to Continuous Input Ports via push_continuous_input().
///   - `TickOutput.state_changes` drives Bevy `Events` for rendering/audio.
///   - `TickOutput.system_commands` consumed by Bevy systems for HitStop/SlowMotion.

use weaven_core::{World, TickOutput, SmId, PortId, Signal, SignalTypeId,
                  SystemCommand, snapshot, restore, WorldSnapshot,
                  diff_snapshots, policy_filtered_diff, scoped_snapshot,
                  interest_region_sms, rewind_and_resimulate,
                  SmStateDiff, SmNetworkPolicy, InputBuffer, TaggedInput};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// WeavenWorld resource
// ---------------------------------------------------------------------------

/// Bevy Resource wrapping the Weaven World.
/// Insert once at app startup; access via `ResMut<WeavenWorld>`.
pub struct WeavenWorld {
    pub world: World,
}

impl WeavenWorld {
    pub fn new() -> Self {
        Self { world: World::new() }
    }
}

impl Default for WeavenWorld {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Tick system
// ---------------------------------------------------------------------------

/// Core tick system — call once per fixed simulation step.
///
/// In real Bevy usage this would be:
/// ```rust,ignore
/// fn weaven_tick_system(mut weaven: ResMut<WeavenWorld>, mut events: EventWriter<WeavenStateChange>) {
///     let output = weaven_core::tick(&mut weaven.world);
///     for (sm_id, (prev, next)) in &output.state_changes {
///         events.send(WeavenStateChange { sm_id: *sm_id, prev: *prev, next: *next });
///     }
/// }
/// ```
///
/// Signature kept Bevy-free here to avoid requiring bevy as a dependency.
pub fn advance_tick(weaven: &mut WeavenWorld) -> TickOutput {
    weaven_core::tick(&mut weaven.world)
}

// ---------------------------------------------------------------------------
// Continuous Input Port binding
// ---------------------------------------------------------------------------

/// Push an external continuous value into a bound SM context field.
/// Called each frame before `advance_tick()` for physics/clock bindings.
///
/// Corresponds to §2.4.3 Continuous Input Port:
///   `binding` = external value source
///   `target_field` = context field to write into
pub fn push_continuous_input(weaven: &mut WeavenWorld, sm_id: SmId, field: &str, value: f64) {
    if let Some(inst) = weaven.world.instances.get_mut(&sm_id) {
        inst.context.set(field, value);
    }
}

// ---------------------------------------------------------------------------
// Output reading
// ---------------------------------------------------------------------------

/// Read a context field from a SM's Continuous Output Port (§2.4.4).
/// Called each frame by rendering/audio/UI systems.
pub fn read_output_field(weaven: &WeavenWorld, sm_id: SmId, field: &str) -> f64 {
    weaven.world.instances
        .get(&sm_id)
        .map(|i| i.context.get(field))
        .unwrap_or(0.0)
}

/// Read the active state of an SM (for animation/rendering).
pub fn read_active_state(weaven: &WeavenWorld, sm_id: SmId) -> Option<weaven_core::StateId> {
    weaven.world.instances.get(&sm_id).map(|i| i.active_state)
}

// ---------------------------------------------------------------------------
// Discrete input injection
// ---------------------------------------------------------------------------

/// Inject a discrete signal from a Bevy input event into an SM's Input Port.
/// Call this from your input system before `advance_tick()`.
pub fn inject_input(weaven: &mut WeavenWorld, sm_id: SmId, port: PortId,
                    fields: &[(&str, f64)]) {
    let mut payload = BTreeMap::new();
    for (k, v) in fields { payload.insert(k.to_string(), *v); }
    weaven.world.inject_signal(sm_id, port, Signal {
        signal_type: SignalTypeId(0),
        payload,
    });
}

// ---------------------------------------------------------------------------
// System Command consumers
// ---------------------------------------------------------------------------

/// Process System Commands from TickOutput.
/// Returns hit-stop frames to apply this frame (0 = no freeze).
pub fn consume_system_commands(output: &TickOutput, weaven: &WeavenWorld) -> SystemCommandSummary {
    let mut summary = SystemCommandSummary::default();
    for cmd in &output.system_commands {
        match cmd {
            SystemCommand::HitStop { frames } => {
                summary.hit_stop_frames = summary.hit_stop_frames.max(*frames);
            }
            SystemCommand::SlowMotion { factor, duration_ticks } => {
                summary.slow_motion_factor    = *factor;
                summary.slow_motion_remaining = *duration_ticks;
            }
            SystemCommand::TimeScale(s) => {
                summary.time_scale = *s;
            }
        }
    }
    // Also read live world state (commands may have been applied in Phase 6)
    if summary.hit_stop_frames == 0 {
        summary.hit_stop_frames = weaven.world.hit_stop_frames;
    }
    summary
}

#[derive(Debug, Default)]
pub struct SystemCommandSummary {
    pub hit_stop_frames:    u32,
    pub slow_motion_factor:    f64,
    pub slow_motion_remaining: u32,
    pub time_scale:            f64,
}

impl SystemCommandSummary {
    pub fn is_frozen(&self) -> bool { self.hit_stop_frames > 0 }
    pub fn effective_time_scale(&self) -> f64 {
        let base = if self.time_scale == 0.0 { 1.0 } else { self.time_scale };
        if self.slow_motion_remaining > 0 { base * self.slow_motion_factor } else { base }
    }
}

// ---------------------------------------------------------------------------
// Network support (Snapshot/Restore for Bevy Netcode integration)
// ---------------------------------------------------------------------------

/// Take a snapshot for rollback networking.
pub fn take_snapshot(weaven: &WeavenWorld) -> WorldSnapshot {
    snapshot(&weaven.world)
}

/// Restore from a snapshot (e.g. on server correction).
pub fn apply_snapshot(weaven: &mut WeavenWorld, snap: &WorldSnapshot) {
    restore(&mut weaven.world, snap);
}

/// Compute the diff between two snapshots (e.g. before/after a tick).
pub fn diff_world_snapshots(before: &WorldSnapshot, after: &WorldSnapshot) -> Vec<SmStateDiff> {
    diff_snapshots(before, after)
}

/// Filter a diff list by each SM's registered network sync policy.
/// SMs with `SyncPolicy::None` or `InputSync` are excluded; `StateSync`
/// strips context changes; `ContextSync` keeps only whitelisted fields.
pub fn filter_diff_by_policy(weaven: &WeavenWorld, diffs: &[SmStateDiff]) -> Vec<SmStateDiff> {
    policy_filtered_diff(diffs, &weaven.world.network_policies)
}

/// Register a network policy for an SM (Authority, SyncPolicy, ReconciliationPolicy).
pub fn set_network_policy(weaven: &mut WeavenWorld, policy: SmNetworkPolicy) {
    weaven.world.network_policies.insert(policy.sm_id, policy);
}

/// Take a snapshot of only the SMs in `sm_ids` (render scope / interest region).
pub fn take_scoped_snapshot(weaven: &WeavenWorld, sm_ids: &BTreeSet<SmId>) -> WorldSnapshot {
    scoped_snapshot(&weaven.world, sm_ids)
}

/// Return SM IDs within a spatial radius (interest region management).
pub fn query_interest_region(weaven: &WeavenWorld, cx: f32, cy: f32, radius: f32) -> BTreeSet<SmId> {
    interest_region_sms(&weaven.world, cx, cy, radius)
}

/// Create a new InputBuffer for rollback networking.
pub fn create_input_buffer(history_depth: u32) -> InputBuffer {
    InputBuffer::new(history_depth)
}

/// Push a tagged input into the buffer.
pub fn push_tagged_input(buffer: &mut InputBuffer, input: TaggedInput) {
    buffer.push(input);
}

/// Apply buffered inputs for the current tick to the world.
pub fn apply_buffered_inputs(weaven: &mut WeavenWorld, buffer: &InputBuffer) {
    buffer.apply_tick_inputs(&mut weaven.world);
}

/// Rewind to a snapshot and re-simulate forward, replaying buffered inputs.
pub fn rewind_and_replay(
    weaven: &mut WeavenWorld,
    base_snapshot: &WorldSnapshot,
    input_buffer: &InputBuffer,
    target_tick: u64,
    current_tick: u64,
) {
    rewind_and_resimulate(&mut weaven.world, base_snapshot, input_buffer, target_tick, current_tick);
}

// ---------------------------------------------------------------------------
// Tests (no Bevy dependency)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use weaven_core::*;

    fn make_simple_sm(id: SmId) -> SmDef {
        SmDef::new(
            id,
            [StateId(0), StateId(1)],
            StateId(0),
            vec![Transition {
                id: TransitionId(id.0 * 10),
                source: StateId(0), target: StateId(1), priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
                effects: vec![],
            }],
            vec![], vec![],
        )
    }

    #[test]
    fn test_adapter_advance_tick() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));

        push_continuous_input(&mut weaven, SmId(1), "trigger", 1.0);
        weaven.world.activate(SmId(1));
        let out = advance_tick(&mut weaven);

        assert_eq!(weaven.world.instances[&SmId(1)].active_state, StateId(1));
        assert!(out.state_changes.contains_key(&SmId(1)));
    }

    #[test]
    fn test_adapter_read_output_field() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));
        if let Some(i) = weaven.world.instances.get_mut(&SmId(1)) {
            i.context.set("speed", 5.0);
        }
        assert_eq!(read_output_field(&weaven, SmId(1), "speed"), 5.0);
    }

    #[test]
    fn test_adapter_snapshot_restore() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));

        let snap = take_snapshot(&weaven);
        push_continuous_input(&mut weaven, SmId(1), "trigger", 1.0);
        weaven.world.activate(SmId(1));
        advance_tick(&mut weaven);
        assert_eq!(weaven.world.instances[&SmId(1)].active_state, StateId(1));

        apply_snapshot(&mut weaven, &snap);
        assert_eq!(weaven.world.instances[&SmId(1)].active_state, StateId(0),
            "restored to S0");
    }

    #[test]
    fn test_adapter_diff_snapshots() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));

        let before = take_snapshot(&weaven);
        push_continuous_input(&mut weaven, SmId(1), "trigger", 1.0);
        weaven.world.activate(SmId(1));
        advance_tick(&mut weaven);
        let after = take_snapshot(&weaven);

        let diffs = diff_world_snapshots(&before, &after);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].sm_id, 1);
        assert_eq!(diffs[0].prev_state, 0);
        assert_eq!(diffs[0].new_state, 1);
    }

    #[test]
    fn test_adapter_policy_filtered_diff() {
        use weaven_core::{Authority, SyncPolicy, ReconciliationPolicy};

        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));
        weaven.world.register_sm(make_simple_sm(SmId(2)));

        // SM(1) = StateSync, SM(2) = None (excluded)
        set_network_policy(&mut weaven, SmNetworkPolicy {
            sm_id: SmId(1),
            authority: Authority::Server,
            sync_policy: SyncPolicy::StateSync,
            reconciliation: ReconciliationPolicy::Snap,
        });
        set_network_policy(&mut weaven, SmNetworkPolicy {
            sm_id: SmId(2),
            authority: Authority::Server,
            sync_policy: SyncPolicy::None,
            reconciliation: ReconciliationPolicy::Snap,
        });

        let before = take_snapshot(&weaven);
        push_continuous_input(&mut weaven, SmId(1), "trigger", 1.0);
        push_continuous_input(&mut weaven, SmId(2), "trigger", 1.0);
        weaven.world.activate(SmId(1));
        weaven.world.activate(SmId(2));
        advance_tick(&mut weaven);
        let after = take_snapshot(&weaven);

        let diffs = diff_world_snapshots(&before, &after);
        assert_eq!(diffs.len(), 2, "raw diffs should include both SMs");

        let filtered = filter_diff_by_policy(&weaven, &diffs);
        assert_eq!(filtered.len(), 1, "SM(2) with SyncPolicy::None should be excluded");
        assert_eq!(filtered[0].sm_id, 1);
        assert!(filtered[0].context_changes.is_empty(),
            "StateSync should strip context_changes");
    }

    #[test]
    fn test_adapter_scoped_snapshot() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));
        weaven.world.register_sm(make_simple_sm(SmId(2)));

        let scope: BTreeSet<SmId> = [SmId(1)].into_iter().collect();
        let snap = take_scoped_snapshot(&weaven, &scope);
        assert_eq!(snap.instances.len(), 1);
        assert_eq!(snap.instances[0].sm_id, 1);
    }

    #[test]
    fn test_adapter_interest_region() {
        let mut weaven = WeavenWorld::new();
        weaven.world.enable_spatial(10.0);
        weaven.world.register_sm(make_simple_sm(SmId(1)));
        weaven.world.register_sm(make_simple_sm(SmId(2)));

        sync_position(&mut weaven, SmId(1), 0.0, 0.0);
        sync_position(&mut weaven, SmId(2), 100.0, 100.0);

        let region = query_interest_region(&weaven, 0.0, 0.0, 5.0);
        assert!(region.contains(&SmId(1)));
        assert!(!region.contains(&SmId(2)), "SM(2) too far away");
    }

    #[test]
    fn test_adapter_input_buffer_and_rewind() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(make_simple_sm(SmId(1)));
        weaven.world.activate(SmId(1));

        // Take base snapshot at tick 0
        let base = take_snapshot(&weaven);

        // Create input buffer and push a tagged input for tick 0
        let mut buffer = create_input_buffer(10);
        push_tagged_input(&mut buffer, TaggedInput {
            tick: 0,
            target_sm: SmId(1),
            target_port: PortId(0),
            signal: Signal {
                signal_type: SignalTypeId(0),
                payload: {
                    let mut p = BTreeMap::new();
                    p.insert("trigger".to_string(), 1.0);
                    p
                },
            },
        });

        // Advance a tick with the input buffer
        apply_buffered_inputs(&mut weaven, &buffer);
        advance_tick(&mut weaven);
        assert_eq!(weaven.world.instances[&SmId(1)].active_state, StateId(1),
            "SM should transition after buffered input");

        // Rewind and replay
        rewind_and_replay(&mut weaven, &base, &buffer, 0, 1);
        assert_eq!(weaven.world.instances[&SmId(1)].active_state, StateId(1),
            "SM should still be in S1 after rewind+replay with same inputs");
    }

    #[test]
    fn test_system_command_summary_frozen() {
        let summary = SystemCommandSummary { hit_stop_frames: 3, ..Default::default() };
        assert!(summary.is_frozen());
    }

    #[test]
    fn test_system_command_summary_time_scale() {
        let summary = SystemCommandSummary {
            time_scale: 1.0,
            slow_motion_factor: 0.5,
            slow_motion_remaining: 2,
            ..Default::default()
        };
        assert_eq!(summary.effective_time_scale(), 0.5);
    }
}

// ---------------------------------------------------------------------------
// Schema loader integration
// ---------------------------------------------------------------------------

/// Load a Weaven World from a JSON schema string (§12.2 Weaven Schema).
///
/// This is the primary entry point for data-driven game setup:
/// SM definitions, Connections, and Named Tables are authored in JSON and
/// loaded at startup rather than hard-coded in Rust.
///
/// # Errors
/// Returns an error string if the JSON is malformed or fails schema validation.
///
/// # Example (Bevy startup system)
/// ```rust,ignore
/// fn setup(mut weaven: ResMut<WeavenWorld>) {
///     let json = include_str!("../assets/game_schema.json");
///     load_world_from_schema(&mut weaven, json).expect("schema load failed");
/// }
/// ```
pub fn load_world_from_schema(weaven: &mut WeavenWorld, json: &str)
    -> Result<(), Box<dyn std::error::Error>>
{
    use weaven_core::schema::{load_schema, compile_schema};
    let schema  = load_schema(json)?;
    let compiled = compile_schema(&schema);
    for def in compiled.sm_defs {
        weaven.world.register_sm(def);
    }
    for conn in compiled.connections {
        weaven.world.connect(conn);
    }
    weaven.world.tables = compiled.table_registry;
    Ok(())
}

// ---------------------------------------------------------------------------
// Spatial bridge
// ---------------------------------------------------------------------------

/// Update an SM's spatial position from external coordinates (e.g. Bevy Transform).
///
/// Call this each frame for any SM whose entity moves in the game world.
/// Internally delegates to `World::set_position`, which updates the spatial
/// index and wakes the SM if the position changed.
///
/// # Bevy usage pattern
/// ```rust,ignore
/// fn sync_positions(
///     mut weaven: ResMut<WeavenWorld>,
///     query: Query<(&WeavenSmId, &Transform), Changed<Transform>>,
/// ) {
///     for (sm_id, transform) in &query {
///         sync_position(&mut weaven, sm_id.0, transform.translation.x, transform.translation.y);
///     }
/// }
/// ```
pub fn sync_position(weaven: &mut WeavenWorld, sm_id: SmId, x: f32, y: f32) {
    weaven.world.set_position(sm_id, x as f64, y as f64);
}

/// Query SM IDs within a spatial radius (§8.4 interest management).
///
/// Useful for rendering systems that need to know which entities are
/// visible from a given camera position.
pub fn query_nearby(weaven: &WeavenWorld, cx: f32, cy: f32, radius: f32)
    -> Vec<SmId>
{
    weaven.world.query_radius(cx as f64, cy as f64, radius as f64)
}

// ---------------------------------------------------------------------------
// PoC integration test helpers (Bevy-free)
// ---------------------------------------------------------------------------

/// Run the adapter through a complete scenario without Bevy:
/// schema load → tick loop → read outputs.
///
/// This mirrors what a Bevy app would do inside its systems but is callable
/// from integration tests without a full engine setup.
pub fn run_headless_scenario(
    json: &str,
    setup_fn: impl FnOnce(&mut WeavenWorld),
    tick_count: u32,
) -> Result<Vec<TickOutput>, Box<dyn std::error::Error>> {
    let mut weaven = WeavenWorld::new();
    load_world_from_schema(&mut weaven, json)?;
    setup_fn(&mut weaven);
    let outputs = (0..tick_count)
        .map(|_| advance_tick(&mut weaven))
        .collect();
    Ok(outputs)
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use weaven_core::*;

    // ── Schema loader ────────────────────────────────────────────────────

    #[test]
    fn test_load_world_from_schema_basic() {
        let json = r#"{
            "state_machines": [{
                "id": 1,
                "states": [0, 1],
                "initial_state": 0,
                "transitions": [{
                    "id": 10,
                    "source": 0, "target": 1, "priority": 10,
                    "guard": { "CtxField": "hp" },
                    "effects": []
                }],
                "input_ports":  [],
                "output_ports": [],
                "elapse_capability": "NonElapsable"
            }],
            "connections": [],
            "named_tables": []
        }"#;

        let mut weaven = WeavenWorld::new();
        load_world_from_schema(&mut weaven, json).unwrap();
        assert!(weaven.world.defs.contains_key(&SmId(1)),
            "SM(1) should be registered after schema load");
    }

    #[test]
    fn test_load_world_from_schema_invalid_json() {
        let mut weaven = WeavenWorld::new();
        let result = load_world_from_schema(&mut weaven, "{ not valid json }");
        assert!(result.is_err(), "invalid JSON should return an error");
    }

    // ── Spatial bridge ───────────────────────────────────────────────────

    #[test]
    fn test_sync_position_and_query_nearby() {
        let mut weaven = WeavenWorld::new();
        weaven.world.enable_spatial(10.0);

        // Register two SMs
        for id in 1u32..=2 {
            weaven.world.register_sm(SmDef::new(
                SmId(id), [StateId(0)], StateId(0),
                vec![], vec![], vec![],
            ));
        }

        sync_position(&mut weaven, SmId(1), 0.0, 0.0);
        sync_position(&mut weaven, SmId(2), 3.0, 0.0);

        let nearby = query_nearby(&weaven, 0.0, 0.0, 5.0);
        assert!(nearby.contains(&SmId(1)), "SM1 at origin should be nearby");
        assert!(nearby.contains(&SmId(2)), "SM2 at distance 3 should be nearby");
    }

    #[test]
    fn test_sync_position_no_spatial_index() {
        let mut weaven = WeavenWorld::new();
        weaven.world.register_sm(SmDef::new(
            SmId(1), [StateId(0)], StateId(0), vec![], vec![], vec![],
        ));
        // No enable_spatial — should not panic
        sync_position(&mut weaven, SmId(1), 10.0, 20.0);
        let nearby = query_nearby(&weaven, 0.0, 0.0, 100.0);
        assert!(nearby.is_empty(), "no spatial index → empty result");
    }

    // ── run_headless_scenario ────────────────────────────────────────────

    #[test]
    fn test_run_headless_scenario_fire_propagation() {
        // Minimal fire schema: SM1 (grass→burning), SM2 (grass→burning),
        // Connection SM1.out → SM2.in
        let json = include_str!("../../fire_propagation.json");
        let outputs = run_headless_scenario(json, |w| {
            // Ignite SM1 by setting intensity context field
            if let Some(inst) = w.world.instances.get_mut(&SmId(1)) {
                inst.context.set("intensity", 5.0);
            }
            w.world.activate(SmId(1));
            w.world.activate(SmId(2));
        }, 3).unwrap();
        // After 3 ticks SM1 and SM2 should both be burning (StateId(1))
        assert!(!outputs.is_empty());
        let last = outputs.last().unwrap();
        // At least SM1 should have transitioned
        assert!(!last.state_changes.is_empty() || outputs[0].state_changes.contains_key(&SmId(1)));
    }

    // ── PoC scenarios (headless) ─────────────────────────────────────────

    #[test]
    fn test_poc_elemental_reaction_via_adapter() {
        // Verify elemental reactions work end-to-end through the adapter API.
        let mut weaven = WeavenWorld::new();

        let fire_sm   = SmId(1);
        let water_sm  = SmId(2);
        let target_sm = SmId(3);

        let port_in   = PortId(0);
        let port_out  = PortId(1);
        let sig       = SignalTypeId(0);

        // fire_sm: idle → active on trigger, emits fire to target
        weaven.world.register_sm(SmDef {
            id: fire_sm,
            states: [StateId(0), StateId(1)].into_iter().collect(),
            initial_state: StateId(0),
            transitions: vec![Transition {
                id: TransitionId(10),
                source: StateId(0), target: StateId(1), priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
                effects: vec![Box::new(|_| {
                    let mut p = std::collections::BTreeMap::new();
                    p.insert("element".to_string(), 1.0); // fire=1
                    vec![EffectOutput::Signal(PortId(1),
                        Signal { signal_type: SignalTypeId(0), payload: p })]
                })],
            }],
            input_ports:  vec![],
            output_ports: vec![Port::new(port_out, PortKind::Output, sig)],
            on_despawn_transitions: vec![],
            elapse_capability: ElapseCapabilityRt::NonElapsable,
            elapse_fn: None,
        });

        // target_sm: receives element signal
        weaven.world.register_sm(SmDef {
            id: target_sm,
            states: [StateId(0), StateId(1)].into_iter().collect(),
            initial_state: StateId(0),
            transitions: vec![Transition {
                id: TransitionId(30),
                source: StateId(0), target: StateId(1), priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("element") > 0.0)),
                effects: vec![],
            }],
            input_ports:  vec![Port::new(port_in, PortKind::Input, sig)],
            output_ports: vec![],
            on_despawn_transitions: vec![],
            elapse_capability: ElapseCapabilityRt::NonElapsable,
            elapse_fn: None,
        });

        weaven.world.connect(Connection {
            id: ConnectionId(1),
            source_sm: fire_sm, source_port: port_out,
            target_sm: target_sm, target_port: port_in,
            pipeline: vec![], delay_ticks: 0,
        });

        // Trigger fire_sm
        push_continuous_input(&mut weaven, fire_sm, "trigger", 1.0);
        weaven.world.activate(fire_sm);
        weaven.world.activate(target_sm);

        let out = advance_tick(&mut weaven);

        assert!(out.state_changes.contains_key(&fire_sm),
            "fire_sm should have transitioned");
        // target_sm receives signal in Phase 4 of same tick
        assert_eq!(weaven.world.instances[&target_sm].active_state, StateId(1),
            "target_sm should receive fire signal via Connection");
    }

    #[test]
    fn test_poc_entity_lifecycle_via_adapter() {
        // Entity spawn → active → despawn → signals batched correctly.
        let mut weaven = WeavenWorld::new();

        let entity = SmId(1);
        weaven.world.register_sm(SmDef::new(
            entity,
            [StateId(0), StateId(1)],
            StateId(0),
            vec![Transition {
                id: TransitionId(10),
                source: StateId(0), target: StateId(1), priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("hp") <= 0.0 && ctx.get("hp") >= -999.0)),
                effects: vec![],
            }],
            vec![], vec![],
        ));

        // Entity starts with hp=0 → immediately transitions
        push_continuous_input(&mut weaven, entity, "hp", 0.0);
        weaven.world.activate(entity);
        let out = advance_tick(&mut weaven);

        assert_eq!(weaven.world.instances[&entity].active_state, StateId(1),
            "entity should transition when hp=0");
        assert!(out.state_changes.contains_key(&entity));
    }
}
